// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::melodic::*;
use fm1_engine::sequencer::Rng;
fn harmony() -> Harmony {
    Harmony {
        steps: [0, 2, 4, 5, 7, 9, 11, 0, 0, 0, 0, 0],
        pcs: [0, 4, 7, 0],
        nsteps: 7,
        chord_count: 3,
        pool: 0xab5,
        ..Harmony::default()
    }
}
fn word(mut h: u32, n: u32) -> u32 {
    for byte in n.to_le_bytes() {
        h = (h ^ byte as u32).wrapping_mul(16_777_619);
    }
    h
}
#[test]
fn acid_generation_mutation_and_all_answer_previews_match_frozen_c() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("melodic-acid.json")).unwrap();
    for test in 0u32..64 {
        let mut h = harmony();
        h.degree = (test % 7) as i32;
        h.root = (test % 12) as i32;
        h.song_octave = (test % 7) as i32 - 3;
        let c = AcidConfig {
            length: 1 + (test % 16) as i32,
            density: (test * 17 % 101) as i32,
            range: 1 + (test % 3) as i32,
            slide: (test * 31 % 101) as i32,
            mutation: (test * 29 % 101) as i32,
            change: (test % 4) as i32,
            locked: i32::from(test % 7 == 0),
            octave: (test % 5) as i32 - 2,
            evolve: (test % 10) as f32 * 0.1,
            ..AcidConfig::default()
        };
        let mut state = Acid::default();
        let mut rng = Rng((test + 1).wrapping_mul(2_654_435_761));
        acid_regrow(&mut state, &c, &h, &mut rng);
        let mut hash = 2_166_136_261;
        for round in 0..9 {
            if round != 0 {
                acid_mutate(&mut state, &c, &h, &mut rng);
            }
            hash = word(word(word(hash, state.gate), state.accent), state.slide);
            for step in 0..16 {
                hash = word(
                    word(hash, state.offsets[step] as u8 as u32),
                    state.octaves[step] as u8 as u32,
                );
            }
            hash = word(hash, rng.0);
            for value in 0..45 {
                for step in 0..16 {
                    hash = word(hash, acid_preview(&state, &c, &h, step, value) as u32);
                }
            }
        }
        assert_eq!(
            hash,
            corpus[test as usize].as_u64().unwrap() as u32,
            "acid configuration {test}"
        );
    }
}
#[test]
fn zero_mutation_keeps_notes_but_still_consumes_the_legacy_rng_draws() {
    let h = harmony();
    let c = AcidConfig {
        length: 13,
        ..AcidConfig::default()
    };
    let mut s = Acid::default();
    let mut rng = Rng(91);
    let mut expected = Rng(91);
    acid_mutate(&mut s, &c, &h, &mut rng);
    for _ in 0..13 {
        expected.next_u32();
    }
    assert_eq!(rng.0, expected.0);
    assert_eq!(s.gate, 0);
}
#[test]
fn midi_gate_admission_handles_duplicate_full_and_equal_deadlines() {
    let notes = [60, 61, 62, 63, 64, 65, 66, 67];
    let waits = [256; 8];
    assert_eq!(midi_slot(&notes, &waits, 70), 1);
    assert_eq!(midi_slot(&notes, &waits, 63), (3 << 8) | (1 << 3));
    let mut notes = notes;
    notes[6] = 0;
    assert_eq!(midi_slot(&notes, &waits, 70), 6 << 8);
    let mut deadline = 0;
    assert!(midi_due(true, &mut deadline, 128));
    assert_eq!(deadline, -128);
    assert!(!midi_due(false, &mut deadline, 128));
    assert_eq!(deadline, -128);
}
#[test]
fn free_drift_requests_one_change_while_bar_harmony_waits_at_zero() {
    let mut left = 128.5;
    assert!(!harmonic_samples(false, false, &mut left, 128));
    assert_eq!(left, 0.5);
    assert!(harmonic_samples(false, false, &mut left, 1));
    assert_eq!(left, -0.5);
    assert!(harmonic_samples(true, false, &mut left, 1024));
    assert_eq!(left, -0.5);
    assert!(!harmonic_samples(false, true, &mut left, 1));
    assert_eq!(left.to_bits(), 0.0f32.to_bits());
    assert!(harmonic_samples(true, true, &mut left, 128));
    assert_eq!(left.to_bits(), 0.0f32.to_bits());
}
#[test]
fn stop_cancels_all_slots_and_start_resets_only_its_scheduler_state() {
    let mut waits = [99; 16];
    let mut ratchets = [5; 6];
    clear_pending(&mut waits, &mut ratchets);
    assert_eq!(waits, [0; 16]);
    assert_eq!(ratchets, [0; 6]);
    let mut s = Start {
        song_position: 32,
        sixteenth: u32::MAX,
        bars: 99,
        acid_position: 15,
        chain: 7,
        slice: 8,
        pattern: 3,
    };
    start(&mut s, 5);
    assert_eq!(s.song_position, 0);
    assert_eq!(s.pattern, 5);
    assert_eq!(transport_message(false, true, false), 0xfa);
    assert_eq!(transport_message(false, false, false), 0xfb);
    assert_eq!(transport_message(false, false, true), 0xfc);
    assert_eq!(transport_message(true, true, false), 0);
    assert_eq!(tick_actions(1, 1, 1, 3), [1, 2, 3, 4, 5 | (2 << 8)]);
}
