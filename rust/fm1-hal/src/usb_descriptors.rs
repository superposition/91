// SPDX-License-Identifier: GPL-3.0-only
//! The original device descriptor bytes, streamed without an expanded cache.
const DEVICE: [u8; 18] = [18, 1, 0x10, 1, 0, 0, 0, 64, 9, 0x12, 1, 0, 0, 3, 1, 2, 0, 1];
const AUDIO_IAD: [u8; 8] = [8, 0x0b, 0, 2, 1, 1, 0, 0];
const AUDIO_CONTROL: [u8; 9] = [9, 4, 0, 0, 0, 1, 1, 0, 0];
const AC_MIDI: [u8; 9] = [9, 0x24, 1, 0, 1, 9, 0, 1, 1];
const AC_UAC: [u8; 31] = [
    10, 0x24, 1, 0, 1, 31, 0, 2, 1, 2, 12, 0x24, 2, 1, 3, 6, 0, 2, 3, 0, 0, 0, 9, 0x24, 3, 2, 1, 1,
    0, 1, 0,
];
const MIDI: [u8; 74] = [
    9, 4, 1, 0, 2, 1, 3, 0, 0, 7, 0x24, 1, 0, 1, 0x41, 0, 6, 0x24, 2, 1, 1, 0, 6, 0x24, 2, 2, 2, 0,
    9, 0x24, 3, 1, 3, 1, 2, 1, 0, 9, 0x24, 3, 2, 4, 1, 1, 1, 0, 9, 5, 1, 2, 64, 0, 0, 0, 0, 5,
    0x25, 1, 1, 1, 9, 5, 0x81, 2, 64, 0, 0, 0, 0, 5, 0x25, 1, 1, 3,
];
const UAC: [u8; 52] = [
    9, 4, 2, 0, 0, 1, 2, 0, 0, 9, 4, 2, 1, 1, 1, 2, 0, 0, 7, 0x24, 1, 2, 1, 1, 0, 11, 0x24, 2, 1,
    2, 2, 16, 1, 0x44, 0xac, 0, 9, 5, 0x84, 5, 184, 0, 1, 0, 0, 7, 0x25, 1, 1, 0, 0, 0,
];
const CDC: [u8; 66] = [
    8, 0x0b, 2, 2, 2, 2, 1, 0, 9, 4, 2, 0, 1, 2, 2, 1, 0, 5, 0x24, 0, 0x10, 1, 5, 0x24, 1, 0, 3, 4,
    0x24, 2, 2, 5, 0x24, 6, 2, 3, 7, 5, 0x82, 3, 8, 0, 16, 9, 4, 3, 0, 2, 0x0a, 0, 0, 0, 7, 5, 3,
    2, 64, 0, 0, 7, 5, 0x83, 2, 64, 0, 0,
];
const LANGUAGE: [u8; 4] = [4, 3, 9, 4];
const MANUFACTURER: &[u8] = b"\x1a\x03R\0e\0d\0t\0a\0i\0l\0W\0o\0r\0k\0s\0";
const LOADER_MANUFACTURER: &[u8] =
    b"\x2a\x03H\0\xfc\0g\0e\0l\0t\0o\0n\0 \0I\0n\0s\0t\0r\0u\0m\0e\0n\0t\0s\0";
