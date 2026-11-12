// SPDX-License-Identifier: GPL-3.0-only
//! Synchronous A/B NOR policy. The caller owns the I/O and one 3,840-byte buffer.
//! Header bytes and the legacy numeric sequence comparison are deliberately fixed.
use fm1_formats::crc32;
pub const SECTOR: u32 = 4096;
pub const PAYLOAD_OFFSET: u32 = 256;
pub const MAX: usize = 3840;
pub const MAGIC: u32 = 0x3058_3058;
pub const HEADER_LEN: usize = 32;
/// 1 means lock before attempting a restore. A failed restore remains locked.
pub const fn access(available: bool, locked: bool, restore: bool) -> i32 {
    if !available {
        -9
    } else if restore {
        1
    } else if locked {
        -8
    } else {
        0
    }
}

/// Hooks must complete synchronously, must not retain slices, and must not
/// re-enter this store. Program receives RAM, at most one 256-byte page.
pub trait Nor {
    fn read(&mut self, offset: u32, output: &mut [u8]) -> i32;
    fn erase(&mut self, offset: u32) -> i32;
    fn program(&mut self, offset: u32, input: &[u8]) -> i32;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub kind: u16,
    pub slot: u16,
    pub sequence: u32,
    pub length: u32,
    pub checksum: u32,
    pub reserved: [u32; 2],
}
impl Header {
    pub fn decode(object: u32, bytes: &[u8; HEADER_LEN]) -> Option<Self> {
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let kind = u16::from_le_bytes([bytes[4], bytes[5]]);
        if word(0) != MAGIC
            || u32::from(kind) != object
            || word(12) > MAX as u32
            || word(28) != crc32(&bytes[..28])
        {
            return None;
        }
        Some(Self {
            kind,
            slot: u16::from_le_bytes([bytes[6], bytes[7]]),
            sequence: word(8),
            length: word(12),
            checksum: word(16),
            reserved: [word(20), word(24)],
        })
    }
    pub fn encode(self) -> [u8; HEADER_LEN] {
        let mut out = [0; HEADER_LEN];
        out[..4].copy_from_slice(&MAGIC.to_le_bytes());
        out[4..6].copy_from_slice(&self.kind.to_le_bytes());
        out[6..8].copy_from_slice(&self.slot.to_le_bytes());
        for (i, value) in [
            self.sequence,
            self.length,
            self.checksum,
            self.reserved[0],
            self.reserved[1],
        ]
        .iter()
        .enumerate()
        {
            out[8 + i * 4..12 + i * 4].copy_from_slice(&value.to_le_bytes());
        }
        let checksum = crc32(&out[..28]);
        out[28..].copy_from_slice(&checksum.to_le_bytes());
        out
    }
}
/// Keep the original u32 address arithmetic, including out-of-contract IDs.
/// The platform is responsible for restricting physical access to real objects.
pub const fn sector(object: u32, copy: u32) -> u32 {
    let base: u32 = match object {
        26..=30 => 0xc5000 + (object - 26) * 8192,
        25 => 0xc3000,
        24 => 0xc1000,
        23 => 0xbf000,
        1 => 0xdc000,
        2 => 0xde000,
        3 => 0x97000,
        4 => 0x99000,
        5.. => 0x9b000u32.wrapping_add(object.wrapping_sub(5).wrapping_mul(8192)),
        _ => 0xfc000,
    };
    base.wrapping_add(copy.wrapping_mul(SECTOR))
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub loads: u32,
    pub saves: u32,
    pub selected_a: u32,
    pub selected_b: u32,
    pub missing: u32,
    pub programs: u32,
    pub verify_failed: u32,
    pub bytes: u32,
}
pub struct Store<'a, N> {
    pub io: N,
    pub scratch: &'a mut [u8; MAX],
    pub counts: Counts,
}
impl<'a, N: Nor> Store<'a, N> {
    pub fn new(io: N, scratch: &'a mut [u8; MAX]) -> Self {
        Self {
            io,
            scratch,
            counts: Counts::default(),
        }
    }
    fn head(&mut self, object: u32, copy: u32) -> Option<Header> {
        let mut bytes = [0; HEADER_LEN];
        if self.io.read(sector(object, copy), &mut bytes) != 0 {
            return None;
        }
        Header::decode(object, &bytes)
    }
    fn body(&mut self, object: u32, copy: u32, header: Header) -> bool {
        let bytes = &mut self.scratch[..header.length as usize];
        self.io
            .read(sector(object, copy).wrapping_add(PAYLOAD_OFFSET), bytes)
            == 0
            && crc32(bytes) == header.checksum
    }
    pub fn current(&mut self, object: u32) -> Option<(u32, Header)> {
        let a = self.head(object, 0);
        let mut b = self.head(object, 1);
        if let Some(h) = b {
            if a.is_none_or(|a| h.sequence > a.sequence) {
                if self.body(object, 1, h) {
                    self.counts.selected_b = self.counts.selected_b.wrapping_add(1);
                    return Some((1, h));
                }
                b = None;
            }
        }
        if let Some(h) = a {
            if self.body(object, 0, h) {
                self.counts.selected_a = self.counts.selected_a.wrapping_add(1);
                return Some((0, h));
            }
        }
        if let Some(h) = b {
            if self.body(object, 1, h) {
                self.counts.selected_b = self.counts.selected_b.wrapping_add(1);
                return Some((1, h));
            }
        }
        self.counts.missing = self.counts.missing.wrapping_add(1);
        None
    }
    pub fn load(&mut self, object: u32, output: &mut [u8]) -> i32 {
        self.counts.loads = self.counts.loads.wrapping_add(1);
        let Some((_, header)) = self.current(object) else {
            return -1;
        };
        let n = (header.length as usize).min(output.len());
        output[..n].copy_from_slice(&self.scratch[..n]);
        self.counts.bytes = self.counts.bytes.wrapping_add(n as u32);
        n as i32
    }
    pub fn save(&mut self, object: u32, input: &[u8]) -> i32 {
        self.counts.saves = self.counts.saves.wrapping_add(1);
        if input.len() > MAX {
            return -1;
        }
        let current = self.current(object);
        let target = u32::from(current.is_some_and(|(copy, _)| copy == 0));
        let sequence = current
            .map_or(0, |(_, header)| header.sequence)
            .wrapping_add(1);
        let base = sector(object, target);
        self.scratch[..input.len()].copy_from_slice(input);
        let rc = self.io.erase(base);
        if rc != 0 {
            return rc;
        }
        for (page, bytes) in self.scratch[..input.len()].chunks(256).enumerate() {
            self.counts.programs = self.counts.programs.wrapping_add(1);
            let rc = self.io.program(
                base.wrapping_add(PAYLOAD_OFFSET)
                    .wrapping_add(page as u32 * 256),
                bytes,
            );
            if rc != 0 {
                return rc;
            }
        }
        let header = Header {
            kind: object as u16,
            slot: target as u16,
            sequence,
            length: input.len() as u32,
            checksum: crc32(&self.scratch[..input.len()]),
            reserved: [u32::MAX; 2],
        };
        self.counts.programs = self.counts.programs.wrapping_add(1);
        let rc = self.io.program(base, &header.encode());
        if rc != 0 {
            return rc;
        }
        if !self
            .head(object, target)
            .is_some_and(|h| h.sequence == sequence && self.body(object, target, h))
        {
            self.counts.verify_failed = self.counts.verify_failed.wrapping_add(1);
            return -7;
        }
        self.counts.bytes = self.counts.bytes.wrapping_add(input.len() as u32);
        0
    }
}
