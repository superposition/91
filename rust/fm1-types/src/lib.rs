// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-capacity musical representations shared by firmware, tools and browser.
#![no_std]
#![forbid(unsafe_code)]

mod params;
pub use params::*;
mod jam;
pub use jam::JamSettings;
mod command;
pub use command::{CommandKind, DecodedCommand};
mod musical;
pub use musical::{
    AcidLine, AcidStep, ChordKinds, GenreId, LaneId, PatternId, PieceId, PluckLine, PluckStep,
    Progression, StepId,
};

pub const SAMPLE_RATE: u32 = 44_100;
pub const RENDER_FRAMES: usize = 256;
pub const DMA_HALF_FRAMES: usize = 512;
pub const LANES: usize = 6;
pub const PATTERNS: usize = 6;
pub const STEPS: usize = 16;
pub const COMMAND_CAPACITY: usize = 1024;
pub const STORE_MAX: usize = 3840;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueError {
    Index,
    Range,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StepMask(u16);

impl StepMask {
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u16 {
        self.0
    }
    pub fn contains(self, step: u8) -> bool {
        step < 16 && self.0 & (1u16 << step) != 0
    }
    pub fn set(&mut self, step: u8, hit: bool) -> Result<(), ValueError> {
        if step >= 16 {
            return Err(ValueError::Index);
        }
        let mask = 1u16 << step;
        self.0 = if hit { self.0 | mask } else { self.0 & !mask };
        Ok(())
    }
}

pub type PatternBank = [[StepMask; LANES]; PATTERNS];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StepMod(u8);

impl StepMod {
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn ratchet(self) -> u8 {
        self.0 & 7
    }
    pub const fn push(self) -> u8 {
        (self.0 >> 3) & 7
    }
    pub const fn level(self) -> u8 {
        self.0 >> 6
    }
    pub fn new(ratchet: u8, push: u8, level: u8) -> Result<Self, ValueError> {
        if ratchet > 7 || push > 7 || level > 3 {
            return Err(ValueError::Range);
        }
        Ok(Self(ratchet | push << 3 | level << 6))
    }
}

/// Only the low twelve bits represent pitch classes, relative to the tonic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ScaleMask(u16);

impl ScaleMask {
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & 0x0fff)
    }
    pub const fn bits(self) -> u16 {
        self.0
    }
    pub fn contains(self, pitch_class: u8) -> bool {
        pitch_class < 12 && self.0 & (1u16 << pitch_class) != 0
    }
}

/// The stored word is stable even if the decoded command API grows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct CommandWord(u32);

impl CommandWord {
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
    pub const fn new(kind: u8, argument: u8, value: i16) -> Self {
        Self((kind as u32) << 24 | (argument as u32) << 16 | value as u16 as u32)
    }
    pub const fn bits(self) -> u32 {
        self.0
    }
    pub const fn kind(self) -> u8 {
        (self.0 >> 24) as u8
    }
    pub const fn argument(self) -> u8 {
        (self.0 >> 16) as u8
    }
    pub const fn value(self) -> i16 {
        self.0 as u16 as i16
    }
    pub const fn set(parameter: ParamId, value: i16) -> Self {
        Self::new(0, parameter.index() as u8, value)
    }
}

const _: [(); 72] = [(); core::mem::size_of::<PatternBank>()];
const _: [(); 1] = [(); core::mem::size_of::<StepMod>()];
const _: [(); 4096] = [(); core::mem::size_of::<[CommandWord; COMMAND_CAPACITY]>()];
