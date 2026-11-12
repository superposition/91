// SPDX-License-Identifier: GPL-3.0-only
//! Native/browser platform metadata. Payload buffers remain with their host.
pub const OBJECTS: usize = 31;
pub const STORE_MAX: usize = fm1_types::STORE_MAX;
pub const PIXELS: usize = 240 * 240;

pub struct Controls {
    pub now: u32,
    pub buttons: u32,
    pub keys: u32,
    pub master: u32,
    pub enc: [i32; 7],
    pub battery: u32,
    pub usb: i32,
    pub seamless: i32,
    pub seed: u32,
    pub lit_buttons: u32,
    pub lit_keys: u32,
    pub keys_dim: u32,
    pub dim_keys: u32,
    pub glow: i32,
    pub midi: [u32; 256],
    pub read: u32,
    pub write: u32,
    pub midi_count: u32,
    pub clocks: u32,
    pub cc: [u32; 128],
    pub last_out: u32,
    pub last_velocity: i32,
    pub host_want: i32,
    pub host_message: i32,
    pub restore_valid: i32,
    pub blits: u32,
}
impl Controls {
    pub const fn new() -> Self {
        Self {
            now: 0,
            buttons: 0,
            keys: 0,
            master: 2048,
            enc: [0; 7],
            battery: u32::MAX,
            usb: 0,
            seamless: 0,
            seed: 0,
            lit_buttons: 0,
            lit_keys: 0,
            keys_dim: 0,
            dim_keys: 0,
            glow: 1,
            midi: [0; 256],
            read: 0,
            write: 0,
            midi_count: 0,
            clocks: 0,
            cc: [0; 128],
            last_out: 0,
            last_velocity: 0,
            host_want: 0,
            host_message: 0,
            restore_valid: 0,
            blits: 0,
        }
    }
    pub fn take_encoder(&mut self, role: i32) -> i32 {
        self.enc
            .get_mut(role as usize)
            .map(core::mem::take)
            .unwrap_or(0)
    }
    /// Reject an unrepresentable foreign detent burst before changing state.
    pub fn add_encoder(&mut self, role: i32, detents: i32) -> bool {
        let Some(slot) = self.enc.get_mut(role as usize) else {
            return false;
        };
        let Some(value) = slot.checked_add(detents) else {
            return false;
        };
        *slot = value;
        true
    }
    pub fn enqueue(&mut self, packet: u32) {
        self.midi[self.write as usize % 256] = packet;
        self.write = self.write.wrapping_add(1);
    }
    pub fn dequeue(&mut self) -> Option<u32> {
        if self.read == self.write {
            None
        } else {
            let p = self.midi[self.read as usize % 256];
            self.read = self.read.wrapping_add(1);
            Some(p)
        }
    }
    pub fn midi_out(&mut self, packet: u32) -> bool {
        self.midi_count = self.midi_count.wrapping_add(1);
        let status = (packet >> 8) & 255;
        let data1 = (packet >> 16) & 255;
        let data2 = packet >> 24;
        if status & 240 == 0x90 && data2 != 0 {
            self.last_velocity = data2 as i32;
        }
        if status & 240 == 0xb0 {
            self.cc[(data1 & 127) as usize] = self.cc[(data1 & 127) as usize].wrapping_add(1);
        }
        if status == 0xf8 {
            self.clocks = self.clocks.wrapping_add(1);
            false
        } else {
            self.last_out = status << 16 | data1 << 8 | data2;
            true
        }
    }
    pub fn leds(&mut self, buttons: u32, keys: u32) {
        self.lit_buttons = buttons;
        self.lit_keys = keys;
        self.dim_keys = self.keys_dim & !keys;
        self.keys_dim = 0;
    }
}
impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StoreMeta {
    pub lengths: [i32; OBJECTS],
    pub writes: u32,
    pub locked: bool,
}
impl StoreMeta {
    pub const fn new() -> Self {
        let mut lengths = [0; OBJECTS];
        let mut i = 0;
        while i < 4 {
            lengths[i] = -1;
            i += 1;
        }
        Self {
            lengths,
            writes: 0,
            locked: false,
        }
    }
    pub fn set_length(&mut self, object: u32, length: i32) -> bool {
        let Some(slot) = self.lengths.get_mut(object as usize) else {
            return false;
        };
        *slot = length.clamp(-1, STORE_MAX as i32);
        true
    }
    pub fn load_length(&self, object: u32, capacity: usize) -> Result<usize, i32> {
        let Some(&length) = self.lengths.get(object as usize) else {
            return Err(-1);
        };
        if length < 0 || length as usize > STORE_MAX {
            return Err(-1);
        }
        Ok((length as usize).min(capacity))
    }
    pub fn writable(&self, object: u32, length: usize, restore: bool) -> Result<(), i32> {
        if object as usize >= OBJECTS || length > STORE_MAX {
            return Err(-1);
        }
        if self.locked && !restore {
            return Err(-8);
        }
        Ok(())
    }
    /// Call only after a validated payload copy, without an intervening callback.
    pub fn committed(&mut self, object: u32, length: usize, restore: bool) {
        self.lengths[object as usize] = length as i32;
        self.locked |= restore;
        self.writes = self.writes.wrapping_add(1);
    }
}
impl Default for StoreMeta {
    fn default() -> Self {
        Self::new()
    }
}

