// SPDX-License-Identifier: GPL-3.0-only
//! Checked views over the original byte/word musical encodings.
use crate::ValueError;

macro_rules! index {
    ($name:ident, $count:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(transparent)]
        pub struct $name(u8);
        impl $name {
            pub const COUNT: usize = $count;
            pub const fn new(value: usize) -> Result<Self, ValueError> {
                if value < Self::COUNT {
                    Ok(Self(value as u8))
                } else {
                    Err(ValueError::Index)
                }
            }
            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}
index!(GenreId, 19);
index!(PieceId, 10);
index!(PatternId, 6);
index!(LaneId, 6);
index!(StepId, 16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct AcidStep(u16);
impl AcidStep {
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & 511)
    }
    pub const fn bits(self) -> u16 {
        self.0
    }
    pub const fn new(
        offset: u8,
        octave: u8,
        gate: bool,
        accent: bool,
        slide: bool,
    ) -> Result<Self, ValueError> {
        if offset > 15 || octave > 3 {
            return Err(ValueError::Range);
        }
        Ok(Self(
            offset as u16
                | (octave as u16) << 4
                | (gate as u16) << 6
                | (accent as u16) << 7
                | (slide as u16) << 8,
        ))
    }
    pub const fn offset(self) -> u8 {
        (self.0 & 15) as u8
    }
    pub const fn octave(self) -> u8 {
        ((self.0 >> 4) & 3) as u8
    }
    pub const fn gate(self) -> bool {
        self.0 & 64 != 0
    }
    pub const fn accent(self) -> bool {
        self.0 & 128 != 0
    }
    pub const fn slide(self) -> bool {
        self.0 & 256 != 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct PluckStep(u8);
impl PluckStep {
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 63)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    /// All sixteen wire rungs are retained; playback may clamp the musical ladder.
    pub const fn new(rung: u8, level: u8) -> Result<Self, ValueError> {
        if rung > 15 || level > 3 {
            Err(ValueError::Range)
        } else {
            Ok(Self(rung | level << 4))
        }
    }
    pub const fn rung(self) -> u8 {
        self.0 & 15
    }
    pub const fn level(self) -> u8 {
        self.0 >> 4
    }
}

fn read<const N: usize>(bytes: &[u8; N], bit: usize, width: usize) -> Result<u16, ValueError> {
    if width == 0 || width > 16 || bit.checked_add(width).is_none_or(|end| end > N * 8) {
        return Err(ValueError::Index);
    }
    let mut value = 0;
    for index in 0..width {
        value |= ((bytes[(bit + index) / 8] >> ((bit + index) % 8) & 1) as u16) << index;
    }
    Ok(value)
}
fn write<const N: usize>(
    bytes: &mut [u8; N],
    bit: usize,
    width: usize,
    value: u16,
) -> Result<(), ValueError> {
    read(bytes, bit, width)?;
    if width < 16 && value >= 1 << width {
        return Err(ValueError::Range);
    }
    for index in 0..width {
        let position = bit + index;
        bytes[position / 8] = (bytes[position / 8] & !(1 << (position % 8)))
            | (((value >> index) & 1) as u8) << (position % 8);
    }
    Ok(())
}

macro_rules! payload {
    ($name:ident, $len:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(transparent)]
        pub struct $name([u8; $len]);
        impl $name {
            pub const EMPTY: Self = Self([0; $len]);
            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }
            pub const fn bytes(self) -> [u8; $len] {
                self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::EMPTY
            }
        }
        const _: [(); $len] = [(); core::mem::size_of::<$name>()];
    };
}
payload!(AcidLine, 18);
payload!(PluckLine, 12);
payload!(Progression, 6);
payload!(ChordKinds, 3);

impl AcidLine {
    pub fn step(self, slot: StepId) -> AcidStep {
        AcidStep::from_bits(read(&self.0, slot.index() * 9, 9).unwrap())
    }
    pub fn set(&mut self, slot: StepId, step: AcidStep) {
        write(&mut self.0, slot.index() * 9, 9, step.bits()).unwrap();
    }
}
impl PluckLine {
    pub fn step(self, slot: StepId) -> PluckStep {
        PluckStep::from_bits(read(&self.0, slot.index() * 6, 6).unwrap() as u8)
    }
    pub fn set(&mut self, slot: StepId, step: PluckStep) {
        write(&mut self.0, slot.index() * 6, 6, step.bits() as u16).unwrap();
    }
    pub fn any(self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}
impl Progression {
    pub fn degree(self, slot: usize) -> Result<u8, ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        Ok(read(&self.0, slot * 3, 3)? as u8)
    }
    pub fn set_degree(&mut self, slot: usize, value: u8) -> Result<(), ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        write(&mut self.0, slot * 3, 3, value as u16)
    }
    pub fn bars_log2(self, slot: usize) -> Result<u8, ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        Ok(read(&self.0, 24 + slot * 2, 2)? as u8)
    }
    pub fn set_bars_log2(&mut self, slot: usize, value: u8) -> Result<(), ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        write(&mut self.0, 24 + slot * 2, 2, value as u16)
    }
    pub fn playing(self) -> bool {
        self.0[5] & 1 != 0
    }
    pub fn set_playing(&mut self, value: bool) {
        write(&mut self.0, 40, 1, value as u16).unwrap();
    }
    /// 0=genre, 1=bars, 2=seconds; wire value3 is retained without reinterpretation.
    pub fn unit(self) -> u8 {
        (self.0[5] >> 1) & 3
    }
    pub fn set_unit(&mut self, value: u8) -> Result<(), ValueError> {
        write(&mut self.0, 41, 2, value as u16)
    }
}
impl ChordKinds {
    pub fn kind(self, slot: usize) -> Result<u8, ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        Ok(read(&self.0, slot * 3, 3)? as u8)
    }
    pub fn set(&mut self, slot: usize, value: u8) -> Result<(), ValueError> {
        if slot >= 8 {
            return Err(ValueError::Index);
        }
        write(&mut self.0, slot * 3, 3, value as u16)
    }
}
