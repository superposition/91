// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::voices::Drum;
fn drum_words(v: &Drum) -> [u32; 26] {
    [
        v.ph.to_bits(),
        v.inc.to_bits(),
        v.inc_end.to_bits(),
        v.amp.to_bits(),
        v.adec.to_bits(),
        v.idx.to_bits(),
        v.x1.to_bits(),
        v.x2.to_bits(),
        v.att.to_bits(),
        v.mph[0].to_bits(),
        v.mph[1].to_bits(),
        v.mph[2].to_bits(),
        v.mph[3].to_bits(),
        v.pend.to_bits(),
        v.t.to_bits(),
        v.hp.to_bits(),
        v.ph2.to_bits(),
        v.inc2.to_bits(),
        v.wait as u32,
        v.wait2 as u32,
        v.pend2.to_bits(),
        v.rgap as u32,
        v.rmul.to_bits(),
        v.xl.to_bits(),
        v.fo.to_bits(),
        v.rn as u32 | ((v.on as u32) << 8) | ((v.note as u32) << 16),
    ]
}
use fm1_engine::{
    sequencer::Rng,
    voices::{acid, data, drone, pad, setup},
};
fn pad_words(s: &pad::State) -> Vec<u32> {
    let mut w = Vec::new();
    for a in [
        &s.ph[..],
        &s.inc,
        &[
            s.sub, s.note, s.target, s.t, s.rate, s.peak, s.amp, s.fa, s.fh, s.fr,
        ],
        &s.ic,
        &s.c1,
        &s.c2,
        &[s.vib, s.pan, s.hf],
    ] {
        w.extend(a.iter().map(|v| v.to_bits()));
    }
    w.push(
        s.midi_note as u32
            | ((s.hand as u32) << 8)
            | ((s.voices as u32) << 16)
            | ((s.play as u32) << 24),
    );
    w.push(s.pnote as u32);
    w.push(s.pord);
    w.push(s.pamp.to_bits());
    w
}
fn drone_words(s: &drone::State) -> Vec<u32> {
    let mut w = Vec::new();
    for f in [
        s.ph.as_slice(),
        &[s.sph, s.mph, s.note, s.target, s.amp],
        &s.s,
        &s.c1,
        &s.c2,
        &[s.env, s.gtarget],
    ] {
        w.extend(f.iter().map(|v| v.to_bits()));
    }
    w.push(
        s.midi_note as u32
            | ((s.rest as u32) << 8)
            | ((s.env_up as u32) << 16)
            | ((s.play as u32) << 24),
    );
    w.push(s.velocity.to_bits());
    w
}
fn acid_words(s: &acid::State) -> Vec<u32> {
    let mut w = Vec::new();
    for f in [
        &[s.ph, s.inc, s.inc_t, s.amp, s.amp_t, s.env, s.edec, s.acc][..],
        &s.s,
        &s.c1,
        &s.c2,
    ] {
        w.extend(f.iter().map(|v| v.to_bits()));
    }
    w.push(s.gate_left as u32);
    w.push(s.note as u32 | ((s.sliding as u32) << 8) | ((s.play as u32) << 16));
    w.extend([s.ps, s.p3, s.p4, s.p5, s.lph].map(f32::to_bits));
    w
}
fn hash_word(mut h: u32, w: u32) -> u32 {
    for b in w.to_le_bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}
