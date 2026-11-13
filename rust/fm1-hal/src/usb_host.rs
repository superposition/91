// SPDX-License-Identifier: GPL-3.0-only
//! Bounded USB-MIDI host descriptor and packet policy; no controller access.
pub const CONFIG_BYTES: usize = 768;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Midi {
    pub found: u8,
    pub cfg: u8,
    pub iface: u8,
    pub alt: u8,
    pub in_ep: u8,
    pub in_type: u8,
    pub in_interval: u8,
    pub reserved0: u8,
    pub in_mps: u16,
    pub out_ep: u8,
    pub out_type: u8,
    pub out_interval: u8,
    pub reserved1: u8,
    pub out_mps: u16,
}
const _: [(); 16] = [(); core::mem::size_of::<Midi>()];
const _: [(); 8] = [(); core::mem::offset_of!(Midi, in_mps)];
const _: [(); 14] = [(); core::mem::offset_of!(Midi, out_mps)];
/// Select the first streaming alternate with a usable endpoint. Incomplete or
/// malformed descriptors stop the search; endpoints never span alternates.
pub fn parse(bytes: &[u8]) -> Midi {
    let mut result = Midi::default();
    let (mut index, mut config, mut streaming) = (0, 0, false);
    while bytes.len() - index >= 2 {
        let length = usize::from(bytes[index]);
        if length < 2 || length > bytes.len() - index {
            break;
        }
        let d = &bytes[index..index + length];
        if d[1] == 2 && length >= 9 {
            config = d[5];
        } else if d[1] == 4 && length >= 9 {
            if result.found != 0 {
                break;
            }
            result.cfg = config;
            result.iface = d[2];
            result.alt = d[3];
            streaming = d[5] == 1 && d[6] == 3 && d[7] == 0;
        } else if d[1] == 5 && length >= 7 && streaming {
            let address = d[2];
            let kind = d[3] & 3;
            let size = u16::from_le_bytes([d[4], d[5]]);
            if address & 15 != 0
                && address & 0x70 == 0
                && (4..=64).contains(&size)
                && (kind == 2 && matches!(size, 8 | 16 | 32 | 64) || kind == 3 && d[6] != 0)
            {
                if address & 0x80 != 0 && result.in_ep == 0 {
                    result.found = 1;
                    result.in_ep = address & 15;
                    result.in_type = kind;
                    result.in_interval = d[6];
                    result.in_mps = size;
                } else if address & 0x80 == 0 && result.out_ep == 0 {
                    result.found = 1;
                    result.out_ep = address & 15;
                    result.out_type = kind;
                    result.out_interval = d[6];
                    result.out_mps = size & !3;
                }
            }
        }
        index += length;
    }
    result
}
/// Preserve the legacy cable/CIN/status filter, including untouched trailing
/// bytes for realtime and two-byte channel messages. SysEx is not admitted.
pub fn packet(word: u32) -> u32 {
    let cin = word & 15;
    let status = (word >> 8) & 255;
    let clean = word & !0xf0;
    if cin == 15 {
        return if matches!(status, 0xf8 | 0xfa | 0xfb | 0xfc) {
            clean
        } else {
            0
        };
    }
    if (8..=14).contains(&cin)
        && status >> 4 == cin
        && word & 0x0080_0000 == 0
        && (matches!(cin, 12 | 13) || word & 0x8000_0000 == 0)
    {
        clean
    } else {
        0
    }
}
pub trait Receive {
    fn write(&self) -> u32;
    fn read(&self) -> u32;
    fn byte(&self, index: usize) -> u8;
    fn put(&mut self, index: usize, value: u32);
    fn publish(&mut self);
    fn set_write(&mut self, value: u32);
}
/// A known-ready host RX packet stays retained until all potential sixteen
/// events fit. Only accepted messages consume slots; a partial final word does
/// not. Publication precedes each write-index advance, in original order.
pub fn receive(io: &mut impl Receive, count: usize) -> Option<u32> {
    let used = io.write().wrapping_sub(io.read());
    if count > 64 || used > 64 || 64 - used < 16 {
        return None;
    }
    let mut accepted = 0;
    for offset in (0..count / 4).map(|i| i * 4) {
        let word = u32::from_le_bytes([
            io.byte(offset),
            io.byte(offset + 1),
            io.byte(offset + 2),
            io.byte(offset + 3),
        ]);
        let value = packet(word);
        if value != 0 {
            let write = io.write();
            io.put((write % 64) as usize, value);
            io.publish();
            io.set_write(write.wrapping_add(1));
            accepted += 1;
        }
    }
    Some(accepted)
}