pub fn fill(frame: &mut [u16; PIXELS], x: u32, y: u32, w: u32, h: u32, color: u16) {
    for row in y..y.wrapping_add(h).min(240) {
        for col in x..x.wrapping_add(w).min(240) {
            frame[(row * 240 + col) as usize] = color.swap_bytes();
        }
    }
}
pub fn blit(frame: &mut [u16; PIXELS], x: u32, y: u32, w: u32, h: u32, pixels: &[u16]) -> bool {
    let Some(length) = (w as usize).checked_mul(h as usize) else {
        return false;
    };
    if length > pixels.len() {
        return false;
    }
    for row in 0..h {
        for col in 0..w {
            let dx = x.wrapping_add(col);
            let dy = y.wrapping_add(row);
            if dx < 240 && dy < 240 {
                frame[(dy * 240 + dx) as usize] = pixels[(row * w + col) as usize];
            }
        }
    }
    true
}

#[derive(Default)]
pub struct Schedule {
    pub audio_due: f64,
    pub ui_due: u32,
}
impl Schedule {
    pub const fn new() -> Self {
        Self {
            audio_due: 0.0,
            ui_due: 0,
        }
    }
    pub fn audio_ready(&self, now: u32) -> bool {
        self.audio_due <= now as f64
    }
    pub fn rendered(&mut self) {
        self.audio_due += 256.0 * 1000.0 / 44100.0;
    }
    pub fn frame_ready(&mut self, now: u32) -> bool {
        if now >= self.ui_due {
            self.ui_due = now.wrapping_add(16);
            true
        } else {
            false
        }
    }
}
#[derive(Default)]
pub struct Clock {
    pub now: u32,
    pub schedule: Schedule,
    pub renders: u64,
}
impl core::ops::Deref for Clock {
    type Target = Schedule;
    fn deref(&self) -> &Schedule {
        &self.schedule
    }
}
impl core::ops::DerefMut for Clock {
    fn deref_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }
}
impl Clock {
    pub fn tick(&mut self) {
        self.now = self.now.wrapping_add(1);
    }
    pub fn audio_ready(&self) -> bool {
        self.schedule.audio_ready(self.now)
    }
    pub fn rendered(&mut self) {
        self.schedule.rendered();
        self.renders += 1;
    }
    pub fn frame_ready(&mut self) -> bool {
        self.schedule.frame_ready(self.now)
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

#[test]
fn foreign_encoder_bursts_reject_overflow_before_mutation() {
    let mut controls = Controls::new();
    assert!(controls.add_encoder(0, i32::MAX));
    assert!(!controls.add_encoder(0, 1));
    assert!(!controls.add_encoder(-1, 2));
    assert!(!controls.add_encoder(7, 2));
    assert_eq!(controls.take_encoder(0), i32::MAX);
    assert_eq!(controls.take_encoder(0), 0);
    assert!(controls.add_encoder(6, i32::MIN));
    assert!(!controls.add_encoder(6, -1));
    assert_eq!(controls.take_encoder(6), i32::MIN);
    assert_eq!(controls.take_encoder(7), 0);
}
