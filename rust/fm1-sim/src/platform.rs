// SPDX-License-Identifier: GPL-3.0-only
pub const OBJECTS: usize = 31;
pub const STORE_MAX: usize = 3840;
pub struct Store {
    pub bytes: Box<[[u8; STORE_MAX]; OBJECTS]>,
    pub lengths: [i32; OBJECTS],
    pub writes: u32,
    pub locked: bool,
}
impl Default for Store {
    fn default() -> Self {
        let mut lengths = [0; OBJECTS];
        lengths[..4].fill(-1);
        Self {
            bytes: Box::new([[0; STORE_MAX]; OBJECTS]),
            lengths,
            writes: 0,
            locked: false,
        }
    }
}
impl Store {
    pub fn load(&self, object: u32, dst: &mut [u8]) -> i32 {
        let i = object as usize;
        if i >= OBJECTS || self.lengths[i] < 0 {
            return -1;
        }
        let n = (self.lengths[i] as usize).min(dst.len());
        dst[..n].copy_from_slice(&self.bytes[i][..n]);
        n as i32
    }
    pub fn save(&mut self, object: u32, src: &[u8], restore: bool) -> i32 {
        let i = object as usize;
        if i >= OBJECTS || src.len() > STORE_MAX {
            return -1;
        }
        if restore {
            self.locked = true;
        } else if self.locked {
            return -8;
        }
        self.bytes[i][..src.len()].copy_from_slice(src);
        self.lengths[i] = src.len() as i32;
        self.writes = self.writes.wrapping_add(1);
        0
    }
}
pub struct Platform {
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
    pub framebuffer: Box<[u16; 57600]>,
    pub blits: u32,
    pub store: Store,
}
impl Default for Platform {
    fn default() -> Self {
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
            framebuffer: Box::new([0; 57600]),
            blits: 0,
            store: Store::default(),
        }
    }
}
impl Platform {
    /// Stable little-endian host-state evidence, including every stored byte.
    pub fn trace(&self, audio_due: f64, ui_due: u32, peak: f32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut word = |v: u32| out.extend_from_slice(&v.to_le_bytes());
        for v in [self.now, self.buttons, self.keys, self.master] {
            word(v);
        }
        for v in self.enc {
            word(v as u32);
        }
        for v in [
            self.battery,
            self.usb as u32,
            self.seamless as u32,
            self.seed,
            self.lit_buttons,
            self.lit_keys,
            self.keys_dim,
            self.dim_keys,
            self.glow as u32,
            self.read,
            self.write,
            self.midi_count,
            self.clocks,
        ] {
            word(v);
        }
        for v in self.cc {
            word(v);
        }
        for v in [
            self.last_out,
            self.last_velocity as u32,
            self.host_want as u32,
            self.host_message as u32,
            self.restore_valid as u32,
            self.store.locked as u32,
            self.blits,
            self.store.writes,
        ] {
            word(v);
        }
        for v in self.store.lengths {
            word(v as u32);
        }
        for v in self.midi {
            word(v);
        }
        for object in self.store.bytes.iter() {
            out.extend_from_slice(object);
        }
        for pixel in self.framebuffer.iter() {
            out.extend_from_slice(&pixel.to_le_bytes());
        }
        out.extend_from_slice(&audio_due.to_le_bytes());
        out.extend_from_slice(&ui_due.to_le_bytes());
        out.extend_from_slice(&peak.to_bits().to_le_bytes());
        out
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
    pub fn fill(&mut self, x: u32, y: u32, w: u32, h: u32, color: u16) {
        for j in y..y.wrapping_add(h).min(240) {
            for i in x..x.wrapping_add(w).min(240) {
                self.framebuffer[(j * 240 + i) as usize] = color.swap_bytes();
            }
        }
    }
    pub fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, pixels: &[u16]) {
        self.blits = self.blits.wrapping_add(1);
        for j in 0..h {
            for i in 0..w {
                if x.wrapping_add(i) < 240 && y.wrapping_add(j) < 240 {
                    self.framebuffer[((y + j) * 240 + x + i) as usize] =
                        pixels[(j * w + i) as usize];
                }
            }
        }
    }
    pub fn value(&self, id: u32, index: u32) -> u32 {
        match id {
            0 => self.store.writes,
            1 => self.clocks,
            2 => self.midi_count,
            3 => self.host_want as u32,
            4 => self.restore_valid as u32,
            5 => self.store.locked as u32,
            6 => self.cc[index as usize & 127],
            7 => self.last_velocity as u32,
            8 => self.last_out,
            9 => self.lit_buttons,
            10 => self.dim_keys,
            11 => self.lit_keys,
            _ => 0,
        }
    }
}
#[test]
fn object_capacity_missing_empty_truncation_restore_and_restart() {
    let mut s = Store::default();
    let mut a = [99; 4];
    assert_eq!(s.load(0, &mut a), -1);
    assert_eq!(s.load(30, &mut a), 0);
    assert_eq!(s.save(31, &[], false), -1);
    assert_eq!(s.save(30, &[1, 2, 3, 4, 5], false), 0);
    assert_eq!(s.load(30, &mut a), 4);
    assert_eq!(a, [1, 2, 3, 4]);
    assert_eq!(s.save(4, &[7], true), 0);
    assert_eq!(s.save(1, &[], false), -8);
    assert_eq!(s.save(31, &[], true), -1);
    s.locked = false;
    assert_eq!(s.load(4, &mut a), 1);
    assert_eq!(s.writes, 2);
}
#[test]
fn midi_queue_overwrite_fifo_stats_led_consumption_and_clipping() {
    let mut p = Platform::default();
    for i in 0..257 {
        p.enqueue(i);
    }
    assert_eq!(p.dequeue(), Some(256));
    assert_eq!(p.dequeue(), Some(1));
    assert!(p.midi_out(9 | 0x90 << 8 | 60 << 16 | 100 << 24));
    assert!(!p.midi_out(15 | 0xf8 << 8));
    assert_eq!(
        (p.midi_count, p.clocks, p.last_velocity, p.last_out),
        (2, 1, 100, 0x903c64)
    );
    p.keys_dim = 7;
    p.leds(3, 2);
    assert_eq!((p.dim_keys, p.keys_dim), (5, 0));
    p.leds(0, 0);
    assert_eq!(p.dim_keys, 0);
    p.fill(239, 239, 4, 4, 0x1234);
    assert_eq!(p.framebuffer[57599], 0x3412);
    p.blit(239, 239, 2, 2, &[1, 2, 3, 4]);
    assert_eq!((p.framebuffer[57599], p.blits), (1, 1));
}
