// SPDX-License-Identifier: GPL-3.0-only
//! Exact width/order sequences. Callers serialize each shared register/RMW
//! transaction with its required IRQ/core policy; &mut Io alone cannot do that.
use crate::io::{r16, r32, r8, Reg16, Reg32, Reg8, RegisterIo};
fn or8(io: &mut impl RegisterIo, r: Reg8, v: u8) {
    let x = io.read8(r);
    io.write8(r, x | v);
}
fn and8(io: &mut impl RegisterIo, r: Reg8, v: u8) {
    let x = io.read8(r);
    io.write8(r, x & v);
}
fn or16(io: &mut impl RegisterIo, r: Reg16, v: u16) {
    let x = io.read16(r);
    io.write16(r, x | v);
}
fn and16(io: &mut impl RegisterIo, r: Reg16, v: u16) {
    let x = io.read16(r);
    io.write16(r, x & v);
}
fn or32(io: &mut impl RegisterIo, r: Reg32, v: u32) {
    let x = io.read32(r);
    io.write32(r, x | v);
}
fn and32(io: &mut impl RegisterIo, r: Reg32, v: u32) {
    let x = io.read32(r);
    io.write32(r, x & v);
}
pub fn time_init(io: &mut impl RegisterIo) {
    if io.read32(r32(0x10800)) & 15 == 9 {
        return;
    }
    io.write32(r32(0x10800), 0);
    io.write32(r32(0x10808), u32::MAX);
    io.write32(r32(0x10804), 0);
    io.write32(r32(0x10800), 0x4009);
}
pub fn delay_us(io: &mut impl RegisterIo, us: u32) {
    let start = io.read32(r32(0x10804));
    let span = us.wrapping_mul(24);
    while io.read32(r32(0x10804)).wrapping_sub(start) < span {}
}
pub fn delay_ms(io: &mut impl RegisterIo, ms: u32) {
    for _ in 0..ms {
        delay_us(io, 1000);
    }
}
pub fn irq_attach(io: &mut impl RegisterIo, n: u32, handler: u32, priority: u32) -> bool {
    if n >= 128 {
        return false;
    }
    let shift = (n & 7) * 4;
    io.write32(r32(0x01c7fe00 + 4 * n), handler);
    let reg = r32(0x1eef100 + 4 * (n >> 3));
    let value = io.read32(reg);
    io.write32(
        reg,
        (value & !(15 << shift)) | ((((priority & 7) << 1) | 1) << shift),
    );
    true
}
pub fn timer5_start(io: &mut impl RegisterIo, handler: u32, priority: u32) {
    io.write32(r32(0x10900), 0x4000);
    io.write32(r32(0x10904), 0);
    io.write32(r32(0x10908), 600);
    irq_attach(io, 63, handler, priority);
    io.write32(r32(0x10900), 0x4019);
}
pub fn adc_init(io: &mut impl RegisterIo) {
    let mask = (1 << 6) | (1 << 1);
    and32(io, r32(0x5004c), !mask);
    and32(io, r32(0x50050), !mask);
    and32(io, r32(0x50054), !mask);
    or32(io, r32(0x50048), mask);
}
pub fn adc_read(io: &mut impl RegisterIo, ch: u32) -> i32 {
    io.write32(r32(0x13100), 0);
    if io.read32(r32(0x11900)) & (1 << 14) != 0 {
        and32(io, r32(0x11900), !(1 << 14));
    }
    io.write32(r32(0x13100), 0xf04e | ((ch & 15) << 8));
    or32(io, r32(0x13100), 0x10);
    or32(io, r32(0x13100), 0x40);
    let mut count = 0;
    while count < 20000 {
        if io.read32(r32(0x13100)) & 0x80 != 0 {
            break;
        }
        count += 1;
    }
    let value = io.read32(r32(0x13104)) & 0x3ff;
    io.write32(r32(0x13100), 0x40);
    io.write32(r32(0x13100), 0);
    if count == 20000 {
        -1
    } else {
        value as i32
    }
}
fn pc_out(io: &mut impl RegisterIo, bit: u32, high: bool) {
    let mask = 1 << bit;
    or32(io, r32(0x5008c), mask);
    if high {
        or32(io, r32(0x50080), mask);
    } else {
        and32(io, r32(0x50080), !mask);
    }
    and32(io, r32(0x50088), !mask);
}
pub fn audio_init(
    io: &mut impl RegisterIo,
    buffer: u32,
    half_words: u32,
    handler: u32,
    priority: u32,
) {
    pc_out(io, 6, false);
    delay_ms(io, 5);
    io.write16(r16(0x12e00), 0);
    io.write16(r16(0x12e04), 0);
    io.write8(r8(0x12e08), 0);
    io.write8(r8(0x12e0c), 0);
    and32(io, r32(0x51030), !0xc0);
    or8(io, r8(0x12e0c), 3);
    pc_out(io, 0, true);
    or16(io, r16(0x12e00), 0x100);
    pc_out(io, 2, true);
    pc_out(io, 1, true);
    or16(io, r16(0x12e00), 0x80);
    and16(io, r16(0x12e00), !0x40);
    io.write16(r16(0x12e20), half_words as u16);
    and16(io, r16(0x12e00), !0x400);
    and16(io, r16(0x12e00), !0x200);
    and32(io, r32(0x10014), 0xfffff0ff);
    and8(io, r8(0x12e0c), !0x1c);
    let value = io.read8(r8(0x12e0c));
    io.write8(r8(0x12e0c), (value & 0x1f) | 0x80);
    or16(io, r16(0x12e04), 1 << 12);
    or16(io, r16(0x12e04), 1 << 14);
    io.write32(r32(0x12e1c), buffer);
    pc_out(io, 6, true);
    and16(io, r16(0x12e04), !(1 << 15));
    io.write8(r8(0x12e08), 15);
    irq_attach(io, 11, handler, priority);
    or16(io, r16(0x12e00), 0x800);
    delay_ms(io, 5);
    pc_out(io, 6, true);
}
pub fn lcd_init(io: &mut impl RegisterIo) {
    and32(io, r32(0x50000), !(1 << 2));
    and32(io, r32(0x50008), !(1 << 2));
    or32(io, r32(0x51020), 0x10);
    or32(io, r32(0x50080), 1 << 7);
    and32(io, r32(0x50080), !((1 << 8) | (1 << 9) | (1 << 10)));
    and32(
        io,
        r32(0x50088),
        !((1 << 7) | (1 << 8) | (1 << 9) | (1 << 10)),
    );
    io.write32(r32(0x11d00), 0x4021);
    io.write32(r32(0x11d04), 4);
}
pub fn lcd_baud(io: &mut impl RegisterIo, value: u32) {
    io.write32(r32(0x11d04), value);
}
pub fn lcd_command(io: &mut impl RegisterIo, value: u8) {
    and32(io, r32(0x50080), !(1 << 8));
    and32(io, r32(0x50080), !(1 << 7));
    or32(io, r32(0x11d00), 0x4000);
    io.write32(r32(0x11d08), value as u32);
}
pub fn lcd_data(io: &mut impl RegisterIo, address: u32, length: u32) {
    or32(io, r32(0x50080), 1 << 8);
    and32(io, r32(0x50080), !(1 << 7));
    or32(io, r32(0x11d00), 0x4000);
    io.write32(r32(0x11d0c), address);
    io.write32(r32(0x11d10), length);
}

/// Completion polling never manufactures a release on timeout. The caller
/// retains/quarantines the DMA source until this returns true or a separately
/// qualified stop backend confirms quiescence.
pub fn lcd_ready(io: &mut impl RegisterIo, single: bool) -> bool {
    let polls = if single { 1 } else { 4_000_000 };
    for _ in 0..polls {
        if io.read32(r32(0x11d00)) & 0x8000 != 0 {
            or32(io, r32(0x11d00), 0x4000);
            return true;
        }
    }
    false
}

pub fn audio_free_half(io: &mut impl RegisterIo) -> u32 {
    u32::from((io.read16(r16(0x12e00)) >> 15) & 1) ^ 1
}
pub fn audio_request_stop(io: &mut impl RegisterIo) {
    and16(io, r16(0x12e00), !0x800);
}
