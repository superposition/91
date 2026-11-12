// SPDX-License-Identifier: GPL-3.0-only
//! Sample phase and render cadence for the existing 44.1 kHz instrument.
//!
//! A pulse is 1/24 of a quarter note, six pulses make a musical sixteenth.
//! The caller dispatches musical events before scheduling the next pulse: those
//! events can change the tempo. A late MIDI pulse retains its absolute phase.

pub const SAMPLE_RATE: f32 = 44_100.0;
pub const CONTROL_INTERVAL: u32 = 128;

/// Exactly the four legacy scalar counters, with no allocation or pointers.
/// C may allocate/zero-initialize this transitional representation, but all
/// mutations belong to one render owner through scoped Rust calls.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Transport {
    pulse_left: f32,
    pulse: u32,
    external_age: u32,
    control_acc: u32,
}

const _: [(); 16] = [(); core::mem::size_of::<Transport>()];
const _: [(); 4] = [(); core::mem::align_of::<Transport>()];

impl Default for Transport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport {
    pub const fn new() -> Self {
        Self {
            pulse_left: 0.0,
            pulse: 0,
            external_age: 0,
            control_acc: 0,
        }
    }

    /// Start immediately. Continue preserves the six-pulse subdivision.
    /// Neither operation changes control cadence or external-clock age.
    pub fn start(&mut self, from_top: bool) {
        if from_top {
            self.pulse = 0;
        }
        self.pulse_left = 0.0;
    }

    /// Check the subdivision before dispatching a musical event.
    pub fn at_sixteenth(&self) -> bool {
        self.pulse == 0
    }

    /// Complete a received/internal pulse after musical/MIDI dispatch, before
    /// scheduling the next period. Keeping these operations separate preserves
    /// the original callback order, including any tempo change in the event.
    pub fn finish_pulse(&mut self) {
        self.pulse += 1;
        if self.pulse >= 6 {
            self.pulse = 0;
        }
    }

    pub fn receive_external(&mut self) {
        self.external_age = 0;
    }

    /// The legacy timeout is 101 render calls, deliberately not 101 chunks or
    /// a new wall-clock duration. Inactive calls do not age the clock.
    pub fn external_expired(&mut self, external: bool) -> bool {
        if !external {
            return false;
        }
        self.external_age = self.external_age.wrapping_add(1);
        self.external_age > 100
    }

    pub fn pulse_due(&self) -> bool {
        self.pulse_left <= 0.0
    }

    /// Call after the musical event, using the possibly updated tempo.
    pub fn schedule_next(&mut self, step_length: f32) {
        self.pulse_left += step_length * (1.0 / 6.0);
    }

    /// Limit a chunk at the next clock boundary. DX Songbook voices coalesce
    /// intermediate outgoing clocks but keep musical sixteenths sample-timed.
    /// Preserve the original f32 operation order and truncation-plus-one rule.
    pub fn clock_limit(&self, frames: u32, step_length: f32, coalesce: bool) -> u32 {
        let mut boundary = self.pulse_left;
        if coalesce && self.pulse != 0 {
            boundary += (6 - self.pulse) as f32 * step_length * (1.0 / 6.0);
        }
        if frames as f32 > boundary {
            // A valid render state has added one period before this call.
            // A coalesced pulse can be late; the next sixteenth remains ahead.
            (boundary as u32).saturating_add(1)
        } else {
            frames
        }
    }

    /// Advance the same sample phase through a chunk and account for the slow
    /// controls. Return their elapsed frames when due; short chunks accumulate.
    pub fn advance(&mut self, frames: u32, timed: bool) -> Option<u32> {
        if timed {
            self.pulse_left -= frames as f32;
        }
        self.control_acc = self.control_acc.wrapping_add(frames);
        if self.control_acc >= CONTROL_INTERVAL {
            let elapsed = self.control_acc;
            self.control_acc = 0;
            Some(elapsed)
        } else {
            None
        }
    }
}

