// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::voices::*;
fn words(value: &serde_json::Value) -> Vec<u32> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
fn drum(w: &[u32]) -> Drum {
    let f = |i| f32::from_bits(w[i]);
    Drum {
        ph: f(0),
        inc: f(1),
        inc_end: f(2),
        amp: f(3),
        adec: f(4),
        idx: f(5),
        x1: f(6),
        x2: f(7),
        att: f(8),
        mph: [f(9), f(10), f(11), f(12)],
        pend: f(13),
        t: f(14),
        hp: f(15),
        ph2: f(16),
        inc2: f(17),
        wait: w[18] as i32,
        wait2: w[19] as i32,
        pend2: f(20),
        rgap: w[21] as i32,
        rmul: f(22),
        xl: f(23),
        fo: f(24),
        rn: w[25] as u8,
        on: (w[25] >> 8) as u8,
        note: (w[25] >> 16) as u8,
    }
}
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
fn config(w: &[u32]) -> DrumsConfig {
    let f = |i| f32::from_bits(w[i]);
    DrumsConfig {
        gains: [f(0), f(1), f(2), f(3), f(4), f(5)],
        hat_inc: [f(6), f(7), f(8), f(9)],
        drive: f(10),
        drop: f(11),
        click: f(12),
        rumble: f(13),
        sn_body: f(14),
        sn_noise: f(15),
        cl_gap: f(16),
        h_noise: f(17),
        gains0: [f(18), f(19), f(20), f(21)],
        gains1: [f(22), f(23), f(24), f(25)],
        inverse: f(26),
    }
}
fn pluck_state(w: &[u32]) -> Pluck {
    let f = |i| f32::from_bits(w[i]);
    Pluck {
        ph: f(0),
        ps: f(1),
        inc: f(2),
        env: f(3),
        dm: f(4),
        vel: f(5),
        age: w[6],
    }
}
fn pluck_config(w: &[u32]) -> PluckConfig {
    let f = |i| f32::from_bits(w[i]);
    PluckConfig {
        gain: f(0),
        pitch: f(1),
        ratio: f(2),
        index0: f(3),
        index1: f(4),
        gain0: f(5),
        gain1: f(6),
        inverse: f(7),
        old_level: f(8),
        rise: w[9],
        fm: w[10],
    }
}
fn pluck_words(v: &Pluck) -> [u32; 7] {
    [
        v.ph.to_bits(),
        v.ps.to_bits(),
        v.inc.to_bits(),
        v.env.to_bits(),
        v.dm.to_bits(),
        v.vel.to_bits(),
        v.age,
    ]
}
fn hash_word(mut hash: u32, word: u32) -> u32 {
    for byte in word.to_le_bytes() {
        hash = (hash ^ byte as u32).wrapping_mul(16777619);
    }
    hash
}
fn table(corpus: &serde_json::Value) -> [f32; 1025] {
    words(&corpus["sine"])
        .iter()
        .map(|v| f32::from_bits(*v))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

#[test]
fn all_kits_plain_plucks_releases_and_irregular_blocks_match_frozen_c() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("voices-native.json")).unwrap();
    let table = table(&corpus);
    let mut cases = 0;
    for case in corpus["cases"].as_array().unwrap() {
        let id = case["test"].as_u64().unwrap();
        let state = words(&case["state"]);
        let cfg = words(&case["config"]);
        let mut drum_state = if id < 98 {
            core::array::from_fn(|i| drum(&state[i * 26..]))
        } else {
            [Drum::default(); 6]
        };
        let mut plucks = if id >= 100 {
            [pluck_state(&state[..7]), pluck_state(&state[7..])]
        } else {
            [Pluck::default(); 2]
        };
        let mut seed = case["seed"].as_u64().unwrap_or(0) as u32;
        let dc = if id < 98 { Some(config(&cfg)) } else { None };
        let mut pc = if id >= 100 {
            Some(pluck_config(&cfg))
        } else {
            None
        };
        let mut hash = 2166136261;
        let mut peak = 0.0f32;
        for block in 0..256 {
            let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][block % 9];
            let (mut left, mut right, mut reverb, mut echo) =
                ([0.0; 256], [0.0; 256], [0.0; 256], [0.0; 256]);
            let b = Mix {
                left: &mut left[..n],
                right: &mut right[..n],
                reverb: &mut reverb[..n],
                echo: &mut echo[..n],
            };
            if let Some(c) = dc.as_ref() {
                drums(&mut drum_state, c, &table, &mut seed, b);
            } else {
                let c = pc.as_mut().unwrap();
                let level = pluck(&mut plucks, c, &table, b);
                c.old_level = level;
            }
            for buffer in [&left, &right, &reverb, &echo] {
                for sample in &buffer[..n] {
                    assert!(sample.is_finite(), "case {id}");
                    hash = hash_word(hash, sample.to_bits());
                }
            }
            for j in 0..n {
                peak = peak.max(left[j].abs()).max(right[j].abs());
            }
            if id < 98 {
                for v in &drum_state {
                    for word in drum_words(v) {
                        hash = hash_word(hash, word);
                    }
                }
                hash = hash_word(hash, seed);
            } else {
                for v in &plucks {
                    for word in pluck_words(v) {
                        hash = hash_word(hash, word);
                    }
                }
                hash = hash_word(hash, pc.as_ref().unwrap().old_level.to_bits());
            }
        }
        assert_eq!(hash, case["hash"].as_u64().unwrap() as u32, "case {id}");
        assert!(peak < 8.0);
        cases += 1;
    }
    assert_eq!(cases, 136);
}

#[test]
fn sine_interpolation_wraps_negative_fm_and_exact_turns() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("voices-native.json")).unwrap();
    let t = table(&corpus);
    for phase in [-8.25, -1.5, -0.25, 0.0, 0.5, 0.75, 1.0, 8.25] {
        let wrapped = if phase < 0.0 { phase + 16.0 } else { phase };
        assert_eq!(sine_any(&t, phase).to_bits(), sine(&t, wrapped).to_bits());
    }
}
