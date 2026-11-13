// SPDX-License-Identifier: GPL-3.0-only
//! Automatic arrangement and effect policy, over existing caller-owned state.
use crate::performance::{self, parameter, query, tempo, Action, Notice, Port, Query};
use fm1_engine::effects::ids as fx;
use fm1_types::ParamId;

const DJ_LENGTH: [u8; 5] = [8, 16, 32, 64, 16];
const ROUNDS: [u8; 4] = [1, 2, 4, 8];
const RAMP: [u8; 5] = [0, 2, 4, 8, 12];
const INTRO: [u16; 5] = [0, 60, 120, 240, 480];
macro_rules! ui {
    ($p:ident,$u:ident,$($e:tt)+) => {
        $p.with_ui(|$u| $($e)+)
    };
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DjState {
    pub last: u8,
    pub intro: u8,
    pub map: u8,
    pub host: u8,
    pub sync: u8,
}
const _: [(); 5] = [(); core::mem::size_of::<DjState>()];
impl DjState {
    pub const fn initial() -> Self {
        Self {
            last: 255,
            intro: 255,
            map: 255,
            host: 0,
            sync: 0,
        }
    }
}
pub trait DjPort: Port {
    fn with_dj<R>(&mut self, f: impl FnOnce(&mut DjState) -> R) -> R;
}
fn fraction(p: &mut impl Port, q: Query) -> f32 {
    f32::from_bits(query(p, q))
}
pub fn dj_next(p: &mut impl Port) {
    let slot = if query(p, Query::SetlistPlay) != 0 {
        let from = ui!(p, u, u.sl_pos);
        p.query(Query::SetlistStep, u32::from(from)) as i32
    } else {
        -1
    };
    if slot >= 0 {
        ui!(p, u, u.sl_q = (slot + 1) as u8);
    } else {
        let piece = query(p, Query::Piece);
        ui!(p, u, u.piece_q = ((piece + 1) % 10 + 1) as u8);
    }
}
pub fn dj_bar(p: &mut impl Port) {
    let length = ui!(p, u, i32::from(DJ_LENGTH[usize::from(u.dj_len % 5)]));
    let build = query(p, Query::BuildLength) as i32;
    let break_bars = ui!(
        p,
        u,
        if u.dj_break == 0 {
            4
        } else if u.dj_break == 1 {
            (length / 2).max(4)
        } else {
            length
        }
    );
    let aligned = (break_bars + build + 4) / 8 * 8;
    let break_bars = if aligned - build >= 2 {
        aligned - build
    } else {
        aligned + 8 - build
    };
    let sync = ui!(p, u, u.dj_len == 4);
    let rounds = ui!(p, u, i32::from(ROUNDS[usize::from(u.dj_rounds & 3)]));
    let section = query(p, Query::Section);
    let phase = if section == 0 {
        0
    } else if section == 1 {
        1
    } else {
        2
    };
    let last_round = ui!(p, u, i32::from(u.dj_round) + 1 >= rounds);
    ui!(p, u, {
        if phase != u.dj_phase {
            if phase == 0 {
                u.dj_round = u.dj_round.wrapping_add(1);
            }
            u.dj_phase = phase;
            u.dj_n = 0;
        }
        u.dj_fill = 0;
    });
    if fraction(p, Query::IntroFraction) < 1.0 {
        ui!(p, u, u.dj_n = 0);
        return;
    }
    ui!(p, u, u.dj_n = u.dj_n.wrapping_add(1));
    if phase == 0 && ui!(p, u, u.dj_ramp != 0 && u.mx_bars == 0) {
        let bpm = ui!(p, u, i32::from(RAMP[usize::from(u.dj_ramp % 5)]));
        let total = rounds * length;
        let progress = ui!(
            p,
            u,
            if i32::from(u.dj_round) < rounds {
                i32::from(u.dj_round)
            } else {
                rounds - 1
            }
        ) * length
            + ui!(p, u, i32::from(u.dj_n));
        let current = parameter(p, ParamId::TEMPO);
        if current != ui!(p, u, i32::from(u.dj_tlast)) {
            ui!(p, u, u.dj_t0 = (current - bpm * progress / total) as i16);
        }
        let target =
            (ui!(p, u, i32::from(u.dj_t0)) + bpm * progress.min(total) / total).clamp(70, 190);
        if target != parameter(p, ParamId::TEMPO) {
            tempo(p, target);
        }
        ui!(p, u, u.dj_tlast = target as i16);
    }
    if phase == 0 && sync {
        if fraction(p, Query::DriftBars) <= 2.0
            && (ui!(p, u, u.dj_piece != 0) || query(p, Query::SetlistPlay) != 0)
            && last_round
            && ui!(p, u, u.piece_q == 0 && u.sl_q == 0)
        {
            dj_next(p);
        }
    } else if phase == 0 {
        if ui!(p, u, u.dj_n == (length - 1) as u16) {
            if ui!(p, u, u.dj_fillon != 0) {
                p.action(Action::Fill, 0, 0);
            }
            if (ui!(p, u, u.dj_piece != 0) || query(p, Query::SetlistPlay) != 0)
                && last_round
                && ui!(p, u, u.piece_q == 0 && u.sl_q == 0)
            {
                dj_next(p);
            }
        }
        if ui!(p, u, u.dj_n >= length as u16) && query(p, Query::SectionRequested) == 0 {
            p.action(Action::Section, 0, 0);
        }
    } else if phase == 1
        && ui!(p, u, u.dj_n >= break_bars as u16)
        && query(p, Query::SectionRequested) == 0
    {
        p.action(Action::Section, 1, 0);
    }
}
pub fn dj_tick(p: &mut impl DjPort) {
    let position = query(p, Query::DrumPosition) as u8;
    if ui!(p, u, u.pat_rev != 0 && u.pat_up != 0)
        && (query(p, Query::Playing) == 0
            || query(p, Query::PatternNow) == ui!(p, u, u32::from(u.pat_held)))
    {
        let pattern = ui!(p, u, i32::from(u.pat_rev) - 1);
        performance::knob(p, ParamId::PATTERN.index() as u32, pattern);
        let pattern = ui!(p, u, i32::from(u.pat_rev) - 1);
        let playing = query(p, Query::Playing);
        p.notice(Notice::PatternBack, pattern as u32, playing);
        ui!(p, u, {
            u.pat_rev = 0;
            u.pat_up = 0;
        });
    }
    if ui!(p, u, u.dj_on == 0) {
        ui!(p, u, u.dj_fill = 0);
    }
    let intro = ui!(p, u, u.intro);
    if p.with_dj(|s| s.intro != intro) {
        p.with_dj(|s| s.intro = intro);
        let seconds = ui!(p, u, u32::from(INTRO[usize::from(u.intro % 5)]));
        p.action(Action::Intro, seconds, 0);
    }
    let map = ui!(p, u, u.midi_map);
    if p.with_dj(|s| s.map != map) {
        p.with_dj(|s| s.map = map);
        let value = ui!(p, u, u.midi_map);
        p.action(Action::MidiMap, u32::from(value), 0);
    }
    let host = ui!(p, u, u.usb_host);
    if p.with_dj(|s| s.host != host) {
        p.with_dj(|s| s.host = host);
        let value = ui!(p, u, u.usb_host);
        p.action(Action::UsbHost, u32::from(value), 0);
    }
    let message = query(p, Query::HostMessage) as i32;
    if message == 4 || (1..7).contains(&message) {
        p.notice(Notice::Host, message as u32, 0);
        let now = query(p, Query::Now);
        ui!(
            p,
            u,
            u.msg_until = now.wrapping_add(if message == 4 { 4000 } else { 2500 })
        );
    }
    if query(p, Query::JamOn) != 0 || query(p, Query::Playing) == 0 {
        p.with_dj(|s| s.last = 255);
        ui!(p, u, u.scene_q = 0);
        return;
    }
    let want = u8::from(ui!(p, u, u.dj_on != 0 && u.dj_len == 4) && query(p, Query::Section) == 0);
    if p.with_dj(|s| s.sync != want) {
        p.with_dj(|s| s.sync = want);
        p.action(Action::DjSync, u32::from(want), 0);
    }
    let last = p.with_dj(|s| s.last);
    if position != last {
        if position == 0 && last != 255 {
            let scene = ui!(p, u, u.scene_q);
            if scene != 0 {
                ui!(p, u, u.scene_q = 0);
                // Only initialized scene slots can enter this queue through normal policy.
                if scene <= 4 {
                    performance::scene_apply(p, usize::from(scene - 1));
                }
            }
            ui!(
                p,
                u,
                if u.mx_bars != 0 && u.mx_n < 255 {
                    u.mx_n += 1;
                }
            );
            if ui!(p, u, u.dj_on != 0) {
                dj_bar(p);
            }
        }
        p.with_dj(|s| s.last = position);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AutoEffect {
    pub effect: u8,
    pub level: u8,
    pub sections: u8,
    pub length: u8,
    pub favorite: u32,
}
const _: [(); 8] = [(); core::mem::size_of::<AutoEffect>()];
const fn effect(effect: usize, level: u8, sections: u8, length: u8, favorite: u32) -> AutoEffect {
    AutoEffect {
        effect: effect as u8,
        level,
        sections,
        length,
        favorite,
    }
}
macro_rules! genres {($($g:expr),*)=>{0u32 $(| (1u32<<$g))*};}
pub static AUTO: [AutoEffect; 28] = [
    effect(fx::FX_PHASER, 0, 3, 32, genres!(13, 17, 18, 10, 2)),
    effect(fx::FX_FLANGER, 0, 3, 32, genres!(13, 17, 18, 0)),
    effect(fx::FX_TAPE, 0, 3, 32, genres!(1, 15, 4, 12, 13)),
    effect(fx::FX_THROW, 0, 3, 4, genres!(4, 12, 3, 14, 2)),
    effect(fx::FX_SWEEP, 0, 5, 32, genres!(0, 2, 17, 18, 16)),
    effect(fx::FX_RVSWELL, 0, 6, 16, genres!(4, 15, 1, 3)),
    effect(fx::FX_FILL, 0, 1, 16, genres!(6, 11, 5, 7)),
    effect(fx::FX_LP, 1, 3, 16, genres!(2, 10, 7, 0)),
    effect(fx::FX_HP, 1, 5, 16, genres!(2, 0, 17, 5)),
    effect(fx::FX_GATE16, 1, 1, 16, genres!(0, 17, 18, 16, 13)),
    effect(fx::FX_GATE8, 1, 1, 16, genres!(2, 10, 13, 7)),
    effect(fx::FX_PUMP, 1, 1, 32, genres!(2, 10, 7, 14)),
    effect(fx::FX_RPT8, 1, 1, 8, genres!(6, 5, 8, 11)),
    effect(fx::FX_RPT16, 1, 5, 8, genres!(5, 6, 9, 3, 17)),
    effect(fx::FX_RPT8T, 1, 1, 8, genres!(14, 9, 7)),
    effect(fx::FX_HATROLL, 1, 5, 8, genres!(9, 18, 5, 16)),
    effect(fx::FX_SNROLL, 1, 4, 16, genres!(3, 9, 5, 6)),
    effect(fx::FX_ROLL, 1, 4, 16, genres!(2, 0, 17, 10)),
    effect(fx::FX_FREEZE, 1, 2, 16, genres!(0, 15, 16, 18)),
    effect(fx::FX_RING, 2, 3, 8, genres!(16, 18, 8)),
    effect(fx::FX_SCATTER, 2, 1, 8, genres!(6, 5, 8, 16)),
    effect(fx::FX_SCRATCH, 2, 1, 8, genres!(6, 11, 8, 15, 1)),
    effect(fx::FX_HALF, 2, 1, 16, genres!(3, 9, 15)),
    effect(fx::FX_DBLKICK, 2, 1, 8, genres!(0, 16, 18, 17)),
    effect(fx::FX_REV, 2, 1, 4, genres!(5, 6, 3)),
    effect(fx::FX_STOP, 2, 1, 4, genres!(3, 9, 4, 1, 15)),
    effect(fx::FX_BRAKE, 2, 1, 8, genres!(1, 15, 4, 12)),
    effect(fx::FX_SPIN, 2, 1, 4, genres!(4, 12, 6)),
];
pub fn auto_metadata(index: usize, field: u32) -> Option<u32> {
    if field == 5 {
        return MIX_KEYS.get(index).map(|value| i32::from(*value) as u32);
    }
    let entry = AUTO.get(index)?;
    Some(match field {
        0 => u32::from(entry.effect),
        1 => u32::from(entry.level),
        2 => u32::from(entry.sections),
        3 => u32::from(entry.length),
        4 => entry.favorite,
        _ => return None,
    })
}
fn dice(p: &mut impl Port) -> u32 {
    ui!(p, u, {
        u.af_rng ^= u.af_rng << 13;
        u.af_rng ^= u.af_rng >> 17;
        u.af_rng ^= u.af_rng << 5;
        u.af_rng
    })
}
pub fn auto_fx(p: &mut impl Port) {
    const CHANCE: [u32; 4] = [0, 50, 70, 90];
    let (amount, length, every) = ui!(
        p,
        u,
        (
            u.fxs[fx::EN_FXS_AF_AMT] & 3,
            4i32 << (u.fxs[fx::EN_FXS_AF_LEN] & 3),
            4u32 << (u.fxs[fx::EN_FXS_AF_EVERY] & 3)
        )
    );
    let sixteenth = query(p, Query::Bars);
    let bar = sixteenth >> 4;
    if ui!(p, u, u.af_rng == 0) {
        let now = query(p, Query::Now);
        ui!(p, u, u.af_rng = now.wrapping_mul(2654435761u32) | 1);
    }
    if query(p, Query::Section) != ui!(p, u, u32::from(u.af_sec)) {
        if query(p, Query::Section) == 0 && ui!(p, u, u.af_sec == 2) {
            ui!(p, u, {
                u.af_quiet = bar.wrapping_add(4);
                u.af_end = 0;
            });
        }
        let section = query(p, Query::Section);
        ui!(p, u, u.af_sec = section as u8);
    }
    if query(p, Query::JamOn) != 0
        || amount == 0
        || query(p, Query::Playing) == 0
        || ui!(p, u, u.fx_menu != 0 || u.af_user != 0)
        || fraction(p, Query::IntroFraction) < 1.0
    {
        ui!(p, u, u.af_live = 0);
        if query(p, Query::JamOn) != 0
            || amount == 0
            || query(p, Query::Playing) == 0
            || ui!(p, u, u.af_user != 0)
        {
            ui!(p, u, u.af_end = 0);
        }
        ui!(p, u, u.af_bar = bar);
        return;
    }
    ui!(p, u, {
        if u.af_end != 0 && sixteenth >= u.af_end {
            u.af_end = 0;
        }
        u.af_live = u8::from(u.af_end != 0 && sixteenth >= u.af_start);
    });
    if ui!(p, u, u.af_live != 0 && u.af_said == 0) {
        ui!(p, u, u.af_said = 1);
        let effect = ui!(p, u, u.af_fx);
        p.notice(Notice::AutoFx, u32::from(effect), 0);
    }
    if bar == ui!(p, u, u.af_bar) {
        return;
    }
    ui!(p, u, u.af_bar = bar);
    if ui!(p, u, u.af_end != 0 || bar < u.af_quiet) {
        return;
    }
    let predrop = query(p, Query::Section) == 2 && query(p, Query::SectionBars) == 1 && amount >= 2;
    let count = if predrop {
        1u32
    } else if length > 16 {
        2
    } else {
        1
    };
    if !predrop
        && (bar.wrapping_add(count) % every != 0 || dice(p) % 100 >= CHANCE[usize::from(amount)])
    {
        return;
    }
    let mut candidates = [0u8; 28];
    let mut weights = [0u8; 28];
    let mut available = 0usize;
    let mut total = 0u32;
    for (index, entry) in AUTO.iter().enumerate() {
        if entry.level >= amount
            || ui!(
                p,
                u,
                entry.effect == u.af_last[0] || entry.effect == u.af_last[1]
            )
        {
            continue;
        }
        if predrop {
            if ![fx::FX_STOP, fx::FX_REV, fx::FX_BRAKE, fx::FX_SPIN]
                .contains(&usize::from(entry.effect))
            {
                continue;
            }
        } else if u32::from(entry.sections)
            .checked_shr(query(p, Query::Section))
            .unwrap_or(0)
            & 1
            == 0
        {
            continue;
        }
        if ui!(p, u, u.dj_on != 0) && usize::from(entry.effect) == fx::FX_FILL {
            continue;
        }
        candidates[available] = index as u8;
        let preferred = ui!(p, u, u.fxs[fx::EN_FXS_AF_PICK] & 1 == 0)
            && (entry.favorite >> (query(p, Query::Genre) % 32)) & 1 != 0;
        weights[available] = if preferred { 4 } else { 1 };
        total += u32::from(weights[available]);
        available += 1;
    }
    if available == 0 {
        return;
    }
    let mut pick = dice(p) % total;
    let mut position = 0;
    while position + 1 < available && pick >= u32::from(weights[position]) {
        pick -= u32::from(weights[position]);
        position += 1;
    }
    let entry = &AUTO[usize::from(candidates[position])];
    let length = if predrop {
        i32::from(entry.length)
    } else {
        length.min(i32::from(entry.length))
    };
    ui!(p, u, {
        u.af_fx = entry.effect;
        u.af_last[1] = u.af_last[0];
        u.af_last[0] = u.af_fx;
        u.af_end = bar.wrapping_add(count).wrapping_mul(16);
        u.af_start = u.af_end.wrapping_sub(length as u32);
        u.af_said = 0;
    });
}
pub static MIX_KEYS: [i8; 27] = [
    0, 8, 1, 9, 2, 10, 28, 3, 11, 4, 12, 29, 5, 13, 6, 14, 16, 17, 7, 20, 21, 22, 23, 24, 25, 15,
    27,
];
pub fn mix_key(p: &mut impl Port, key: usize) -> bool {
    let Some(&code) = MIX_KEYS.get(key) else {
        return false;
    };
    if code < 0 {
        return true;
    }
    match code {
        0..=7 => {
            ui!(p, u, u.trk_m ^= 1u8 << code);
            let on = ui!(p, u, u.trk_m & (1u8 << code) != 0);
            p.notice(Notice::MixTrack, code as u32, u32::from(on) << 1);
        }
        8..=15 => {
            let track = code - 8;
            ui!(p, u, u.trk_s ^= 1u8 << track);
            let on = ui!(p, u, u.trk_s & (1u8 << track) != 0);
            p.notice(Notice::MixTrack, track as u32, 1 | (u32::from(on) << 1));
        }
        16 | 17 => {
            performance::undo_save(p);
            let on = ui!(p, u, {
                let value = if code == 16 {
                    &mut u.trk_m
                } else {
                    &mut u.trk_s
                };
                *value = if *value & 15 == 15 {
                    *value & !15
                } else {
                    *value | 15
                };
                *value & 15 != 0
            });
            p.notice(Notice::MixDrums, code as u32, u32::from(on));
        }
        18 => {
            performance::undo_save(p);
            ui!(p, u, {
                u.trk_m = 0;
                u.trk_s = 0;
            });
            p.notice(Notice::AllTracks, 0, 0);
        }
        20..=23 => {
            let scene = (code - 20) as usize;
            ui!(p, u, u.scene_pend |= 1u8 << scene);
            let now = query(p, Query::Now);
            ui!(p, u, u.scene_t[scene] = now);
        }
        24 => performance::scene_reset(p),
        28 => performance::undo_swap(p),
        29 => {
            if !performance::tap(p) {
                return false;
            }
        }
        25 => {
            ui!(p, u, {
                u.dj_on = u8::from(u.dj_on == 0);
                u.dj_phase = 0;
                u.dj_n = 0;
                u.dj_round = 0;
                u.dj_fill = 0;
            });
            let on = ui!(p, u, u.dj_on);
            p.notice(Notice::DjOn, u32::from(on), 0);
        }
        26 => {
            ui!(p, u, u.dj_len = ((u32::from(u.dj_len) + 1) % 5) as u8);
            let length = ui!(p, u, u.dj_len);
            p.notice(Notice::DjLength, u32::from(length), 0);
            ui!(p, u, u.dj_n = 0);
        }
        27 => {
            ui!(p, u, u.dj_piece = u8::from(u.dj_piece == 0));
            let on = ui!(p, u, u.dj_piece);
            p.notice(Notice::DjPieces, u32::from(on), 0);
        }
        _ => {}
    }
    let (mute, solo) = ui!(p, u, (u.trk_m, u.trk_s));
    p.action(Action::Tracks, u32::from(mute), u32::from(solo));
    true
}
