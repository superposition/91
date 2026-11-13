// SPDX-License-Identifier: GPL-3.0-only
//! Recorded from the qualified citypop C engine, before release Rust integration.
//! See chord_reference.c for the explicit regeneration procedure.
use fm1_engine::planner::{bass_event, plan_into, Phrase, Seed, Settings};
use fm1_formats::jam::{JamPack, V2_HEAD};
fn bytes(h: &mut u32, data: &[u8]) {
    for b in data {
        *h = (*h ^ u32::from(*b)).wrapping_mul(16777619);
    }
}
#[test]
fn qualified_c_literal_plans_bass_and_rng_match_exactly() {
    let mut data = include_bytes!("packs/literal-chords.jmp").to_vec();
    for (voicing, want) in [(0, 0xb7227711), (1, 0x063797b1)] {
        data[V2_HEAD + 21] = voicing;
        let pack = JamPack::parse(&data).unwrap();
        let mut h = 2166136261u32;
        for key in 0..12 {
            for minor in 0..2 {
                for bars in [8, 16] {
                    for form in 0..4 {
                        for movement in 0..4 {
                            let s = Settings {
                                key,
                                minor,
                                bars,
                                form,
                                movement,
                                energy: 60,
                                ..Settings::default()
                            };
                            let mut a = Phrase::EMPTY;
                            let mut next = Phrase::EMPTY;
                            let mut seed = Seed::new(91);
                            for section in 0..4 {
                                plan_into(
                                    pack,
                                    &s,
                                    section,
                                    if section == 0 { None } else { Some(&a) },
                                    &mut next,
                                    &mut seed,
                                )
                                .unwrap();
                                bytes(&mut h, &[next.bars, next.role, next.motif, next.section]);
                                for beat in next.beats {
                                    bytes(&mut h, &beat.notes);
                                    bytes(&mut h, &[beat.node, beat.passing]);
                                }
                                bytes(&mut h, &seed.state().to_le_bytes());
                                a = next;
                                for bar in 0..bars as usize {
                                    for step in 0..16 {
                                        let event =
                                            bass_event(pack, &s, &a, &next, bar, step).unwrap();
                                        bytes(&mut h, &(event.note as i32).to_le_bytes());
                                        bytes(&mut h, &(event.velocity as u32).to_le_bytes());
                                        bytes(&mut h, &(event.gate as u32).to_le_bytes());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(h, want, "authored voicing {voicing}");
    }
}
