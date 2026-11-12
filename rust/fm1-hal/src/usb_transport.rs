// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-capacity USB-MIDI/SysEx policy. IO owns foreign buffers/publication;
//! this core never borrows endpoint RAM or retains a collector pointer.
pub const SHORT: usize = 16;
pub const DX: usize = 4112;
pub const OTA: usize = 640;
#[derive(Clone, Copy)]
pub enum Buffer {
    Short,
    Parameter,
    Dx,
    Ota,
}
#[derive(Clone, Copy)]
pub enum Scalar {
    ShortLength,
    ShortOn,
    Uboot,
    OtaRequest,
    Telemetry,
    ParameterPosition,
    ParameterOn,
    DxPosition,
    DxOn,
    DxReady,
    DxKind,
    OtaPosition,
    OtaCollect,
    OtaReady,
    OtaLength,
}
/// Scalar and payload slots are disjoint external state. Methods do not retain
/// references. Publication orders payload before its ready/index word.
pub trait Io {
    fn get(&self, key: Scalar) -> u32;
    fn set(&mut self, key: Scalar, value: u32);
    fn read(&self, buffer: Buffer, index: usize) -> u8;
    fn write(&mut self, buffer: Buffer, index: usize, value: u8);
    fn classify_dx(&self, length: usize) -> u8;
    fn publish(&mut self);
    fn parameter(&mut self, event: u32);
    fn midi(&mut self, packet: u32);
    fn endpoint(&self, index: usize) -> u8;
}
pub fn parameter_event(length: usize, mut byte: impl FnMut(usize) -> u8) -> u32 {
    if length < 4 || byte(0) != 0x7d || byte(1) != 0x50 {
        return 0;
    }
    let value14 = |a: u8, b: u8| (u32::from(a) << 7 | u32::from(b)) & 0x3fff;
    match (byte(2), byte(3), length) {
        (0x50, 1, 8) => 1 << 28 | value14(byte(4), byte(5)) << 14 | value14(byte(6), byte(7)),
        (0x50, 2, 4) => 2 << 28,
        (0x56, 0x7e, 5) => 3 << 28 | u32::from(byte(4) & 127),
        (0x42, 1, 5) => 4 << 28 | u32::from(byte(4) & 127),
        (0x42, 4, 7) => 5 << 28 | u32::from(byte(4) & 31) << 14 | value14(byte(5), byte(6)),
        (0x42, 5, 4) => 6 << 28,
        (0x42, 6, 4) => 7 << 28,
        _ => 0,
    }
}
fn parameter_end(io: &mut impl Io) {
    use Scalar::*;
    let n = io.get(ParameterPosition) as usize;
    io.set(ParameterOn, 0);
    let event = parameter_event(n, |index| io.read(Buffer::Parameter, index));
    if event != 0 {
        io.parameter(event);
    }
}
fn dx_end(io: &mut impl Io) {
    use Scalar::*;
    let n = io.get(DxPosition) as usize;
    let kind = if n <= DX { io.classify_dx(n) } else { 0 };
    io.set(DxOn, 0);
    if kind != 0 {
        io.set(DxKind, kind.into());
        io.publish();
        io.set(DxReady, 2);
    }
}
/// bit0 the DX/editor collector; bit1 OTA/telemetry. Ordinary data, including
/// realtime bytes present inside a SysEx packet, follows the frozen byte rules.
/// A ready DX message retains its length on F0 (intentional Busy lifetime fix).
pub fn sysex(io: &mut impl Io, flags: u32, byte: u8) {
    use Scalar::*;
    if flags & 1 != 0 {
        if byte == 0xf0 {
            let available = io.get(DxReady) == 0;
            io.set(DxOn, u32::from(available));
            if available {
                io.set(DxPosition, 0);
            }
            io.set(ParameterOn, 1);
            io.set(ParameterPosition, 0);
        } else if byte == 0xf7 {
            if io.get(ParameterOn) != 0 {
                parameter_end(io);
            }
            if io.get(DxOn) != 0 {
                dx_end(io);
            }
        } else {
            if io.get(DxOn) != 0 {
                let n = io.get(DxPosition) as usize;
                if n < DX {
                    io.write(Buffer::Dx, n, byte);
                    io.set(DxPosition, (n + 1) as u32);
                } else {
                    io.set(DxOn, 0);
                }
            }
            if io.get(ParameterOn) != 0 {
                let n = io.get(ParameterPosition) as usize;
                if n < SHORT {
                    io.write(Buffer::Parameter, n, byte);
                    io.set(ParameterPosition, (n + 1) as u32);
                } else {
                    io.set(ParameterOn, 0);
                }
            }
        }
    }
    if byte == 0xf0 {
        io.set(ShortOn, 1);
        io.set(ShortLength, 0);
        if flags & 2 != 0 {
            let collect = io.get(OtaReady) == 0;
            io.set(OtaCollect, u32::from(collect));
            io.set(OtaPosition, 0);
        }
    }
    if io.get(ShortOn) == 0 {
        return;
    }
    if flags & 2 != 0 && io.get(OtaCollect) != 0 && byte != 0xf0 && byte != 0xf7 {
        let n = io.get(OtaPosition) as usize;
        if n < OTA {
            io.write(Buffer::Ota, n, byte);
            io.set(OtaPosition, (n + 1) as u32);
        } else {
            io.set(OtaCollect, 0);
        }
    }
    let n = io.get(ShortLength) as usize;
    if n < SHORT {
        io.write(Buffer::Short, n, byte);
        io.set(ShortLength, (n + 1) as u32);
    }
    if byte == 0xf7 {
        let six = io.get(ShortLength) == 6;
        let key = [0xf0, 0x22, 0x24, 0x35, 0x7d, 0xf7];
        let uboot = six
            && key
                .iter()
                .enumerate()
                .all(|(i, b)| io.read(Buffer::Short, i) == *b);
        if uboot {
            io.set(Uboot, 1);
        } else if flags & 2 != 0 {
            if six
                && io.read(Buffer::Short, 1) == 0x22
                && io.read(Buffer::Short, 2) == 0x24
                && io.read(Buffer::Short, 3) == 0x35
                && io.read(Buffer::Short, 4) == 0x7f
            {
                io.set(OtaRequest, 1);
            } else if six
                && io.read(Buffer::Short, 1) == 0x7d
                && io.read(Buffer::Short, 2) == 0x50
                && io.read(Buffer::Short, 3) == 0x46
            {
                let request = io.read(Buffer::Short, 4);
                io.set(
                    Telemetry,
                    if request == 2 {
                        3
                    } else {
                        u32::from(1 + (request & 1))
                    },
                );
            } else if io.get(OtaCollect) != 0 {
                let n = io.get(OtaPosition);
                if n != 0 && n <= OTA as u32 {
                    io.set(OtaLength, n);
                    io.publish();
                    io.set(OtaReady, 1);
                }
            }
        }
        // UBOOT also terminates the OTA collector, as in the original branch.
        if flags & 2 != 0 {
            io.set(OtaCollect, 0);
        }
        io.set(ShortOn, 0);
    }
}
/// Consume only complete four-byte events from the captured bounded packet.
/// Disjoint raw endpoint access belongs to IO, not a Rust byte slice.
pub fn receive(io: &mut impl Io, flags: u32, bytes: usize) {
    for index in (0..bytes.min(64) / 4).map(|i| i * 4) {
        let a = io.endpoint(index);
        let b = io.endpoint(index + 1);
        let c = io.endpoint(index + 2);
        let d = io.endpoint(index + 3);
        let cin = a & 15;
        if (4..=7).contains(&cin) {
            let n = if cin == 4 || cin == 7 {
                3
            } else if cin == 6 {
                2
            } else {
                1
            };
            for byte in [b, c, d].into_iter().take(n) {
                sysex(io, flags, byte);
            }
        } else if (8..=14).contains(&cin) || cin == 15 && b >= 0xf8 {
            io.midi(u32::from_le_bytes([a, b, c, d]));
        }
    }
}
