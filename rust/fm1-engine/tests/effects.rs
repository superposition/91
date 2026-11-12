// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{
    effects::{self, control, control_abi, echo, engage, master, reverb},
    sequencer::Rng,
};
fn master_state() -> master::State {
    master::State {
        cur: 1.0,
        sum: 64.0,
        release: 1.0,
        envelope: 0.0,
        gain: 1.0,
        sub_min: 1.0,
        check: 37,
        index: 0,
        count: 0,
        sub_index: 0,
        threshold: 0.0,
        slope: 0.0,
        makeup: 1.0,
        attack: 0.05,
        decay: 0.0005,
        ceiling: 1.0,
        reduction_hold: 0.0,
        peak_hold: 0.0,
    }
}
#[test]
fn stereo_impulse_has_the_frozen_64_slot_ring_latency_and_reset() {
    let mut s = master_state();
    let mut l = [0.0; 64];
    let mut r = [0.0; 64];
    let mut g = [0.0; 64];
    let mut sub = [0.0; 8];
    master::reset(
        &mut s,
        master::Buffers {
            left: &mut l,
            right: &mut r,
            gains: &mut g,
            sub: &mut sub,
        },
    );
    assert_eq!(s.check, 37);
    let mut left = [0.0; 128];
    let mut right = [0.0; 128];
    left[0] = 0.1;
    right[0] = -0.05;
    master::process(
        &mut s,
        master::Buffers {
            left: &mut l,
            right: &mut r,
            gains: &mut g,
            sub: &mut sub,
        },
        &mut left,
        &mut right,
    );
    assert_eq!(left.iter().position(|x| *x != 0.0), Some(63));
    assert_eq!(left[63], 0.1);
    assert_eq!(right[63], -0.05);
    assert_eq!(left.iter().filter(|x| **x != 0.0).count(), 1);
    master::reset(
        &mut s,
        master::Buffers {
            left: &mut l,
            right: &mut r,
            gains: &mut g,
            sub: &mut sub,
        },
    );
    assert!(l.iter().chain(r.iter()).all(|x| *x == 0.0));
    assert!(g.iter().chain(sub.iter()).all(|x| *x == 1.0));
}
#[test]
fn dynamics_rejects_nonfinite_and_unbounded_samples() {
    let mut s = master_state();
    let mut l = [0.0; 64];
    let mut r = [0.0; 64];
    let mut g = [1.0; 64];
    let mut sub = [1.0; 8];
    let mut left = [0.0; 256];
    let mut right = [0.0; 256];
    left[0] = f32::NAN;
    left[1] = f32::INFINITY;
    right[2] = -f32::INFINITY;
    right[3] = 64.0;
    master::process(
        &mut s,
        master::Buffers {
            left: &mut l,
            right: &mut r,
            gains: &mut g,
            sub: &mut sub,
        },
        &mut left,
        &mut right,
    );
    assert!(left
        .iter()
        .chain(right.iter())
        .all(|x| x.is_finite() && *x == 0.0));
}
#[test]
fn usb_recording_is_fixed_while_master_gain_changes() {
    let l = [0.0, 0.2, -0.8, 100.0];
    let r = [0.0, -0.3, 0.6, -100.0];
    let mut quiet = [0; 8];
    let mut loud = [0; 8];
    let mut usb0 = [0; 8];
    let mut usb1 = [0; 8];
    effects::output(&l, &r, &mut quiet, Some(&mut usb0), 0.0, 1.0, false);
    effects::output(&l, &r, &mut loud, Some(&mut usb1), 3.0, 1.0, true);
    assert_eq!(usb0, usb1);
    assert!(quiet.iter().all(|x| *x == 0));
    assert_ne!(quiet, loud);
    assert!(loud.iter().all(|x| x.abs() <= 8_300_000));
    assert!(usb1.iter().all(|x| x.abs() <= 32_421));
    assert_ne!(usb1[2], 0);
}
#[test]
fn tape_null_impulse_max_feedback_and_shed_are_bounded() {
    let mut s = echo::State {
        tape_write: 0,
        phase: 0,
        accumulator: 0.0,
        low: 0.0,
        high: 0.0,
        previous1: 0.0,
        previous2: 0.0,
        current1: 0.0,
        current2: 0.0,
        plex_write: 0,
        plex_low: 0.0,
        plex_low2: 0.0,
        length: 1000.0,
        head2: 0.75,
    };
    let mut c = echo::Config {
        target_length: 1000.0,
        head_speed: 0.0,
        motion: 0.0,
        modulation: 0.0,
        lfo: 0.0,
        lean: 0.0,
        feedback: 0.86,
        pan: [1.0, 0.0, 0.0, 1.0],
        shed: 0,
    };
    let mut tape: Box<[i16; echo::TAPE]> =
        vec![0; echo::TAPE].into_boxed_slice().try_into().unwrap();
    let mut plex: Box<[i16; echo::PLEX]> =
        vec![0; echo::PLEX].into_boxed_slice().try_into().unwrap();
    for _ in 0..9000 {
        assert_eq!(
            echo::sample(&mut s, &c, &mut tape, &mut plex, 0.0),
            [0.0; 4]
        );
    }
    let mut tail = 0;
    for i in 0..90000 {
        if i == 45000 {
            c.shed = 1;
        }
        let out = echo::sample(
            &mut s,
            &c,
            &mut tape,
            &mut plex,
            if i == 0 { 1.0 } else { 0.0 },
        );
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 16.0));
        if i > 1000 && out.iter().any(|x| *x != 0.0) {
            tail += 1;
        }
        if c.shed != 0 {
            assert_eq!(&out[2..], &[0.0; 2]);
        }
    }
    assert!(tail > 1000);
    assert!((0..echo::TAPE as i32).contains(&s.tape_write));
    assert!((0..echo::PLEX as i32).contains(&s.plex_write));
}
#[test]
fn plate_shimmer_gate_impulses_have_tails_without_nonfinite_growth() {
    for kind in 0..3 {
        let mut s = reverb::State {
            positions: [0; 12],
            gate_envelope: 0.0,
            gate_gain: 1.0,
            gate_hold: 0.0,
            shimmer_feedback: 0.0,
            shimmer_dc: 0.0,
            shimmer_phase: 0.0,
            bandwidth: 0.0,
            damp_left: 0.0,
            damp_right: 0.0,
            sine: 0.0,
            cosine: 1.0,
            shimmer_write: 0,
        };
        let c = reverb::Config {
            kind,
            damping: 0.45,
            hold: 17640.0,
            shimmer: 0.35,
            decay: 0.9995,
            diffusion: 0.5,
        };
        let mut data: Box<[i16; reverb::WORDS]> = vec![0; reverb::WORDS]
            .into_boxed_slice()
            .try_into()
            .unwrap();
        let mut shimmer: Box<[f32; 2048]> = vec![0.0; 2048].into_boxed_slice().try_into().unwrap();
        for _ in 0..1000 {
            assert_eq!(
                reverb::sample(0.0, &mut s, &c, &mut data, &mut shimmer),
                [0.0; 2]
            );
        }
        let mut late = 0;
        for i in 0..132300 {
            let out = reverb::sample(
                if i == 0 { 0.8 } else { 0.0 },
                &mut s,
                &c,
                &mut data,
                &mut shimmer,
            );
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 64.0));
            if i > 44100 && out.iter().any(|x| *x != 0.0) {
                late += 1;
            }
        }
        assert!(late > 100);
        for (index, len) in s.positions.iter().zip(reverb::LENGTHS) {
            assert!((0..len as i32).contains(index));
        }
    }
}
fn config() -> control_abi::Config {
    let mut fxs = [0; 96];
    fxs[0] = 16; // Gate loop never divides by zero, even off.
    control_abi::Config {
        mask_lo: 0,
        mask_hi: 0,
        trk_solo: 0,
        trk_mute: 0,
        gx_pat: 0xffff,
        sixteenth: 1,
        en_jam_on: 0,
        en_playing: 1,
        en_ext: 0,
        flx_active: 0,
        rb_w: 0,
        step_t: 0,
        intro_len: 0,
        brate: -1,
        flx_sel: 0,
        flx_pat: 0,
        vr_which: 0,
        step_len: 5512.5,
        bm_len: 0.0,
        echo_fb: 0.5,
        lfo_tcut: 0.0,
        brk: 0.0,
        fxs_v: fxs,
        fxk_v: [0; 4],
    }
}
#[test]
fn masks_obey_stop_reverse_priority_and_bounded_gain_ramps() {
    let mut c = config();
    let mut s = control_abi::State {
        trk_g1: [1.0; 8],
        ..Default::default()
    };
    c.mask_lo = (1 << 6) | (1 << 7);
    c.trk_mute = 0xff;
    assert_eq!(control::chunk(&mut s, &c, 128) & 255, 0);
    assert_eq!(s.bmode, 3);
    assert_eq!(s.bm_start, 0u32.wrapping_sub(2));
    assert!(s.trk_g1.iter().all(|x| *x > 0.0 && *x < 1.0));
    c.mask_lo = 1 << 6;
    control::chunk(&mut s, &c, 383);
    assert_eq!(s.bmode, 2);
    assert_eq!(s.bm_len, 41984.0);
    assert!(s.trk_g1.iter().all(|x| *x == 0.0));
}
#[test]
fn grain_engagement_uses_exactly_one_draw_per_grain() {
    let mut c = config();
    c.fxs_v[50] = 3; // Verified EN_FXS_FZ_GRAINS ABI constant.
    let mut s = control_abi::Engage::default();
    let mut rng = Rng(91);
    engage::apply(&mut s, &c, &mut rng, 5, 0);
    let mut expected = Rng(91);
    for _ in 0..s.gn_n {
        expected.next_u32();
    }
    assert_eq!(rng.0, expected.0);
    assert!(s.gn_n >= 1 && s.gn_n <= 4);
    assert!(s.gn_pos[..s.gn_n as usize].windows(2).all(|x| x[0] < x[1]));
    assert!(s.gn_st[..s.gn_n as usize]
        .iter()
        .all(|x| *x >= 0.0 && *x + s.gn_len <= s.gn_region));
}
