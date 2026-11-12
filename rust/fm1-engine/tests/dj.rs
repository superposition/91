// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{
    sequencer::Rng,
    voices::{dj::*, Mix},
};
fn words(v: &serde_json::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
fn state(w: &[u32]) -> State {
    let f = |i| f32::from_bits(w[i]);
    State {
        on: w[0] as u8,
        held: (w[0] >> 8) as u8,
        key: (w[0] >> 16) as u8,
        kind: (w[0] >> 24) as u8,
        hold_mode: w[1] as u8,
        padding: [(w[1] >> 8) as u8, (w[1] >> 16) as u8, (w[1] >> 24) as u8],
        hz: f(2),
        aux: f(3),
        time: f(4),
        phase: f(5),
        phase2: f(6),
        phase3: f(7),
        envelope: f(8),
        z: f(9),
        z2: f(10),
        age: w[11],
    }
}
fn state_words(s: &State) -> [u32; 12] {
    [
        s.on as u32 | ((s.held as u32) << 8) | ((s.key as u32) << 16) | ((s.kind as u32) << 24),
        s.hold_mode as u32
            | ((s.padding[0] as u32) << 8)
            | ((s.padding[1] as u32) << 16)
            | ((s.padding[2] as u32) << 24),
        s.hz.to_bits(),
        s.aux.to_bits(),
        s.time.to_bits(),
        s.phase.to_bits(),
        s.phase2.to_bits(),
        s.phase3.to_bits(),
        s.envelope.to_bits(),
        s.z.to_bits(),
        s.z2.to_bits(),
        s.age,
    ]
}
fn hash_word(mut h: u32, w: u32) -> u32 {
    for b in w.to_le_bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}
fn events(test: usize, block: usize, s: &mut [State; 4], age: &mut u32) {
    let hold =
        test == 203 || test == 207 || test == 210 || test < 162 && !(test / 3).is_multiple_of(2);
    let mode = if test < 162 { test % 3 } else { 0 } as i32;
    if test < 162 && block == 128 {
        note(s, age, (test / 6) as i32, false, mode, hold);
    }
    if test == 203 && block == 2048 || test == 207 && block == 128 {
        note(s, age, 0, false, mode, hold);
    }
    if test == 210 {
        if block == 32 {
            note(s, age, 0, false, mode, hold);
        }
        if block == 64 {
            note(s, age, 3, true, mode, hold);
        }
        if block == 128 {
            for key in 0..27 {
                note(s, age, key, false, mode, hold);
            }
        }
        if block == 256 {
            note(s, age, 14, true, mode, hold);
        }
    }
}
#[test]
fn all_snips_octaves_modes_expiry_mixing_rng_and_null_match_frozen_c() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("dj-native.json")).unwrap();
    let sine: Vec<f32> = words(&corpus["sine"])
        .iter()
        .map(|v| f32::from_bits(*v))
        .collect();
    let table: &[f32; 1025] = sine.as_slice().try_into().unwrap();
    let mut saw_zero = false;
    let mut saw_default = false;
    let mut saw_wrap = false;
    for case in corpus["cases"].as_array().unwrap() {
        let test = case["test"].as_u64().unwrap() as usize;
        let w = words(&case["state"]);
        let mut s = core::array::from_fn(|i| state(&w[i * 12..i * 12 + 12]));
        let cfg = words(&case["config"]);
        let c = Config {
            mix: cfg[0] as i32,
            echo: cfg[1] as i32,
            gain0: f32::from_bits(cfg[2]),
            gain1: f32::from_bits(cfg[3]),
            inverse: f32::from_bits(cfg[4]),
        };
        let mut rng = Rng(case["seed"].as_u64().unwrap() as u32);
        let initial_seed = rng.0;
        let mut age = case["age"].as_u64().unwrap() as u32;
        let mut hash = 2166136261u32;
        let mut l = [0.0f32; 256];
        let mut r = [0.0f32; 256];
        let mut rv = [0.0f32; 256];
        let mut ec = [0.0f32; 256];
        let mut total_samples = 0;
        for block in 0..case["blocks"].as_u64().unwrap() as usize {
            let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][block % 9];
            events(test, block, &mut s, &mut age);
            l.fill(0.0);
            r.fill(0.0);
            rv.fill(0.0);
            ec.fill(0.0);
            if test != 206 && test != 207 && test != 208 {
                for j in 0..n {
                    l[j] = (j % 7) as f32 * 0.0001;
                    r[j] = -((j % 9) as f32) * 0.0002;
                    rv[j] = (j % 5) as f32 * 0.0003;
                    ec[j] = -((j % 3) as f32) * 0.0004;
                }
            }
            let result = process(
                &mut s,
                &c,
                &mut rng,
                table,
                Mix {
                    left: &mut l[..n],
                    right: &mut r[..n],
                    reverb: &mut rv[..n],
                    echo: &mut ec[..n],
                },
            );
            total_samples += result.samples;
            for values in [&l[..n], &r[..n], &rv[..n], &ec[..n]] {
                for v in values {
                    assert!(v.is_finite() && v.abs() < 64.0);
                    hash = hash_word(hash, v.to_bits());
                }
            }
            for v in &s {
                for w in state_words(v) {
                    hash = hash_word(hash, w);
                }
            }
            hash = hash_word(hash, rng.0);
            hash = hash_word(hash, age);
            hash = (hash ^ (result.active as u8) as u32).wrapping_mul(16777619);
        }
        assert_eq!(
            hash,
            case["hash"].as_u64().unwrap() as u32,
            "DJ case {test}"
        );
        if test == 206 {
            assert_eq!(total_samples, 0);
            assert_eq!(rng.0, initial_seed);
            assert_eq!(case["peak"].as_f64().unwrap(), 0.0);
            saw_zero = true;
        }
        if test == 207 {
            assert!(total_samples > 0);
            assert_ne!(
                rng.0, initial_seed,
                "gain0 still consumes all scheduled RNG draws"
            );
            assert_eq!(case["peak"].as_f64().unwrap(), 0.0);
        }
        if test == 200 || test == 201 || test == 202 || test == 203 {
            assert!(
                s.iter().all(|v| v.on == 0),
                "long natural/held release expires {test}"
            );
        }
        if test == 205 {
            assert_eq!(s[0].on, 1);
            saw_default = true;
        }
        if test == 210 {
            assert!(age < 0xfffffffdu32);
            saw_wrap = true;
        }
    }
    assert!(saw_zero && saw_default && saw_wrap);
}
#[test]
fn recipes_names_capacity_steal_release_and_age_wrap_match_frozen_c() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("dj-native.json")).unwrap();
    for (i, v) in corpus["names"].as_array().unwrap().iter().enumerate() {
        let bytes = name(i as i32);
        assert_eq!(bytes.last(), Some(&0));
        assert_eq!(
            std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap(),
            v.as_str().unwrap()
        );
    }
    assert_eq!(name(-1), name(0));
    assert_eq!(name(27), name(0));
    for case in corpus["cases"].as_array().unwrap() {
        let test = case["test"].as_u64().unwrap() as usize;
        if test >= 162 {
            continue;
        }
        let mut s = core::array::from_fn(|i| State {
            on: 0,
            held: 0,
            hold_mode: 0,
            key: (i + 1) as u8,
            kind: 4,
            padding: [0x5a; 3],
            hz: 600.0,
            aux: 0.12,
            time: 0.0,
            phase: 0.1,
            phase2: 0.2,
            phase3: 0.3,
            envelope: 0.25,
            z: 0.01,
            z2: -0.01,
            age: 5 + i as u32,
        });
        let mut age = 0;
        assert_eq!(
            note(
                &mut s,
                &mut age,
                (test / 6) as i32,
                true,
                (test % 3) as i32,
                !(test / 3).is_multiple_of(2)
            ),
            Some(0)
        );
        let actual: Vec<u32> = s.iter().flat_map(state_words).collect();
        assert_eq!(actual, words(&case["state"]), "recipe {test}");
        assert_eq!(age, case["age"].as_u64().unwrap() as u32);
    }
    let first = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["test"] == 210)
        .unwrap();
    let w = words(&first["state"]);
    let mut s = core::array::from_fn(|i| state(&w[i * 12..i * 12 + 12]));
    let mut age = first["age"].as_u64().unwrap() as u32;
    let mut hash = 2166136261u32;
    for step in 0..40 {
        let key = if step % 10 == 0 {
            -1
        } else if step % 10 == 1 {
            27
        } else if step % 10 == 2 {
            99
        } else {
            step * 7 % 27
        };
        note(&mut s, &mut age, key, step % 4 != 3, 0, true);
        for s in &s {
            for w in state_words(s) {
                hash = hash_word(hash, w);
            }
        }
        hash = hash_word(hash, age);
        assert_eq!(
            hash,
            corpus["allocation"][step as usize].as_u64().unwrap() as u32,
            "allocation {step}"
        );
    }
    assert_eq!(corpus["stop_panic_init_preserve_dj"], true);
}
