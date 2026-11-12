// SPDX-License-Identifier: GPL-3.0-only
//! Bounded EP0 framing. Returned words are values, never endpoint ownership.
/// bits16..31 remaining, bit8 ZLP pending. C publishes its source pointer/tx flag.
// Keep to arithmetic supported by the pinned mrustc core-library subset.
#[allow(clippy::manual_is_multiple_of)]
pub fn start(length: u16, wanted: u16) -> u32 {
    let length = length.min(wanted);
    u32::from(length) << 16 | (u32::from(length < wanted && length % 64 == 0) << 8)
}
/// bits0..6 packet bytes, bit7 DataEnd, bit8 new ZLP, bits16..31 new remaining.
pub fn packet(left: u16, zlp: bool) -> u32 {
    let n = left.min(64);
    let left = left - n;
    let last = left == 0 && !(n == 64 && zlp);
    let zlp = zlp && !(left == 0 && n == 64);
    u32::from(n) | (u32::from(last) << 7) | (u32::from(zlp) << 8) | (u32::from(left) << 16)
}

/// Selected request effect. Fields are values; the controller owns all retained
/// endpoint memory and performs IO only after this decision returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Stall = 0,
    Ack = 1,
    Address = 2,
    Descriptor = 3,
    Configuration = 4,
    GetConfiguration = 5,
    Status = 6,
    Stream = 7,
    GetStream = 8,
    ClearHalt = 9,
    SetLine = 10,
    GetLine = 11,
    Dtr = 12,
    SetRate = 13,
    GetRate = 14,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub value: u16,
}
impl Decision {
    pub const fn word(self) -> u32 {
        self.action as u32 | ((self.value as u32) << 8)
    }
}
/// Preserve the frozen request acceptance, including low-byte configuration/
/// interface checks and the original CDC interface permissiveness. This is a
/// compatibility port, not a stricter USB request validator.
pub fn request(
    flags: u32,
    pid: u16,
    code: u16,
    value: u16,
    interface: u8,
    wanted: u16,
    configured: bool,
) -> Decision {
    use Action::*;
    let cdc = flags & 1 != 0;
    let uac = flags & 2 != 0;
    let decision = |action, value| Decision { action, value };
    match code {
        0x0005 => decision(Address, value & 127),
        0x8006 => match crate::usb_descriptors::Descriptors::new(flags, pid).length(value.into()) {
            Some(length) if wanted != 0 => decision(Descriptor, length as u16),
            Some(_) => decision(Ack, 0),
            None => decision(Stall, 0),
        },
        0x0009 if value as u8 <= 1 => decision(Configuration, value & 255),
        0x8008 => decision(GetConfiguration, 0),
        0x8000 | 0x8100 | 0x8200 => decision(Status, 0),
        0x010b if uac && interface == 2 && value <= 1 && configured => decision(Stream, value),
        0x010b if value == 0 => decision(Ack, 0),
        0x810a if uac && interface == 2 => decision(GetStream, 0),
        0x810a => decision(Status, 1),
        0x0201 if uac && value == 0 && interface == 0x84 => decision(Ack, 0),
        0x0201 if value == 0 && interface & 15 <= if cdc { 3 } else { 1 } => {
            if interface & 15 == 0 {
                decision(Ack, 0)
            } else {
                decision(ClearHalt, interface.into())
            }
        }
        0x2120 if cdc && wanted != 0 => decision(SetLine, 0),
        0x2120 if cdc => decision(Ack, 0),
        0xa121 if cdc => decision(GetLine, 0),
        0x2122 if cdc => decision(Dtr, value & 1),
        0x2123 if cdc => decision(Ack, 0),
        0x2201 if uac && interface == 0x84 && value >> 8 == 1 => {
            decision(if wanted != 0 { SetRate } else { Ack }, 0)
        }
        0xa281..=0xa283 if uac && interface == 0x84 && value >> 8 == 1 => decision(GetRate, 0),
        _ => decision(Stall, 0),
    }
}