fn byte(h: u32, b: u8) -> u32 {
    (h ^ b as u32).wrapping_mul(16777619)
}
fn events(mut h: u32, e: &setup::Events) -> u32 {
    assert!(e.len <= 8);
    for m in &e.events[..e.len as usize] {
        for w in [m.status, m.note, m.velocity] {
            h = hash_word(h, w);
        }
    }
    h
}
#[test]
fn slow_motion_lfo_envelopes_keys_steals_and_releases_match_frozen_c() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("voice-setup-native.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let test = case["test"].as_u64().unwrap() as usize;
        let mut hash = 2166136261u32;
        let mut pads = [pad::State::default(); 4];
        for (i, v) in pads.iter_mut().enumerate() {
            v.target = 48.0 + 4.0 * i as f32;
            v.amp = 0.1 + 0.1 * i as f32;
            v.hf = 1.0;
        }
        let mut bass = drone::State {
            gtarget: 36.0,
            amp: 0.01,
            ..Default::default()
        };
        let mut acid = acid::State::default();
        let mut drums = [Drum::default(); 6];
        for v in &mut drums {
            v.on = 1;
            v.rn = 3;
            v.wait = 17;
            v.amp = 0.4;
        }
        let mut line = drone::Line {
            env: 0.5,
            ..Default::default()
        };
        let mut slide = 0u8;
        let mut held = [[0u8; 8]; 3];
        let mut counts = [0u8; 3];
        let mut sequence = 0;
        let mut held_ui = 0u8;
        let mut motion = [setup::Motion::default(); 10];
        for (i, v) in motion.iter_mut().enumerate() {
            v.value = i as f32 * 0.1;
            v.left = i as u32 * 16;
            v.rate = i as f32 * 0.0001;
        }
        let mut lfo = setup::Lfo {
            phase: 0.999,
            value: 0.0,
            from: -0.5,
            to: 0.75,
        };
        let mut rng = Rng(0x173894af);
        if (85..91).contains(&test) {
            bass.env = 0.2;
            bass.env_up = ((test - 85) % 2) as u8;
        }
        for j in 0..1024 {
            let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][j % 9];
            if test < 84 {
                setup::lfo(
                    &mut lfo,
                    (test / 7) as i32,
                    (test % 7) as i32,
                    n as f32,
                    200.0,
                    &mut rng,
                );
                for v in [lfo.phase, lfo.value, lfo.from, lfo.to] {
                    hash = hash_word(hash, v.to_bits());
                }
                hash = hash_word(hash, rng.0);
            } else if test == 84 {
                setup::motion(&mut motion, n, &mut rng);
                for v in motion {
                    for f in [v.value, v.target, v.rate] {
                        hash = hash_word(hash, f.to_bits());
                    }
                    hash = hash_word(hash, v.left);
                }
                hash = hash_word(hash, rng.0);
            } else if test < 91 {
                let knob = ((test - 85) / 2 * 50) as i32;
                setup::drone_envelope(&mut bass, n, knob, knob);
                hash = hash_word(hash, bass.env.to_bits());
                hash = byte(hash, bass.env_up);
            } else {
                let voice = test - 100;
                let keys = [48, 55, 60, 64, 67, 72, 76, 79, 84, 88, 91, 96];
                let mut play = |key, velocity| {
                    let c = setup::KeyConfig {
                        voice: voice as i32,
                        key,
                        velocity,
                        octaves: [-1, 1, 0],
                        oscillator: 6,
                        playing: 1,
                        bass_line: 0,
                        acid_decay: 100,
                        tuning: 1.0,
                    };
                    let (ui, _, e) = setup::key(
                        &mut pads,
                        &mut bass,
                        &mut acid,
                        &mut held,
                        &mut counts,
                        &mut sequence,
                        &c,
                    );
                    held_ui = ui;
                    hash = events(hash, &e);
                };
                if j % 128 == 127 {
                    setup::quiet(&mut pads, &mut bass, &mut acid, &mut drums, &mut counts);
                    held_ui = 0;
                    for channel in [0, 1, 2, 3, 9] {
                        for w in [0xb0 + channel, 123, 0] {
                            hash = hash_word(hash, w);
                        }
                    }
                } else if j % 32 == 31 {
                    for key in keys {
                        play(key, 0);
                    }
                } else if j % 32 == 15 && voice == 0 {
                    setup::pad_handoff(&mut pads, 6);
                } else if j % 32 == 20 && voice == 0 {
                    setup::pad_leave(&mut pads, &[0, 4, 7]);
                } else if j % 32 == 25 && voice == 0 {
                    let (_, e) = setup::stab(&mut pads, 6, 0.75, 100);
                    hash = events(hash, &e);
                } else if j % 32 == 15 && voice == 1 {
                    let (s, e) =
                        setup::bass_note(&mut bass, &mut line, 31, (j % 4) as i32, 0, 7, 200.0);
                    slide = s;
                    hash = events(hash, &e);
                } else if j % 32 == 20 && voice == 1 {
                    let e = setup::bass_to(&mut bass, 7, 1, false, true);
                    hash = events(hash, &e);
                } else if j % 32 == 15 && voice == 2 {
                    let c = setup::AcidConfig {
                        note: 60,
                        slid_ok: (j % 2) as u32,
                        accent: u32::from(j % 3 == 0),
                        hat: 1,
                        slide: u32::from(j % 4 == 0),
                        hat_amount: 100,
                        decay: 0.123,
                        step: 200.0,
                        tuning: 1.0,
                    };
                    let e = setup::acid(&mut acid, &c);
                    hash = events(hash, &e);
                } else {
                    play(
                        keys[j % 12],
                        if j % 5 == 4 {
                            0
                        } else if j % 3 == 0 {
                            127
                        } else if j % 3 == 1 {
                            100
                        } else {
                            30
                        },
                    );
                }
                for p in &pads {
                    for w in pad_words(p) {
                        hash = hash_word(hash, w);
                    }
                }
                for w in drone_words(&bass).into_iter().chain(acid_words(&acid)) {
                    hash = hash_word(hash, w);
                }
                for v in &drums {
                    for w in drum_words(v) {
                        hash = hash_word(hash, w);
                    }
                }
                for row in held {
                    for b in row {
                        hash = byte(hash, b);
                    }
                }
                for b in counts {
                    hash = byte(hash, b);
                }
                hash = hash_word(hash, sequence);
                hash = byte(hash, held_ui);
                hash = hash_word(hash, line.env.to_bits());
                hash = hash_word(hash, line.gate.to_bits());
                hash = byte(hash, line.up);
                hash = byte(hash, slide);
            }
        }
        assert_eq!(
            hash,
            case["hash"].as_u64().unwrap() as u32,
            "scenario {test}"
        );
    }
}
#[test]
fn every_immutable_voice_recipe_matches_frozen_c_bits() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("voice-data-native.json")).unwrap();
    let mut actual: Vec<(&str, Vec<u32>)> = Vec::new();
    actual.push(("SINT", data::SINT.into_iter().map(f32::to_bits).collect()));

    {
        let mut w = Vec::new();
        for row in data::KITS {
            w.push(row.k_start.to_bits());
            w.push(row.k_drop.to_bits());
            w.push(row.k_dec0.to_bits());
            w.push(row.k_dec1.to_bits());
            w.push(row.k_click.to_bits());
            w.push(row.k_drive.to_bits());
            w.push(row.k_rumble.to_bits());
            w.push(row.sn_f1.to_bits());
            w.push(row.sn_f2.to_bits());
            w.push(row.sn_noise.to_bits());
            w.push(row.sn_body.to_bits());
            w.push(row.sn_dec.to_bits());
            w.push(row.cl_gap.to_bits());
            w.push(row.cl_dec.to_bits());
            w.push(row.h_metal.to_bits());
            w.push(row.h_noise.to_bits());
            w.push(row.h_dec.to_bits());
            w.push(row.oh_dec.to_bits());
            w.push(row.p_freq.to_bits());
            w.push(row.p_ratio.to_bits());
            w.push(row.p_idx.to_bits());
            w.push(row.duck.to_bits());
            w.push(row.chop.to_bits());
            w.push(row.crush.to_bits());
        }
        actual.push(("KITS", w));
    }
    {
        let mut w = Vec::new();
        for row in data::PAD_SOUND {
            w.push(
                row.osc as u32
                    | ((row.stabs as u32) << 8)
                    | ((row.padding[0] as u32) << 16)
                    | ((row.padding[1] as u32) << 24),
            );
            w.push(row.fc.to_bits());
            w.push(row.g.to_bits());
        }
        actual.push(("PAD_SOUND", w));
    }
    {
        let mut w = Vec::new();
        for row in data::DRONE_SOUND {
            w.push(
                row.osc as u32
                    | ((row.padding[0] as u32) << 8)
                    | ((row.padding[1] as u32) << 16)
                    | ((row.padding[2] as u32) << 24),
            );
            w.push(row.saw.to_bits());
            w.push(row.sub.to_bits());
            w.push(row.drive.to_bits());
            w.push(row.cut.to_bits());
            w.push(row.g.to_bits());
        }
        actual.push(("DRONE_SOUND", w));
    }
    {
        let mut w = Vec::new();
        for row in data::ACID_SOUND {
            w.push(
                row.osc as u32
                    | ((row.padding[0] as u32) << 8)
                    | ((row.padding[1] as u32) << 16)
                    | ((row.padding[2] as u32) << 24),
            );
            w.push(row.drive.to_bits());
            w.push(row.g.to_bits());
        }
        actual.push(("ACID_SOUND", w));
    }
    {
        let mut w = Vec::new();
        for row in data::ACID_FM {
            w.push(row.fr.to_bits());
            w.push(row.i0.to_bits());
            w.push(row.i1.to_bits());
            w.push(
                row.rise as u32
                    | ((row.padding[0] as u32) << 8)
                    | ((row.padding[1] as u32) << 16)
                    | ((row.padding[2] as u32) << 24),
            );
        }
        actual.push(("ACID_FM", w));
    }
    {
        let mut w = Vec::new();
        for row in data::CM_SOUND {
            w.push(row.fr.to_bits());
            w.push(row.i0.to_bits());
            w.push(row.i1.to_bits());
            w.push(
                row.rise as u32
                    | ((row.padding[0] as u32) << 8)
                    | ((row.padding[1] as u32) << 16)
                    | ((row.padding[2] as u32) << 24),
            );
            w.push(row.g.to_bits());
        }
        actual.push(("CM_SOUND", w));
    }
    actual.push((
        "DRONE_FR",
        data::DRONE_FR.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "PAD_PAN",
        data::PAD_PAN.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "BREATH",
        data::BREATH.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "START_T",
        data::START_T.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "VOWEL",
        data::VOWEL
            .into_iter()
            .flatten()
            .map(f32::to_bits)
            .collect(),
    ));
    actual.push((
        "HAT_INC",
        data::HAT_INC.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "LFO_RATE_BEATS",
        data::LFO_RATE_BEATS.into_iter().map(f32::to_bits).collect(),
    ));
    actual.push((
        "DRUM_GM",
        data::DRUM_GM.into_iter().map(|v| v as u32).collect(),
    ));
    actual.push((
        "EN_TRIM",
        data::EN_TRIM
            .into_iter()
            .flatten()
            .map(|v| v as u32)
            .collect(),
    ));
    for (name, words) in actual {
        let expected: Vec<u32> = fixture[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(words, expected, "{name}");
    }
}

#[test]
fn every_envelope_knob_and_invalid_lfo_rate_match_the_c_contract() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("voice-setup-native.json")).unwrap();
    for (i, v) in fixture["times"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            setup::time((i / 101) as u32, (i % 101) as i32).to_bits(),
            v.as_u64().unwrap() as u32
        );
    }
    for invalid in [-1, 12, i32::MIN, i32::MAX] {
        let mut a = setup::Lfo {
            phase: 0.99,
            ..Default::default()
        };
        let mut b = a;
        let mut r = Rng(91);
        let mut q = Rng(91);
        setup::lfo(&mut a, invalid, 5, 128.0, 100.0, &mut r);
        setup::lfo(&mut b, 0, 5, 128.0, 100.0, &mut q);
        assert_eq!(
            [a.phase, a.value, a.from, a.to],
            [b.phase, b.value, b.from, b.to]
        );
        assert_eq!(r.0, q.0);
    }
}
