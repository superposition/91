// SPDX-License-Identifier: GPL-3.0-only
//! TRS MIDI policy. DMA slots remain external raw I/O, never a borrowed slice.
pub const TRS_RING: u32 = 128;
pub const TRS_EMPTY: u8 = 0xfd;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trs {
    pub rd: u32,
    pub bytes: u32,
    pub drops: u32,
    pub messages: u32,
    pub status: u8,
    pub need: u8,
    pub got: u8,
    pub first: u8,
    pub sysex: u8,
}
pub trait TrsIo {
    fn read(&mut self, index: u32) -> u8;
    fn write(&mut self, index: u32, value: u8);
    /// Synchronous admission into the separate existing 64-event MIDI queue.
    fn send(&mut self, packet: u32) -> bool;
}
pub fn clear(io: &mut impl TrsIo) {
    for index in 0..TRS_RING {
        io.write(index, TRS_EMPTY);
    }
}
impl Trs {
    fn emit(&mut self, packet: u32, send: &mut impl FnMut(u32) -> bool) {
        if send(packet) {
            self.messages = self.messages.wrapping_add(1);
        } else {
            self.drops = self.drops.wrapping_add(1);
        }
    }
    pub fn byte(&mut self, byte: u32, send: &mut impl FnMut(u32) -> bool) {
        if byte >= 0xf8 {
            if matches!(byte, 0xf8 | 0xfa | 0xfb | 0xfc) {
                self.emit(0xf | byte << 8, send);
            }
            return;
        }
        if byte & 0x80 != 0 {
            self.sysex = u8::from(byte == 0xf0);
            self.status = if byte < 0xf0 { byte as u8 } else { 0 };
            self.need = if byte & 0xe0 == 0xc0 { 1 } else { 2 };
            self.got = 0;
            return;
        }
        if self.sysex != 0 || self.status == 0 {
            return;
        }
        if self.need == 2 && self.got == 0 {
            self.first = byte as u8;
            self.got = 1;
            return;
        }
        let first = if self.need == 2 {
            u32::from(self.first)
        } else {
            byte
        };
        let second = if self.need == 2 { byte } else { 0 };
        self.got = 0;
        self.emit(
            u32::from(self.status >> 4) | u32::from(self.status) << 8 | first << 16 | second << 24,
            send,
        );
    }
    /// Content determines available bytes; the DMA count only detects laps.
    /// An overrun intentionally keeps `need` and `first`, matching existing C.
    pub fn take(&mut self, count: u32, io: &mut impl TrsIo) -> bool {
        if self.rd >= TRS_RING {
            return false;
        }
        if count > TRS_RING {
            let lost = count - TRS_RING;
            self.drops = self.drops.wrapping_add(lost);
            self.rd = self.rd.wrapping_add(lost) & (TRS_RING - 1);
            self.status = 0;
            self.got = 0;
            self.sysex = 0;
        }
        for _ in 0..TRS_RING {
            let byte = io.read(self.rd);
            if byte == TRS_EMPTY && io.read((self.rd + 1) & (TRS_RING - 1)) == TRS_EMPTY {
                break;
            }
            io.write(self.rd, TRS_EMPTY);
            self.rd = (self.rd + 1) & (TRS_RING - 1);
            self.bytes = self.bytes.wrapping_add(1);
            self.byte(u32::from(byte), &mut |packet| io.send(packet));
        }
        true
    }
}
