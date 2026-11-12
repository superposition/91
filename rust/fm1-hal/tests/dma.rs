// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::dma::{Audio, AudioIo, Phase};
struct Model {
    half: u32,
    step: u32,
    advance_at: u32,
    stop_confirmed: bool,
    stopping: bool,
    writes: Vec<(u32, i32, bool)>,
    after_stop: u32,
}
impl Model {
    fn step(&mut self) {
        self.step += 1;
        if self.step == self.advance_at {
            self.half ^= 1;
        }
    }
}
impl AudioIo for Model {
    fn free_half(&mut self) -> u32 {
        let value = self.half;
        self.step();
        value
    }
    fn store_word(&mut self, index: u32, word: i32) {
        if self.stopping {
            self.after_stop += 1;
        }
        self.writes.push((index, word, self.half == 0));
        self.step();
    }
    fn request_stop(&mut self) {
        self.stopping = true;
        self.step();
    }
    fn stopped(&mut self) -> bool {
        self.step();
        self.stop_confirmed
    }
}
#[test]
fn every_observation_store_boundary_and_unknown_stop_retains_the_original_span() {
    for words in [512, 1024] {
        for confirmed in [false, true] {
            for advance in 1..=words * 3 + 2 {
                let mut audio = Audio::new();
                assert!(audio.configure(words, true));
                let token = audio.begin(0).unwrap();
                let mut bus = Model {
                    half: 0,
                    step: 0,
                    advance_at: advance,
                    stop_confirmed: confirmed,
                    stopping: false,
                    writes: vec![],
                    after_stop: 0,
                };
                for word in 0..words {
                    if !audio.publish(&mut bus, token, word, word as i32) {
                        break;
                    }
                }
                audio.finish(&mut bus, token);
                assert_eq!(bus.after_stop, 0);
                assert!(bus
                    .writes
                    .iter()
                    .all(|(index, value, _)| *index < words && *value == *index as i32));
                // Advancement immediately AFTER a free-half observation can allow one
                // raw word store. It never establishes exclusivity or whole-frame commit.
                assert!(bus.writes.iter().filter(|(_, _, free)| !*free).count() <= 1);
                if bus.stopping {
                    assert_eq!(
                        audio.phase(),
                        if confirmed {
                            Phase::Stopped
                        } else {
                            Phase::Quarantined
                        }
                    );
                    let before = bus.writes.len();
                    assert!(!audio.publish(&mut bus, token, 0, 1));
                    assert_eq!(before, bus.writes.len());
                    if !confirmed {
                        assert!(!audio.configure(words, true));
                        assert!(!audio.poll_stopped(&mut bus));
                        bus.stop_confirmed = true;
                        assert!(audio.poll_stopped(&mut bus));
                        assert_eq!(audio.phase(), Phase::Stopped);
                    }
                }
            }
        }
    }
}
#[test]
fn normal_order_stale_tokens_bounds_and_unconfirmed_start_never_grant_memory_borrows() {
    let mut audio = Audio::new();
    assert!(!audio.configure(1024, false));
    assert_eq!(audio.phase(), Phase::Quarantined);
    let mut audio = Audio::new();
    assert!(audio.configure(1024, true));
    let token = audio.begin(0).unwrap();
    let mut bus = Model {
        half: 0,
        step: 0,
        advance_at: u32::MAX,
        stop_confirmed: true,
        stopping: false,
        writes: vec![],
        after_stop: 0,
    };
    for word in 0..1024 {
        assert!(audio.publish(&mut bus, token, word, word as i32));
    }
    assert!(audio.finish(&mut bus, token));
    let next = audio.begin(0).unwrap();
    assert!(!audio.publish(&mut bus, token, 0, 0));
    assert!(!audio.publish(&mut bus, next, 1024, 0));
    assert!(audio.publish(&mut bus, next, 0, 17));
    assert_eq!(bus.writes.last(), Some(&(0, 17, true)));
}
