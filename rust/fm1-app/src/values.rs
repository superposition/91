// SPDX-License-Identifier: GPL-3.0-only
//! Bounded parameter and knob-value policy over the existing UI carrier.
use crate::{
    performance::{Action, Port as EffectPort, Query},
    ui::State,
};
use fm1_types::{
    BassStep, ChordKinds, GenreId, ParamId, PieceId, Progression, StepMod, PARAMETER_COUNT,
};
pub mod code {
    pub const K_LVL: i32 = 200;
    pub const K_TUNE: i32 = 201;
    pub const K_DEC: i32 = 202;
    pub const K_MXT: i32 = 203;
    pub const K_MXL: i32 = 204;
    pub const K_MXP: i32 = 205;
    pub const K_MXS: i32 = 206;
    pub const K_GRT: i32 = 207;
    pub const K_GRD: i32 = 208;
    pub const K_GRS: i32 = 209;
    pub const K_GRA: i32 = 210;
    pub const K_ACT: i32 = 211;
    pub const K_ACD: i32 = 212;
    pub const K_STR: i32 = 213;
    pub const K_STP: i32 = 214;
    pub const K_STV: i32 = 215;
    pub const K_PCOND: i32 = 216;
    pub const K_ASTY: i32 = 217;
    pub const K_ASNO: i32 = 218;
    pub const K_ASOC: i32 = 219;
    pub const K_PSNO: i32 = 220;
    pub const K_PSLV: i32 = 221;
    pub const K_BSTY: i32 = 222;
    pub const K_BSNO: i32 = 223;
    pub const K_BSOC: i32 = 224;
    pub const K_PGSL: i32 = 225;
    pub const K_PGCH: i32 = 226;
    pub const K_PGBR: i32 = 227;
    pub const K_PGPL: i32 = 228;
    pub const K_PGTY: i32 = 229;
    pub const K_SLSL: i32 = 230;
    pub const K_SLGN: i32 = 231;
    pub const K_SLPC: i32 = 232;
    pub const K_SLON: i32 = 233;
    pub const K_GENRE: i32 = 234;
    pub const K_NONE: i32 = 235;
    pub const K_FXF: i32 = 236;
    pub const K_FXC: i32 = 237;
    pub const K_FXT: i32 = 238;
    pub const K_FXW: i32 = 239;
    pub const K_FXP: i32 = 240;
}
use code::*;
macro_rules! ui {($p:ident,$u:ident,$($e:tt)+)=>{$p.with_ui(|$u|$($e)+)};}
#[repr(u32)]
pub enum Read {
    Progression,
    Preview,
    Acid,
    Kinds,
}
#[repr(u32)]
pub enum Operation {
    Parameter,
    Get,
    Range,
    AcidSync,
    AcidKind,
    MoodValue,
    MoodApply,
}
pub trait Port: EffectPort {
    /// A bounded initialized caller buffer, disjoint from UI and all producer
    /// sources. The callback may write only this span and retains no pointer.
    fn read(&mut self, kind: Read, bytes: &mut [u8]) -> bool;
}
pub fn parameter(state: &State, code: i32) -> i32 {
    match code {
        K_LVL => ParamId::LANE.index() as i32 + 3 * i32::from(state.lane),
        K_TUNE => ParamId::LANE.index() as i32 + 3 * i32::from(state.lane) + 1,
        K_DEC => ParamId::LANE.index() as i32 + 3 * i32::from(state.lane) + 2,
        K_MXL => [
            ParamId::PAD,
            ParamId::BASS,
            ParamId::ACID,
            ParamId::CM_LVL,
            ParamId::PULSE,
        ][usize::from(state.mix_trk % 5)]
        .index() as i32,
        K_GRD => ParamId::DENS.index() as i32 + i32::from(state.lane % 6),
        K_GRS => ParamId::LSRC.index() as i32 + i32::from(state.lane % 6),
        K_PCOND => ParamId::COND.index() as i32 + i32::from(state.lane % 6),
        K_MXP => ParamId::PAN.index() as i32 + i32::from(state.mix_trk % 5),
        K_MXS => ParamId::SEND.index() as i32 + i32::from(state.mix_trk % 5),
        _ => {
            if code < PARAMETER_COUNT as i32 {
                code
            } else {
                -1
            }
        }
    }
}
fn q(p: &mut impl EffectPort, query: Query) -> u32 {
    p.query(query, 0)
}
fn stored(p: &mut impl EffectPort, id: ParamId) -> i32 {
    p.query(Query::Parameter, id.index() as u32) as i32
}
fn fx_valid(p: &mut impl EffectPort, code: i32) -> bool {
    let pages = q(p, Query::FxPages);
    code >= K_FXP
        && pages
            .checked_mul(4)
            .and_then(|n| (K_FXP as u32).checked_add(n))
            .is_some_and(|end| (code as u32) < end)
}
pub fn acid_kind(state: &State, step: usize) -> Option<i32> {
    if step >= 16 {
        return None;
    }
    let gate = state.as_gate & (1u16 << step) != 0;
    let accent = state.as_acc & (1u16 << step) != 0;
    let slide = state.as_slide & (1u16 << step) != 0;
    Some(if !gate {
        0
    } else if accent {
        if slide {
            4
        } else {
            2
        }
    } else if slide {
        3
    } else {
        1
    })
}
pub fn acid_sync(p: &mut impl Port) -> bool {
    let now = q(p, Query::Now);
    if ui!(p, u, now.wrapping_sub(u.as_t) < 400) {
        return true;
    }
    let mut bytes = [0u8; 38];
    if !p.read(Read::Acid, &mut bytes) {
        return false;
    }
    ui!(p, u, {
        for step in 0..16 {
            u.as_off[step] = bytes[step] & 15;
            u.as_oct[step] = bytes[16 + step] & 3;
        }
        u.as_gate = u16::from_le_bytes([bytes[32], bytes[33]]);
        u.as_acc = u16::from_le_bytes([bytes[34], bytes[35]]);
        u.as_slide = u16::from_le_bytes([bytes[36], bytes[37]]);
    });
    true
}
pub fn progression_any(progression: Progression) -> bool {
    progression.playing() || (0..8).any(|slot| progression.degree(slot).unwrap_or(0) != 0)
}
fn progression(p: &mut impl Port) -> Option<(Progression, Progression)> {
    let mut bytes = [0u8; 6];
    if !p.read(Read::Progression, &mut bytes) {
        return None;
    }
    let stored = Progression::from_bytes(bytes);
    if !progression_any(stored) && !p.read(Read::Preview, &mut bytes) {
        return None;
    }
    Some((stored, Progression::from_bytes(bytes)))
}
pub fn get(p: &mut impl Port, code: i32) -> Option<i32> {
    let parameter = ui!(p, u, parameter(u, code));
    Some(match code {
        K_MXT => ui!(p, u, i32::from(u.mix_trk)),
        K_GENRE => q(p, Query::Genre) as i32,
        K_ACT => stored(p, ParamId::AC_RESP) / 5,
        K_ACD => {
            let v = stored(p, ParamId::AC_RESP);
            if v >= 5 {
                v % 5
            } else {
                3
            }
        }
        K_GRT => ui!(p, u, i32::from(u.lane)),
        K_GRA => ui!(p, u, i32::from(u.gr_all)),
        K_ASTY | K_ASNO | K_ASOC => {
            if !acid_sync(p) {
                return None;
            }
            ui!(p, u, {
                let step = usize::from(u.astp & 15);
                if code == K_ASNO {
                    i32::from(u.as_off[step])
                } else if code == K_ASOC {
                    i32::from(u.as_oct[step])
                } else {
                    acid_kind(u, step).unwrap_or(0)
                }
            })
        }
        K_PGSL => ui!(p, u, i32::from(u.pgsl & 7)),
        K_SLSL => ui!(p, u, i32::from(u.slsl % 16)),
        K_SLGN | K_SLPC => {
            let slot = ui!(p, u, u.slsl % 16);
            let entry = p.query(Query::SetlistEntry, u32::from(slot)) as i32;
            if entry == 0 {
                0
            } else if code == K_SLGN {
                (entry - 1) / 10 + 1
            } else {
                (entry - 1) % 10
            }
        }
        K_SLON => q(p, Query::SetlistOn) as i32,
        K_PGTY => {
            let mut bytes = [0u8; 3];
            if !p.read(Read::Kinds, &mut bytes) {
                return None;
            }
            let slot = ui!(p, u, usize::from(u.pgsl & 7));
            i32::from(ChordKinds::from_bytes(bytes).kind(slot).ok()?)
        }
        K_PGCH | K_PGBR | K_PGPL => {
            let (_, view) = progression(p)?;
            let slot = ui!(p, u, usize::from(u.pgsl & 7));
            if code == K_PGCH {
                i32::from(view.degree(slot).ok()?)
            } else if code == K_PGBR {
                i32::from(view.bars_log2(slot).ok()?)
            } else {
                let mut bytes = [0u8; 6];
                if !p.read(Read::Progression, &mut bytes) {
                    return None;
                }
                i32::from(Progression::from_bytes(bytes).playing())
            }
        }
        K_BSTY | K_BSNO | K_BSOC => {
            let step = ui!(p, u, u.bstp & 15);
            let value = BassStep::from_bits(p.query(Query::BassStep, u32::from(step)) as u8);
            i32::from(if code == K_BSTY {
                value.kind()
            } else if code == K_BSNO {
                value.note()
            } else {
                value.octave()
            })
        }
        K_PSNO | K_PSLV => {
            let step = ui!(p, u, u.pstp & 15);
            p.query(
                if code == K_PSNO {
                    Query::PluckRung
                } else {
                    Query::PluckLevel
                },
                u32::from(step),
            ) as i32
        }
        K_STR | K_STP | K_STV => {
            let (lane, step) = ui!(p, u, (u.lane % 6, u.stp & 15));
            let value = StepMod::from_bits(
                p.query(Query::ModStep, u32::from(lane) | (u32::from(step) << 8)) as u8,
            );
            i32::from(if code == K_STR {
                value.ratchet()
            } else if code == K_STP {
                value.push()
            } else {
                value.level()
            })
        }
        _ if code >= K_FXP => {
            if !fx_valid(p, code) {
                return None;
            }
            p.query(Query::FxValue, code as u32) as i32
        }
        _ if code >= K_FXF => ui!(p, u, i32::from(u.fxk[(code - K_FXF) as usize])),
        _ if parameter >= 0 => {
            if q(p, Query::JamOn) != 0
                && (parameter == ParamId::ROOT.index() as i32
                    || parameter == ParamId::MODE.index() as i32)
            {
                let settings = q(p, Query::JamSettings);
                if parameter == ParamId::ROOT.index() as i32 {
                    (settings & 255) as i32
                } else if (settings >> 8) & 255 != 0 {
                    5
                } else {
                    0
                }
            } else {
                let query = if q(p, Query::JamOn) != 0 {
                    Query::CurrentParameter
                } else {
                    Query::Parameter
                };
                p.query(query, parameter as u32) as i32
            }
        }
        _ => 0,
    })
}
pub fn range(p: &mut impl EffectPort, code: i32) -> Option<(i32, i32)> {
    let parameter = ui!(p, u, parameter(u, code));
    Some(match code {
        K_MXT => (0, 4),
        K_ACT => (0, 8),
        K_ACD => (0, 4),
        K_GENRE => (0, GenreId::COUNT as i32 - 1),
        K_GRT => (0, 5),
        K_GRA => (0, 100),
        K_STR | K_STP => (0, 7),
        K_STV => (0, 3),
        K_ASTY => (0, 4),
        K_ASNO => (0, 11),
        K_ASOC => (0, 3),
        K_PSNO => (0, 11),
        K_PSLV => (0, 3),
        K_BSTY => (0, 4),
        K_BSNO => (0, 7),
        K_BSOC => (0, 2),
        K_SLSL => (0, 15),
        K_SLGN => (0, GenreId::COUNT as i32),
        K_SLPC => (0, PieceId::COUNT as i32 - 1),
        K_SLON => (0, 1),
        K_PGSL => (0, 7),
        K_PGCH => (0, q(p, Query::ProgressionScale) as i32),
        K_PGBR => (0, 3),
        K_PGTY => (0, 7),
        K_PGPL => (0, 1),
        _ if code >= K_FXP => {
            if !fx_valid(p, code) {
                return None;
            }
            let bits = p.query(Query::FxRange, code as u32);
            (
                i32::from(bits as u16 as i16),
                i32::from((bits >> 16) as u16 as i16),
            )
        }
        _ if code >= K_FXF => {
            if code == K_FXF {
                (-50, 50)
            } else {
                (0, 100)
            }
        }
        _ if parameter >= 0 => {
            let info = ParamId::new(parameter as usize)
                .unwrap_or(ParamId::PAD)
                .info();
            (i32::from(info.min), i32::from(info.max))
        }
        _ => (0, 0),
    })
}
const fn lane_level(lane: usize) -> ParamId {
    match ParamId::new(ParamId::LANE.index() + 3 * lane) {
        Ok(id) => id,
        Err(_) => ParamId::PAD,
    }
}
pub static MOOD: [(ParamId, i8); 15] = [
    (ParamId::AC_DENS, 30),
    (ParamId::AC_CUT, 25),
    (lane_level(3), 25),
    (lane_level(4), 20),
    (lane_level(5), 25),
    (ParamId::BRIGHT, 20),
    (ParamId::CM_DENS, 25),
    (ParamId::CM_LVL, 20),
    (ParamId::CM_DEC, -15),
    (ParamId::DR_CUT, 20),
    (ParamId::DRIVE, 20),
    (ParamId::REVERB, -15),
    (ParamId::ECHO, -10),
    (ParamId::EVOLVE, 20),
    (ParamId::PULSE, 10),
];
pub fn mood_value(mood: u8, parameter: i32, value: i32) -> Option<i32> {
    if mood == 50 || (parameter == ParamId::CM_LVL.index() as i32 && value == 0) {
        return Some(value);
    }
    if let Some(&(id, amount)) = MOOD.iter().find(|(id, _)| id.index() as i32 == parameter) {
        let value = value.checked_add(i32::from(amount) * (i32::from(mood) - 50) / 50)?;
        let info = id.info();
        Some(value.clamp(i32::from(info.min), i32::from(info.max)))
    } else {
        Some(value)
    }
}
pub fn mood_apply(p: &mut impl EffectPort) -> bool {
    for &(id, _) in &MOOD {
        let value = stored(p, id);
        let mood = ui!(p, u, u.mood);
        let Some(value) = mood_value(mood, id.index() as i32, value) else {
            return false;
        };
        p.action(Action::EngineSet, id.index() as u32, value as u32);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bass_bytes_and_reserved_fields_roundtrip_without_losing_bits() {
        assert_eq!(core::mem::size_of::<BassStep>(), 1);
        assert_eq!(core::mem::size_of_val(&MOOD), 30);
        for bits in 0u16..=255 {
            let value = BassStep::from_bits(bits as u8);
            assert_eq!(value.bits(), bits as u8);
            assert_eq!(value.kind(), bits as u8 & 7);
            assert_eq!(value.note(), (bits as u8 >> 3) & 7);
            assert_eq!(value.octave(), bits as u8 >> 6);
            assert_eq!(
                BassStep::new(value.kind(), value.note(), value.octave()),
                Ok(value)
            );
        }
        for field in 0..3 {
            let mut parts = [0, 0, 0];
            parts[field] = [8, 8, 4][field];
            assert!(BassStep::new(parts[0], parts[1], parts[2]).is_err());
        }
    }
    #[test]
    fn mood_keeps_disabled_pluck_and_rejects_arithmetic_overflow() {
        for mood in 0..=255 {
            assert_eq!(mood_value(mood, ParamId::CM_LVL.index() as i32, 0), Some(0));
            assert_eq!(
                mood_value(mood, ParamId::ROOT.index() as i32, -1234),
                Some(-1234)
            );
        }
        assert_eq!(
            mood_value(50, ParamId::PULSE.index() as i32, i32::MAX),
            Some(i32::MAX)
        );
        assert_eq!(
            mood_value(255, ParamId::PULSE.index() as i32, i32::MAX),
            None
        );
        assert_eq!(mood_value(0, ParamId::PULSE.index() as i32, i32::MIN), None);
    }
}