/// Keep the original binary32 tempo calculation. The instrument clamps tempo
/// to 70..190 BPM before calling this function.
pub fn step_length(tempo: u16) -> f32 {
    SAMPLE_RATE * 60.0 / (tempo as f32 * 4.0)
}

/// Swung/humanized notes may cut at `minimum` frames, while clock boundaries
/// remain free to cut earlier. No waiting event means the current limit stays.
pub fn wait_limit(frames: u32, wait: i32, minimum: u32) -> u32 {
    if wait > 0 && (wait as u32) < frames {
        if wait as u32 >= minimum {
            wait as u32
        } else if frames < minimum {
            frames
        } else {
            minimum
        }
    } else {
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_tempo_matches_the_frozen_c_binary32_value() {
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../tests/transport-tempo.json")).unwrap();
        let reference = reference.as_array().unwrap();
        assert_eq!(reference.len(), 121);
        for (offset, expected) in reference.iter().enumerate() {
            assert_eq!(
                step_length(70 + offset as u16).to_bits(),
                expected.as_u64().unwrap() as u32
            );
        }
    }

    #[test]
    fn start_continue_and_external_clock_preserve_the_six_pulse_grid() {
        let mut transport = Transport::new();
        transport.start(true);
        let mut steps = [false; 13];
        for (i, step) in steps.iter_mut().enumerate() {
            *step = transport.at_sixteenth();
            transport.finish_pulse();
            if i == 2 {
                transport.start(false);
            }
        }
        assert_eq!(
            steps,
            [
                true, false, false, false, false, false, true, false, false, false, false, false,
                true
            ]
        );
        transport.start(true);
        assert!(transport.at_sixteenth());
        transport.finish_pulse();
        assert!(!transport.at_sixteenth());
    }

    #[test]
    fn timeout_counts_render_calls_and_restarts_after_any_external_message() {
        let mut transport = Transport::new();
        for _ in 0..500 {
            assert!(!transport.external_expired(false));
        }
        for _ in 0..100 {
            assert!(!transport.external_expired(true));
        }
        assert!(transport.external_expired(true));
        transport.receive_external();
        for _ in 0..100 {
            assert!(!transport.external_expired(true));
        }
        assert!(transport.external_expired(true));
        transport.external_age = u32::MAX;
        assert!(!transport.external_expired(true));
        assert_eq!(transport.external_age, 0);
    }

    #[test]
    fn short_chunks_accumulate_control_time_while_stopped_phase_stays_fixed() {
        let mut transport = Transport::new();
        transport.schedule_next(step_length(128));
        let before = transport.pulse_left.to_bits();
        assert_eq!(transport.advance(1, false), None);
        assert_eq!(transport.advance(126, false), None);
        assert_eq!(transport.advance(1, false), Some(128));
        assert_eq!(transport.pulse_left.to_bits(), before);
        assert_eq!(transport.advance(1, true), None);
        assert_eq!(transport.advance(256, true), Some(257));
        assert_eq!(transport.control_acc, 0);
        assert_eq!(transport.pulse_left, f32::from_bits(before) - 257.0);
    }

    #[test]
    fn clock_boundaries_can_cut_earlier_than_the_waiting_hit_floor() {
        let mut transport = Transport::new();
        transport.pulse_left = 42.0;
        assert_eq!(transport.clock_limit(256, step_length(128), false), 43);
        assert_eq!(wait_limit(43, 1, 128), 43);
        assert_eq!(wait_limit(256, 1, 128), 128);
        assert_eq!(wait_limit(256, 127, 128), 128);
        assert_eq!(wait_limit(256, 129, 128), 129);
        assert_eq!(wait_limit(256, 256, 128), 256);
        assert_eq!(wait_limit(256, -1, 128), 256);
        transport.pulse = 1;
        assert_eq!(transport.clock_limit(256, step_length(128), true), 256);
        transport.pulse = 0;
        assert_eq!(transport.clock_limit(256, step_length(128), true), 43);
    }
}
