// SPDX-License-Identifier: GPL-3.0-only
//! Observed publication policy, never a Rust borrow/exclusion grant. Hardware
//! can advance immediately after every observation. The sealed production sink
//! uses raw volatile words; the model never creates a reference into DMA RAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Phase {
    Disabled,
    Idle,
    Rendering,
    Stopped,
    Quarantined,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    generation: u32,
    half: u32,
}
impl Token {
    pub fn generation(self) -> u32 {
        self.generation
    }
    pub fn half(self) -> u32 {
        self.half
    }
}
pub trait AudioIo {
    fn free_half(&mut self) -> u32;
    fn store_word(&mut self, index: u32, word: i32);
    fn request_stop(&mut self);
    /// True requires backend-confirmed bus quiescence, not elapsed time or an
    /// enable-bit readback. False retains every original allocation indefinitely.
    fn stopped(&mut self) -> bool;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Audio {
    half_words: u32,
    generation: u32,
    half: u32,
    phase: Phase,
}
impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
impl Audio {
    pub const fn new() -> Self {
        Self {
            half_words: 0,
            generation: 0,
            half: 0,
            phase: Phase::Disabled,
        }
    }
    /// Explicit backend stop even when a failed start already quarantined the
    /// generation. This requests quiescence; it does not fabricate completion.
    pub fn retire(&mut self, io: &mut impl AudioIo) {
        self.fault(io);
    }
    pub fn half_words(&self) -> u32 {
        self.half_words
    }
    pub fn abort(&mut self, io: &mut impl AudioIo) {
        if matches!(self.phase, Phase::Idle | Phase::Rendering) {
            self.fault(io);
        }
    }
    pub fn check(&mut self, io: &mut impl AudioIo, token: Token) -> bool {
        self.observe(io, token)
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn configure(&mut self, half_words: u32, backend_stopped: bool) -> bool {
        if !matches!(self.phase, Phase::Disabled | Phase::Stopped) {
            return false;
        }
        if !backend_stopped || ![512, 1024].contains(&half_words) {
            self.phase = Phase::Quarantined;
            return false;
        }
        self.half_words = half_words;
        self.phase = Phase::Idle;
        true
    }
    pub fn begin(&mut self, half: u32) -> Option<Token> {
        if self.phase != Phase::Idle || half > 1 {
            return None;
        }
        let Some(next) = self.generation.checked_add(1) else {
            self.phase = Phase::Quarantined;
            return None;
        };
        self.generation = next;
        self.half = half;
        self.phase = Phase::Rendering;
        Some(Token {
            generation: next,
            half,
        })
    }
    pub fn token(&self) -> Option<Token> {
        if self.phase == Phase::Rendering {
            Some(Token {
                generation: self.generation,
                half: self.half,
            })
        } else {
            None
        }
    }
    fn valid(&self, token: Token) -> bool {
        self.phase == Phase::Rendering
            && token.generation == self.generation
            && token.half == self.half
    }
    fn fault(&mut self, io: &mut impl AudioIo) {
        io.request_stop();
        self.phase = if io.stopped() {
            Phase::Stopped
        } else {
            Phase::Quarantined
        };
    }
    fn observe(&mut self, io: &mut impl AudioIo, token: Token) -> bool {
        if !self.valid(token) {
            return false;
        }
        if io.free_half() != self.half {
            self.fault(io);
            return false;
        }
        true
    }
    pub fn publish(&mut self, io: &mut impl AudioIo, token: Token, word: u32, value: i32) -> bool {
        if word >= self.half_words || !self.observe(io, token) {
            return false;
        }
        io.store_word(token.half * self.half_words + word, value);
        self.observe(io, token)
    }
    pub fn finish(&mut self, io: &mut impl AudioIo, token: Token) -> bool {
        if !self.observe(io, token) {
            return false;
        }
        self.phase = Phase::Idle;
        true
    }
    pub fn poll_stopped(&mut self, io: &mut impl AudioIo) -> bool {
        match self.phase {
            Phase::Stopped | Phase::Disabled => true,
            Phase::Quarantined => {
                if io.stopped() {
                    self.phase = Phase::Stopped;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Bus {
        stop: bool,
    }
    impl AudioIo for Bus {
        fn free_half(&mut self) -> u32 {
            0
        }
        fn store_word(&mut self, _: u32, _: i32) {
            panic!("retired generation stored output")
        }
        fn request_stop(&mut self) {
            self.stop = true;
        }
        fn stopped(&mut self) -> bool {
            false
        }
    }
    #[test]
    fn exhausted_generation_never_wraps_or_skips_the_backend_stop() {
        let mut audio = Audio::new();
        assert!(audio.configure(512, true));
        audio.generation = u32::MAX;
        assert_eq!(audio.begin(0), None);
        let mut io = Bus { stop: false };
        audio.retire(&mut io);
        assert!(io.stop);
        assert_eq!(audio.phase(), Phase::Quarantined);
        assert!(!audio.configure(512, true));
    }
}
