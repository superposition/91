// SPDX-License-Identifier: GPL-3.0-only
#[derive(Default)]
pub struct Clock {
    pub now: u32,
    pub audio_due: f64,
    pub ui_due: u32,
    pub renders: u64,
}
impl Clock {
    pub fn tick(&mut self) {
        self.now = self.now.wrapping_add(1);
    }
    pub fn audio_ready(&self) -> bool {
        self.audio_due <= self.now as f64
    }
    pub fn rendered(&mut self) {
        self.audio_due += 256.0 * 1000.0 / 44100.0;
        self.renders += 1;
    }
    pub fn frame_ready(&mut self) -> bool {
        if self.now >= self.ui_due {
            self.ui_due = self.now.wrapping_add(16);
            true
        } else {
            false
        }
    }
}
#[test]
fn exact_audio_ui_cadence_and_restart_retention() {
    let mut c = Clock::default();
    let mut frames = 0;
    for _ in 0..1000 {
        c.tick();
        while c.audio_ready() {
            c.rendered();
        }
        frames += c.frame_ready() as u32;
    }
    assert_eq!((c.now, c.renders, frames), (1000, 173, 63));
    assert_eq!(c.ui_due, 1009);
    let before = c.audio_due;
    c.tick();
    assert_eq!(c.audio_due, before);
}
