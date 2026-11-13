// SPDX-License-Identifier: GPL-3.0-only
use crate::{CommandWord, ValueError};

macro_rules! command_kinds {
    ($($name:ident = $value:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u8)]
        pub enum CommandKind { $($name = $value),+ }
        impl TryFrom<u8> for CommandKind {
            type Error = ValueError;
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value { $($value => Ok(Self::$name),)+ _ => Err(ValueError::Range) }
            }
        }
    };
}

// Stable legacy C_* IDs. These identify actions, not a stored Rust enum layout.
command_kinds! {
    Set = 0, Pool = 1, Play = 2, Next = 3, Freeze = 4, Seed = 5,
    Panic = 6, Clock = 7, Regrow = 8, Section = 9, Fx = 10, Mute = 11,
    Note = 12, Hush = 13, FxKnob = 14, Tracks = 15, GatePattern = 16,
    FxSetting = 17, FillNext = 18, DjSync = 19, FlexHold = 20, Shed = 21,
    UserVoice = 22, UserClear = 23, AcidStep = 24, AcidCapture = 25,
    PluckCapture = 26, Intro = 27, PluckStep = 28, Progression = 29,
    Progression2 = 30, ProgressionKinds = 31, MidiMap = 32, DrumTap = 33,
    ChordDegree = 34, Bend = 35, Pedal = 36, BassStep = 37, Jam = 38,
    JamKey = 39,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedCommand {
    pub kind: CommandKind,
    pub argument: u8,
    pub value: i16,
}

impl CommandWord {
    pub const fn encode(kind: CommandKind, argument: u8, value: i16) -> Self {
        Self::new(kind as u8, argument, value)
    }

    pub fn decode(self) -> Result<DecodedCommand, ValueError> {
        Ok(DecodedCommand {
            kind: self.kind().try_into()?,
            argument: self.argument(),
            value: self.value(),
        })
    }
}
