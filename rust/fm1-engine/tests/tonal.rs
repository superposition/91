// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::voices::{acid, drone};
fn words(v: &serde_json::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as u32)
        .collect()
}
fn floats(w: &[u32]) -> Vec<f32> {
    w.iter().map(|w| f32::from_bits(*w)).collect()
}
fn drone_state(w: &[u32]) -> drone::State {
    let f = |i| f32::from_bits(w[i]);
    drone::State {
        ph: [f(0), f(1), f(2)],
        sph: f(3),
        mph: f(4),
        note: f(5),
        target: f(6),
        amp: f(7),
        s: [f(8), f(9), f(10), f(11)],
        c1: [f(12), f(13), f(14), f(15)],
        c2: [f(16), f(17), f(18), f(19)],
        env: f(20),
        gtarget: f(21),
        midi_note: w[22] as u8,
        rest: (w[22] >> 8) as u8,
        env_up: (w[22] >> 16) as u8,
        play: (w[22] >> 24) as u8,
        velocity: f(23),
    }
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
fn drone_config(w: &[u32]) -> drone::Config {
    let f = |i| f32::from_bits(w[i]);
    drone::Config {
        oscillator: w[0],
        filter: w[1] as i32,
        line: w[2],
        playing: w[3],
        drive: f(4),
        k1: f(5),
        k2: f(6),
        input: f(7),
        gain: f(8),
        saw: f(9),
        sub: f(10),
        target: f(11),
        rate: f(12),
        fm_index: f(13),
        pulse_width: f(14),
        glide: f(15),
        pitch: f(16),
        bend: f(17),
        tuning: f(18),
        fm_ratio: f(19),
        cutoff: f(20),
        envelope: f(21),
        motion: f(22),
        arc: f(23),
        break_pos: f(24),
        break_neg: f(25),
        duck: f(26),
        links: f(27),
        gain0: f(28),
        gain1: f(29),
        inverse: f(30),
    }
}
fn acid_state(w: &[u32]) -> acid::State {
    let f = |i| f32::from_bits(w[i]);
    acid::State {
        ph: f(0),
        inc: f(1),
        inc_t: f(2),
        amp: f(3),
        amp_t: f(4),
        env: f(5),
        edec: f(6),
        acc: f(7),
        s: [f(8), f(9), f(10), f(11)],
        c1: [f(12), f(13), f(14), f(15)],
        c2: [f(16), f(17), f(18), f(19)],
        gate_left: w[20] as i32,
        note: w[21] as u8,
        sliding: (w[21] >> 8) as u8,
        play: (w[21] >> 16) as u8,
        ps: f(22),
        p3: f(23),
        p4: f(24),
        p5: f(25),
        lph: f(26),
    }
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
fn acid_config(w: &[u32]) -> acid::Config {
    let f = |i| f32::from_bits(w[i]);
    acid::Config {
        oscillator: w[0],
        filter: w[1] as i32,
        gain: f(2),
        drive: f(3),
        resonance: f(4),
        envelope: f(5),
        accent: f(6),
        base: f(7),
        k1: f(8),
        k2: f(9),
        input: f(10),
        wow: f(11),
        velocity: f(12),
        glide: f(13),
        bend: f(14),
        gain0: f(15),
        gain1: f(16),
        inverse: f(17),
        fm_ratio: f(18),
        fm_index0: f(19),
        fm_index1: f(20),
        fm_rise: w[21],
    }
}
fn hash_word(mut h: u32, w: u32) -> u32 {
    for b in w.to_le_bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}
#[test]
fn every_drone_and_acid_sound_controls_extremes_and_releases_match_frozen_c() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("tonal-native.json")).unwrap();
    let sine: serde_json::Value = serde_json::from_str(include_str!("voices-native.json")).unwrap();
    let table: [f32; 1025] = floats(&words(&sine["sine"])).try_into().unwrap();
    let mut count = 0;
    for case in corpus["cases"].as_array().unwrap() {
        let id = case["test"].as_u64().unwrap();
        let w = words(&case["state"]);
        let cfg = words(&case["config"]);
        let mut ds = if id < 100 {
            drone_state(&w)
        } else {
            drone::State::default()
        };
        let mut ac = if id >= 100 {
            acid_state(&w)
        } else {
            acid::State::default()
        };
        let dc = if id < 100 {
            Some(drone_config(&cfg))
        } else {
            None
        };
        let cc = if id >= 100 {
            Some(acid_config(&cfg))
        } else {
            None
        };
        let mut line = if id < 100 {
            let w = words(&case["line"]);
            drone::Line {
                env: f32::from_bits(w[0]),
                gate: f32::from_bits(w[1]),
                up: case["up"].as_u64().unwrap() as u8,
            }
        } else {
            drone::Line::default()
        };
        let mut control = case["control"].as_u64().unwrap() as u32;
        let mut hash = 2166136261u32;
        let mut peak = 0.0f32;
        for block in 0..256 {
            let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][block % 9];
            let mut out = [0.0; 256];
            let level = if let Some(c) = dc.as_ref() {
                drone::process(&mut ds, c, &mut line, &mut control, &table, &mut out[..n])
            } else {
                acid::process(
                    &mut ac,
                    cc.as_ref().unwrap(),
                    &mut control,
                    &table,
                    &mut out[..n],
                )
            };
            for v in &out[..n] {
                assert!(v.is_finite(), "case {id}");
                peak = peak.max(v.abs());
                hash = hash_word(hash, v.to_bits());
            }
            if id < 100 {
                for w in drone_words(&ds) {
                    hash = hash_word(hash, w);
                }
                for w in [line.env.to_bits(), line.gate.to_bits(), line.up as u32] {
                    hash = hash_word(hash, w);
                }
            } else {
                for w in acid_words(&ac) {
                    hash = hash_word(hash, w);
                }
            }
            hash = hash_word(hash, control);
            hash = hash_word(hash, level.to_bits());
        }
        assert_eq!(hash, case["hash"].as_u64().unwrap() as u32, "case {id}");
        assert!(peak < 64.0, "case {id}");
        count += 1;
    }
    assert_eq!(count, 222);
}
