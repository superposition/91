// SPDX-License-Identifier: GPL-3.0-only
//! Widths are part of the interface. Addresses cannot be constructed by a user
//! of the driver API; these values do not carry a Rust reference into MMIO/RAM.
macro_rules! register {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(u32);
        impl $name {
            pub const fn address(self) -> u32 {
                self.0
            }
        }
    };
}
register!(Reg8);
register!(Reg16);
register!(Reg32);
pub trait RegisterIo {
    fn read8(&mut self, r: Reg8) -> u8;
    fn read16(&mut self, r: Reg16) -> u16;
    fn read32(&mut self, r: Reg32) -> u32;
    fn write8(&mut self, r: Reg8, v: u8);
    fn write16(&mut self, r: Reg16, v: u16);
    fn write32(&mut self, r: Reg32, v: u32);
}
pub(crate) const fn r8(a: u32) -> Reg8 {
    Reg8(a)
}
pub(crate) const fn r16(a: u32) -> Reg16 {
    Reg16(a)
}
pub(crate) const fn r32(a: u32) -> Reg32 {
    Reg32(a)
}