const PRODUCT: &[u8] = b"\x16\x03H\0Y\0P\0-\x001\0 \0F\0M\0-\x001\0";
const LOADER_PRODUCT: &[u8] = b"\x1e\x03F\0e\0l\0u\0c\0c\0a\0 \0U\0p\0d\0a\0t\0e\0";
#[derive(Clone, Copy)]
pub struct Descriptors {
    flags: u32,
    pid: u16,
}
impl Descriptors {
    /// bit0 CDC, bit1 UAC, bit2 the recovery loader. PID is ignored by CDC, as C.
    pub const fn new(flags: u32, pid: u16) -> Self {
        Self { flags, pid }
    }
    fn cdc(self) -> bool {
        self.flags & 1 != 0
    }
    fn uac(self) -> bool {
        self.flags & 2 != 0
    }
    fn loader(self) -> bool {
        self.flags & 4 != 0
    }
    pub fn config_length(self) -> usize {
        101 + 74 * usize::from(self.cdc()) + 74 * usize::from(self.uac())
    }
    fn string(self, index: u32) -> Option<&'static [u8]> {
        match index {
            0 => Some(&LANGUAGE),
            1 => Some(if self.loader() {
                LOADER_MANUFACTURER
            } else {
                MANUFACTURER
            }),
            2 => Some(if self.loader() {
                LOADER_PRODUCT
            } else {
                PRODUCT
            }),
            _ => None,
        }
    }
    pub fn length(self, request: u32) -> Option<usize> {
        match request >> 8 {
            1 => Some(18),
            2 => Some(self.config_length()),
            3 => self.string(request & 255).map(<[u8]>::len),
            _ => None,
        }
    }
    pub fn byte(self, request: u32, index: usize) -> Option<u8> {
        match request >> 8 {
            1 => {
                let mut byte = *DEVICE.get(index)?;
                if self.cdc() {
                    byte = match index {
                        2 => 0,
                        3 => 2,
                        4 => 0xef,
                        5 => 2,
                        6 => 1,
                        10 => 1,
                        11 => 0,
                        _ => byte,
                    };
                } else if index == 10 {
                    byte = self.pid as u8;
                } else if index == 11 {
                    byte = (self.pid >> 8) as u8;
                }
                if index == 12 {
                    byte = u8::from(self.cdc()) + 0x10 * u8::from(self.uac());
                }
                Some(byte)
            }
            2 => self.config_byte(index),
            3 => self.string(request & 255)?.get(index).copied(),
            _ => None,
        }
    }
    fn config_byte(self, mut index: usize) -> Option<u8> {
        if index < 9 {
            let length = self.config_length();
            return Some(match index {
                0 => 9,
                1 => 2,
                2 => length as u8,
                3 => (length >> 8) as u8,
                4 => 2 + u8::from(self.uac()) + 2 * u8::from(self.cdc()),
                5 => 1,
                6 => 0,
                7 => 0x80,
                _ => 50,
            });
        }
        index -= 9;
        if self.cdc() {
            if index < 8 {
                return Some(AUDIO_IAD[index] + if index == 3 { u8::from(self.uac()) } else { 0 });
            }
            index -= 8;
        }
        fn part(index: &mut usize, bytes: &[u8]) -> Option<u8> {
            if *index < bytes.len() {
                Some(bytes[*index])
            } else {
                *index -= bytes.len();
                None
            }
        }
        if let Some(byte) = part(&mut index, &AUDIO_CONTROL) {
            return Some(byte);
        }
        if let Some(byte) = part(&mut index, if self.uac() { &AC_UAC } else { &AC_MIDI }) {
            return Some(byte);
        }
        if let Some(byte) = part(&mut index, &MIDI) {
            return Some(byte);
        }
        if self.uac() {
            if let Some(byte) = part(&mut index, &UAC) {
                return Some(byte);
            }
        }
        if self.cdc() {
            let byte = *CDC.get(index)?;
            return Some(
                byte + if matches!(index, 2 | 10 | 26 | 34 | 35 | 45) {
                    u8::from(self.uac())
                } else {
                    0
                },
            );
        }
        None
    }
    /// No output is touched when request/offset is invalid. The caller owns its
    /// existing raw endpoint buffer; this visits only the bounded selected prefix.
    pub fn copy(
        self,
        request: u32,
        offset: usize,
        count: usize,
        mut write: impl FnMut(usize, u8),
    ) -> Option<usize> {
        let length = self.length(request)?;
        if offset > length {
            return None;
        }
        let count = count.min(length - offset);
        for index in 0..count {
            write(index, self.byte(request, offset + index)?);
        }
        Some(count)
    }
}
