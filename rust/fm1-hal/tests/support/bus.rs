// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::{
    drivers,
    io::{Reg16, Reg32, Reg8, RegisterIo},
};
use std::collections::BTreeMap;
pub struct Bus {
    registers: BTreeMap<u32, u32>,
    seed: u32,
    timer: u32,
    adc_reads: u32,
    ready_after: u32,
    pub hash: u32,
    pub events: u32,
    pub trace: Vec<u8>,
}
impl Bus {
    pub fn new(variant: u32) -> Self {
        let mut registers = BTreeMap::new();
        registers.insert(0x10800, if variant & 1 != 0 { 0x4009 } else { 0 });
        Self {
            registers,
            seed: variant.wrapping_mul(0x9e3779b9),
            timer: u32::MAX - variant * 997,
            adc_reads: 0,
            ready_after: if variant & 1 != 0 { 3 } else { u32::MAX },
            hash: 2166136261,
            events: 0,
            trace: Vec::new(),
        }
    }
    pub fn word(&mut self, value: u32) {
        for b in value.to_le_bytes() {
            self.hash = (self.hash ^ u32::from(b)).wrapping_mul(16777619);
            self.trace.push(b);
        }
    }
    fn event(&mut self, kind: u32, address: u32, value: u32) {
        self.word(kind);
        self.word(address);
        self.word(value);
        self.events += 1;
    }
    fn read(&mut self, address: u32, width: u32) -> u32 {
        let mut value = self
            .registers
            .get(&address)
            .copied()
            .unwrap_or(address.wrapping_mul(1664525) ^ self.seed);
        if address == 0x10804 {
            self.timer = self.timer.wrapping_add(24000);
            value = self.timer;
        }
        if address == 0x13100 {
            value &= !0x80;
            self.adc_reads += 1;
            if self.adc_reads >= self.ready_after {
                value |= 0x80;
            }
        }
        if width != 32 {
            value &= (1 << width) - 1;
        }
        self.event(width, address, value);
        value
    }
    fn write(&mut self, address: u32, width: u32, mut value: u32) {
        if width != 32 {
            value &= (1 << width) - 1;
        }
        self.registers.insert(address, value);
        self.event(128 | width, address, value);
    }
}
impl RegisterIo for Bus {
    fn read8(&mut self, r: Reg8) -> u8 {
        self.read(r.address(), 8) as u8
    }
    fn read16(&mut self, r: Reg16) -> u16 {
        self.read(r.address(), 16) as u16
    }
    fn read32(&mut self, r: Reg32) -> u32 {
        self.read(r.address(), 32)
    }
    fn write8(&mut self, r: Reg8, v: u8) {
        self.write(r.address(), 8, u32::from(v));
    }
    fn write16(&mut self, r: Reg16, v: u16) {
        self.write(r.address(), 16, u32::from(v));
    }
    fn write32(&mut self, r: Reg32, v: u32) {
        self.write(r.address(), 32, v);
    }
}
pub fn execute(io: &mut Bus, operation: u32, variant: u32) -> u32 {
    let handler = 0x02001200 + 4 * variant;
    match operation {
        0 => drivers::time_init(io),
        1 => drivers::timer5_start(io, handler, variant),
        2 => drivers::adc_init(io),
        3 => return drivers::adc_read(io, variant) as u32,
        4 => drivers::audio_init(
            io,
            0x01c08000,
            if variant & 1 != 0 { 512 } else { 1024 },
            handler,
            variant,
        ),
        5 => drivers::lcd_init(io),
        6 => drivers::lcd_baud(io, variant),
        7 => drivers::lcd_command(io, (variant * 17) as u8),
        8 => drivers::lcd_data(io, 0x01c20000 + variant * 32, variant * 240),
        9 => return u32::from(drivers::irq_attach(io, variant * 4, handler, variant)),
        _ => panic!("unknown test operation"),
    }
    0
}
