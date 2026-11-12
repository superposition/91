// SPDX-License-Identifier: GPL-3.0-only
use crate::ValueError;

/// Desired/applied Songbook controls. The C bridge has a checked ten-byte ABI;
/// saved sessions use an explicit codec instead of copying this struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct JamSettings {
    pub on: u8,
    pub key: u8,
    pub minor: u8,
    pub bars: u8,
    pub form: u8,
    pub movement: u8,
    pub energy: u8,
    pub hold: u8,
    pub bass: u8,
    pub slot: u8,
}

impl Default for JamSettings {
    fn default() -> Self {
        Self {
            on: 1,
            key: 0,
            minor: 0,
            bars: 16,
            form: 0,
            movement: 3,
            energy: 60,
            hold: 0,
            bass: 0,
            slot: 0,
        }
    }
}

impl JamSettings {
    pub fn validate(&self) -> Result<(), ValueError> {
        if self.on > 1
            || self.key >= 12
            || self.minor > 1
            || !(self.bars == 8 || self.bars == 16)
            || self.form >= 4
            || self.movement > 3
            || self.energy > 100
            || self.hold > 1
            || self.bass >= 8
            || self.slot > 4
        {
            Err(ValueError::Range)
        } else {
            Ok(())
        }
    }
}

const _: [(); 10] = [(); core::mem::size_of::<JamSettings>()];
