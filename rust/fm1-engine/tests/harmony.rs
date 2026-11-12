// SPDX-License-Identifier: GPL-3.0-only
//! Inputs enumerate the independent R01 C oracle corpus, not a second policy.
use fm1_engine::{
    harmony::{
        abi::{Config, Mood, State},
        basic, legacy,
    },
    sequencer::Rng,
};
fn record(hash: &mut u32, value: i32) {
    for b in value.to_le_bytes() {
        *hash = (*hash ^ b as u32).wrapping_mul(16777619);
    }
}
fn golden(mode: &str, hash: u32, rows: u32) {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("harmony-native-golden.json")).unwrap();
    assert_eq!(data[mode]["trace_hash"], format!("{hash:08x}"));
    assert_eq!(data[mode]["rows"], rows);
}
#[test]
fn all_pitch_pools_signed_roots_degree_wraps_and_chord_kinds_match_frozen_c() {
    let mut hash = 2166136261;
    let mut rows = 0;
    for mask in 0..4096 {
        let scale = basic::scale(mask | 0xf0000000);
        let mut steps = [0x6a6a6a6a; 12];
        steps[..scale.count].copy_from_slice(&scale.steps[..scale.count]);
        record(&mut hash, scale.count as i32);
        for n in steps {
            record(&mut hash, n);
        }
        rows += 1;
        for root in [-24, -1, 0, 1, 5, 11, 12, 100000] {
            for degree in [-1, 0, scale.count as i32 + 1] {
                for kind in -1..=8 {
                    let chord = basic::chord(mask, root, degree, kind);
                    let mut pcs = [0x5b5b5b5b; 4];
                    pcs[..chord.count].copy_from_slice(&chord.pcs[..chord.count]);
                    record(&mut hash, chord.count as i32);
                    for n in pcs {
                        record(&mut hash, n);
                    }
                    rows += 1;
                }
            }
        }
    }
    golden("0", hash, rows);
}
#[test]
fn every_partial_from_to_overlap_preserves_c_candidate_write_and_tie_order() {
    let mut hash = 2166136261;
    let mut rows = 0;
    for seed in 0..24 {
        for f in 0..5 {
            for t in 0..5 {
                for nt in 0..=4 {
                    for (lo, hi) in [(48, 77), (24, 53), (79, 108), (48, 53)] {
                        let mut arena = [0; 16];
                        for (i, n) in arena.iter_mut().enumerate() {
                            *n = 36 + (i as i32 * 7 + seed * 3) % 65;
                        }
                        for (i, n) in [0, 4, 7, 11].iter().enumerate() {
                            arena[8 + i] = *n + seed % 12;
                        }
                        let from = arena[f..f + 4].try_into().unwrap();
                        let pcs = arena[8..12].try_into().unwrap();
                        let mut aliases = [-1; 4];
                        for (v, a) in aliases.iter_mut().enumerate() {
                            for j in 0..4 {
                                if t + v == f + j {
                                    *a = j as i8;
                                }
                            }
                        }
                        let result = basic::voice_lead(from, pcs, nt, lo, hi, aliases);
                        arena[t..t + 4].copy_from_slice(&result);
                        for n in arena {
                            record(&mut hash, n);
                        }
                        rows += 1;
                    }
                }
            }
        }
    }
    golden("1", hash, rows);
}
#[test]
fn capture_preserves_metadata_and_the_existing_chromatic_three_bit_wrap() {
    let s = State {
        nsteps: 12,
        deg: 11,
        pool: 0xfff,
        ..State::default()
    };
    let c = Config {
        drift: 50,
        ..Config::default()
    };
    assert_eq!(
        legacy::capture(&s, &c, [0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xe7]),
        [4, 0, 0, 0xaa, 0xaa, 0xe7]
    );
}
#[test]
fn stopped_arc_holds_its_position_and_timer_expiry_consumes_exactly_three_words() {
    let mut state = Mood {
        arc: 0.3,
        arc_tgt: -0.8,
        brk: 0.0,
        arcm: 0.0,
        arc_left: 256,
    };
    let mut rng = Rng(91);
    legacy::mood(&mut state, 128, false, 1.0, 0, &mut rng);
    assert_eq!(state.arc.to_bits(), 0.3f32.to_bits());
    assert_eq!(state.arc_left, 128);
    assert_eq!(rng.0, 91);
    let mut expected = Rng(91);
    for _ in 0..3 {
        expected.next_u32();
    }
    legacy::mood(&mut state, 128, false, 0.0, 0, &mut rng);
    assert_eq!(rng.0, expected.0);
    assert_eq!(state.arc.to_bits(), 0.3f32.to_bits());
    assert_eq!(state.arcm, 0.0);
    assert!((2_205_000..6_615_000).contains(&state.arc_left));
}
