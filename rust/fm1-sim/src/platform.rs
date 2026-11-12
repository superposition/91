// SPDX-License-Identifier: GPL-3.0-only
pub use fm1_app::host_platform::{Controls, StoreMeta, OBJECTS, STORE_MAX};
pub struct Store {
    pub bytes: Box<[[u8; STORE_MAX]; OBJECTS]>,
    pub meta: StoreMeta,
}
impl core::ops::Deref for Store {
    type Target = StoreMeta;
    fn deref(&self) -> &StoreMeta {
        &self.meta
    }
}
impl core::ops::DerefMut for Store {
    fn deref_mut(&mut self) -> &mut StoreMeta {
        &mut self.meta
    }
}
impl Default for Store {
    fn default() -> Self {
        Self {
            bytes: Box::new([[0; STORE_MAX]; OBJECTS]),
            meta: StoreMeta::new(),
        }
    }
}
impl Store {
    pub fn load(&self, object: u32, dst: &mut [u8]) -> i32 {
        let n = match self.load_length(object, dst.len()) {
            Ok(n) => n,
            Err(error) => return error,
        };
        dst[..n].copy_from_slice(&self.bytes[object as usize][..n]);
        n as i32
    }
    pub fn save(&mut self, object: u32, src: &[u8], restore: bool) -> i32 {
        if let Err(error) = self.writable(object, src.len(), restore) {
            return error;
        }
        self.bytes[object as usize][..src.len()].copy_from_slice(src);
        self.committed(object, src.len(), restore);
        0
    }
}
pub struct Platform {
    pub controls: Controls,
    pub framebuffer: Box<[u16; 57600]>,
    pub store: Store,
}
impl core::ops::Deref for Platform {
    type Target = Controls;
    fn deref(&self) -> &Controls {
        &self.controls
    }
}
impl core::ops::DerefMut for Platform {
    fn deref_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }
}
impl Default for Platform {
    fn default() -> Self {
        Self {
            controls: Controls::new(),
            framebuffer: Box::new([0; 57600]),
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

    pub fn fill(&mut self, x: u32, y: u32, w: u32, h: u32, color: u16) {
        fm1_app::host_platform::fill(self.framebuffer.as_mut(), x, y, w, h, color);
    }
    pub fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, pixels: &[u16]) {
        self.blits = self.blits.wrapping_add(1);
        let _ = fm1_app::host_platform::blit(self.framebuffer.as_mut(), x, y, w, h, pixels);
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
