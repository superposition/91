// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{
    effects::math::exp,
    voices::{config, data, pad, parameters, setup},
};
use fm1_types::ParamId as P;
#[test]
fn every_voice_parameter_value_identity_and_hat_recipe_matches_real_frozen_c() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("voice-parameters-native.json")).unwrap();
    for row in v["curves"].as_array().unwrap() {
        let row = row.as_array().unwrap();
        let id = row[0].as_u64().unwrap() as usize;
        let value = row[1].as_i64().unwrap() as i32;
        let expected = row[2].as_u64().unwrap() as u32;
        assert_eq!(
            parameters::value(id, value).unwrap().to_bits(),
            expected,
            "parameter {id}={value}"
        );
    }
    assert!(parameters::value(P::ECHO.index(), 50).is_none());
    let mut params = [0i16; fm1_types::PARAMETER_COUNT];
    for row in v["identity"].as_array().unwrap() {
        let row = row.as_array().unwrap();
        params[P::PAD_COMP.index()] = row[1].as_i64().unwrap() as i16;
        params[P::PADTONE.index()] = row[2].as_i64().unwrap() as i16;
        assert_eq!(
            config::pad_identity(&params, row[0].as_u64().unwrap() as u32, &data::PAD_SOUND),
            (
                row[3].as_u64().unwrap() as u32,
                row[4].as_u64().unwrap() as u32
            )
        );
    }
    for (i, row) in v["hats"].as_array().unwrap().iter().enumerate() {
        let actual = parameters::hat_increments(data::KITS[i].h_metal, &data::HAT_INC);
        let expected: Vec<u32> = row
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(actual.map(f32::to_bits).as_slice(), expected);
    }
    for (i, bits) in v["acid_decay"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            parameters::acid_decay(i as i32).to_bits(),
            bits.as_u64().unwrap() as u32
        );
    }
}
#[test]
fn note_velocity_midi_duration_and_fm_gate_match_actual_frozen_triggers() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("voice-parameters-native.json")).unwrap();
    let rows = v["notes"].as_array().unwrap();
    assert_eq!(rows.len(), 101 * 2 * 6);
    for row in rows {
        let row = row.as_array().unwrap();
        let fm = row[1].as_u64().unwrap() != 0;
        let mut slot = 0;
        let note = parameters::pluck_note(
            &mut slot,
            row[0].as_i64().unwrap() as i32,
            fm,
            f32::from_bits(row[2].as_u64().unwrap() as u32),
        );
        assert_eq!(note.slot, 1);
        assert_eq!(slot, 1);
        assert_eq!(note.velocity, row[3].as_u64().unwrap() as u32);
        assert_eq!(note.midi_samples, row[4].as_i64().unwrap() as i32);
        if fm {
            let observed = row[5].as_i64().unwrap() as i32;
            assert!(observed > 0, "frozen C must really allocate FM gates");
            assert_eq!(note.key_samples, observed);
        } else {
            assert_eq!(
                exp(-6.9 / (note.decay * 44100.0)).to_bits(),
                row[6].as_u64().unwrap() as u32
            );
        }
    }
    let manual = v["manual"].as_array().unwrap();
    assert_eq!(manual.len(), 324);
    let mut params = [0i16; fm1_types::PARAMETER_COUNT];
    for row in manual {
        let r = row.as_array().unwrap();
        params[P::OCT.index()] = r[0].as_i64().unwrap() as i16;
        params[P::CM_OCT.index()] = r[1].as_i64().unwrap() as i16;
        let out = parameters::manual_pluck(
            &params,
            r[2].as_i64().unwrap() as i32,
            f32::from_bits(r[3].as_u64().unwrap() as u32),
        );
        assert_eq!(out.note, r[4].as_i64().unwrap() as i32);
        assert_eq!(out.velocity.to_bits(), r[5].as_u64().unwrap() as u32);
    }
}
#[test]
fn reinitialize_preserves_unlisted_voice_state_and_motion_rates() {
    let mut pads = [pad::State {
        rate: 0.123,
        t: 0.75,
        sub: 0.33,
        play: 1,
        pnote: 61,
        pamp: 0.8,
        pord: 17,
        voices: 6,
        ic: [0.1, 0.2, 0.3, 0.4],
        c1: [1.0, 2.0, 3.0, 4.0],
        c2: [2.0, 3.0, 4.0, 5.0],
        inc: [0.01; 4],
        ..Default::default()
    }; 4];
    let mut motion = [setup::Motion {
        value: 0.5,
        target: -0.5,
        rate: 0.001,
        left: 123,
    }; 10];
    setup::initialize(&mut pads, &mut motion, &data::PAD_PAN);
    assert_eq!(pads.map(|p| p.note), [50.0, 57.0, 64.0, 71.0]);
    assert_eq!(pads.map(|p| p.pan), data::PAD_PAN);
    for p in pads {
        assert_eq!(p.ph, [0.0, 0.37, 0.61, 0.19]);
        assert_eq!(p.amp, 0.0);
        assert_eq!(
            (p.rate, p.t, p.sub, p.play, p.pnote, p.pamp, p.pord, p.voices),
            (0.123, 0.75, 0.33, 1, 61, 0.8, 17, 6)
        );
        assert_eq!(p.ic, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(p.c1, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(p.c2, [2.0, 3.0, 4.0, 5.0]);
        assert_eq!(p.inc, [0.01; 4]);
    }
    for s in motion {
        assert_eq!((s.value, s.target, s.rate, s.left), (0.0, 0.0, 0.001, 0));
    }
}
#[test]
fn lfo_destination_combinations_and_drum_links_match_actual_frozen_control() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("voice-parameters-native.json")).unwrap();
    let mut params = [0i16; fm1_types::PARAMETER_COUNT];
    let routes = v["routing"].as_array().unwrap();
    assert_eq!(routes.len(), 960);
    for row in routes {
        let r = row.as_array().unwrap();
        params[P::PL_DEST.index()] = r[0].as_i64().unwrap() as i16;
        params[P::DL_DEST.index()] = r[1].as_i64().unwrap() as i16;
        params[P::PL_DEPTH.index()] = r[2].as_i64().unwrap() as i16;
        params[P::DL_DEPTH.index()] = r[3].as_i64().unwrap() as i16;
        let pad = setup::Lfo {
            value: f32::from_bits(r[4].as_u64().unwrap() as u32),
            ..Default::default()
        };
        let drone = setup::Lfo {
            value: f32::from_bits(r[5].as_u64().unwrap() as u32),
            ..Default::default()
        };
        let out = setup::lfo_routes(&params, &pad, &drone);
        let actual = [
            out.filter,
            out.level,
            out.pan,
            out.pitch,
            out.acid_cut,
            out.acid_resonance,
            out.echo,
            out.punch,
        ]
        .map(f32::to_bits);
        let expected: Vec<u32> = r[6..].iter().map(|v| v.as_u64().unwrap() as u32).collect();
        assert_eq!(actual.as_slice(), expected);
    }
    let links = v["links"].as_array().unwrap();
    assert_eq!(links.len(), 72);
    for row in links {
        let r = row.as_array().unwrap();
        let mut s = setup::Links {
            kick: 0.2,
            snare: 0.3,
        };
        let on = r[0].as_u64().unwrap() != 0;
        let amount = r[1].as_i64().unwrap() as i32;
        setup::links_hit(
            &mut s,
            r[2].as_u64().unwrap() as u32,
            f32::from_bits(r[3].as_u64().unwrap() as u32),
            on,
        );
        assert_eq!(
            [s.kick.to_bits(), s.snare.to_bits()],
            [r[4].as_u64().unwrap() as u32, r[5].as_u64().unwrap() as u32]
        );
        setup::links_step(&mut s, 32, amount, amount, on);
        assert_eq!(
            [s.kick.to_bits(), s.snare.to_bits()],
            [r[6].as_u64().unwrap() as u32, r[7].as_u64().unwrap() as u32]
        );
    }
}
