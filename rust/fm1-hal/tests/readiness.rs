// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::{
    drivers,
    io::{Reg16, Reg32, Reg8, RegisterIo},
};
struct Spi {
    reads: u32,
    writes: u32,
    ready_at: u32,
}
impl RegisterIo for Spi {
    fn read8(&mut self, _: Reg8) -> u8 {
        panic!("wrong width")
    }
    fn read16(&mut self, _: Reg16) -> u16 {
        panic!("wrong width")
    }
    fn read32(&mut self, r: Reg32) -> u32 {
        assert_eq!(r.address(), 0x11d00);
        self.reads += 1;
        0x1521
            | if self.reads >= self.ready_at {
                0x8000
            } else {
                0
            }
    }
    fn write8(&mut self, _: Reg8, _: u8) {
        panic!("wrong width")
    }
    fn write16(&mut self, _: Reg16, _: u16) {
        panic!("wrong width")
    }
    fn write32(&mut self, r: Reg32, value: u32) {
        assert_eq!(r.address(), 0x11d00);
        assert_eq!(value, 0xd521); // completion ACK preserves every old bit
        self.writes += 1;
    }
}
#[test]
fn timeout_and_late_ready_never_acknowledge_unfinished_dma() {
    let mut io = Spi {
        reads: 0,
        writes: 0,
        ready_at: 4_000_002,
    };
    assert!(!drivers::lcd_ready(&mut io, false));
    assert_eq!((io.reads, io.writes), (4_000_000, 0));
    assert!(!drivers::lcd_ready(&mut io, true));
    assert_eq!((io.reads, io.writes), (4_000_001, 0));
    assert!(drivers::lcd_ready(&mut io, true));
    assert_eq!((io.reads, io.writes), (4_000_003, 1));
}
#[test]
fn last_permitted_observation_is_required_before_release() {
    let mut io = Spi {
        reads: 0,
        writes: 0,
        ready_at: 4_000_000,
    };
    assert!(drivers::lcd_ready(&mut io, false));
    assert_eq!((io.reads, io.writes), (4_000_001, 1));
}
