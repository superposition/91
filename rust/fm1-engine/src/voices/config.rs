// SPDX-License-Identifier: GPL-3.0-only
//! Pure per-block coefficients from borrowed parameters and scalar captures.
//! Readonly recipe views refer to the same exported storage used by C.
use super::{acid, data, drone, pad, setup, DrumsConfig, PluckConfig};
use crate::{
    effects::math::{max, min},
    scheduling_tables::Section,
};
use fm1_types::{ParamId as P, PARAMETER_COUNT};
pub type Params = [i16; PARAMETER_COUNT];
fn p(params: &Params, id: P) -> i32 {
    params[id.index()] as i32
}
fn sound(index: i32, count: usize) -> usize {
    if (index as u32) < count as u32 {
        index as usize
    } else {
        0
    }
}
const SR: f32 = 44_100.0;
const GLIDE: f32 = 1.0 / (0.6 * SR);
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DroneInput {
    pub playing: u32,
    pub played: u32,
    pub line_slide: u32,
    pub bent: u32,
    pub filter: i32,
    pub squash_target: i32,
    pub squash_drive: i32,
    pub squash_amount: i32,
    pub volume: f32,
    pub resonance: f32,
    pub envelope: f32,
    pub velocity: f32,
    pub lfo: f32,
    pub modulation: f32,
    pub arc: f32,
    pub motion: f32,
    pub section: f32,
    pub duck: f32,
    pub pulse: f32,
    pub link: f32,
    pub bend: f32,
    pub tuning: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
}
pub fn drone(
    params: &Params,
    r: &DroneInput,
    sounds: &[data::DroneSound; 24],
    ratios: &[f32; 24],
) -> drone::Config {
    let index = sound(p(params, P::DR_INST), 24);
    let s = &sounds[index];
    let dest = p(params, P::DL_DEST);
    let sq = r.squash_target >= 1;
    let line = p(params, P::BL_MODE) != 0 && r.played == 0;
    let sqd = if sq {
        r.squash_drive as f32 * 0.01
    } else {
        0.0
    };
    let sqa = if sq {
        r.squash_amount as f32 * 0.01
    } else {
        0.0
    };
    let depth = p(params, P::DL_DEPTH) as f32 * 0.01;
    let level = if dest == 2 {
        1.0 - 0.8 * depth * (0.5 - 0.5 * r.lfo)
    } else {
        1.0
    };
    let manual = p(params, P::DR_MAN) != 0;
    let target = if (r.playing != 0 && !manual) || r.played != 0 {
        1.0
    } else {
        0.0
    };
    let mut glide = if r.played != 0 { GLIDE * 12.0 } else { GLIDE };
    if line {
        glide = if p(params, P::BL_GLIDE) != 0 {
            1.0 / ((0.01 + 0.004 * p(params, P::BL_GLIDE) as f32) * SR)
        } else if r.line_slide != 0 {
            1.0 / (0.06 * SR)
        } else {
            1.0
        };
    }
    drone::Config {
        oscillator: s.osc as u32,
        filter: r.filter,
        line: line as u32,
        playing: r.playing,
        drive: 1.4
            * s.drive
            * if dest == 3 {
                1.0 + 2.5 * depth * (0.5 + 0.5 * r.lfo)
            } else {
                1.0
            }
            * (1.0 + 3.0 * sqd),
        k1: r.resonance
            * (1.0 - 0.93 * sqa)
            * if dest == 4 {
                1.0 - 0.85 * depth * (0.5 + 0.5 * r.lfo)
            } else {
                1.0
            },
        k2: 1.4 * (1.0 - 0.75 * sqa),
        input: 1.0 + 2.0 * sqd,
        gain: r.volume * 0.36 * level * s.g / min(1.0 + 0.15 * (s.drive - 1.0), 1.3)
            * (1.0 + r.velocity)
            / (1.0 + 0.5 * sqd + 0.4 * sqa),
        saw: s.saw,
        sub: s.sub,
        target,
        rate: if target > 0.0 {
            1.0 / (if r.played != 0 { 0.03 } else { 2.0 } * SR)
        } else {
            1.0 / (if manual { 0.3 } else { 4.0 } * SR)
        },
        fm_index: 0.5 + 2.2 * r.envelope + 0.3 * r.arc,
        pulse_width: 0.5 + 0.3 * r.modulation,
        glide,
        pitch: if dest == 1 { depth * r.lfo } else { 0.0 },
        bend: if r.bent != 0 { r.bend } else { 0.0 },
        tuning: r.tuning,
        fm_ratio: ratios[index],
        cutoff: p(params, P::DR_CUT) as f32 * 0.075 - 1.5
            + s.cut
            + if dest == 0 { 2.5 * depth * r.lfo } else { 0.0 },
        envelope: 4.0 * p(params, P::DE_AMT) as f32 * 0.01,
        motion: 0.8 * r.motion * r.modulation,
        arc: 0.8 * r.arc,
        break_pos: 2.0 * max(r.section, 0.0),
        break_neg: 1.6 * min(r.section, 0.0),
        duck: 0.35 * r.duck * r.pulse,
        links: 4.0 * p(params, P::LK_KICK) as f32 * 0.01 * r.link,
        gain0: r.gain0,
        gain1: r.gain1,
        inverse: r.inverse,
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AcidInput {
    pub filter: i32,
    pub squash_target: i32,
    pub squash_drive: i32,
    pub squash_amount: i32,
    pub squash_wow: i32,
    pub sliding: u32,
    pub bent: u32,
    pub volume: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub accent: f32,
    pub bend: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
}
pub fn acid(
    params: &Params,
    r: &AcidInput,
    sounds: &[data::AcidSound; 31],
    fm: &[data::AcidFm; 31],
) -> acid::Config {
    let index = sound(p(params, P::AC_WAVE), 31);
    let s = &sounds[index];
    let fm = &fm[index];
    let sq = r.squash_target != 1;
    let sqd = if sq {
        r.squash_drive as f32 * 0.01
    } else {
        0.0
    };
    let sqa = if sq {
        r.squash_amount as f32 * 0.01
    } else {
        0.0
    };
    let sqw = if sq { r.squash_wow as f32 * 0.01 } else { 0.0 };
    let res = p(params, P::AC_RES) as f32 * 0.01;
    let acc = p(params, P::AC_ACC) as f32 * 0.01;
    acid::Config {
        oscillator: s.osc as u32,
        filter: r.filter,
        gain: r.volume * 0.75 * s.g / (1.0 + 0.5 * sqd + 0.4 * sqa),
        drive: s.drive,
        resonance: res,
        envelope: p(params, P::AC_ENV) as f32 * 0.06,
        accent: acc,
        base: p(params, P::AC_CUT) as f32 * 0.066 + r.cutoff,
        k1: (1.45 - 1.38 * res) * (1.0 - 0.93 * sqa) * (1.0 - 0.85 * r.resonance),
        k2: 1.2 * (1.0 - 0.75 * sqa),
        input: 1.0 + 3.0 * sqd,
        wow: sqw,
        velocity: 0.75 + 0.5 * acc * r.accent,
        glide: if r.sliding != 0 { 0.0012 } else { 1.0 },
        bend: if r.bent != 0 { r.bend } else { 1.0 },
        gain0: r.gain0,
        gain1: r.gain1,
        inverse: r.inverse,
        fm_ratio: fm.fr,
        fm_index0: fm.i0,
        fm_index1: fm.i1,
        fm_rise: fm.rise as u32,
    }
}
pub fn breath(params: &Params, playing: u32, sounds: &[data::PadSound; 24]) -> pad::Breath {
    pad::Breath {
        attack: setup::time(0, p(params, P::PE_ATK)),
        hold: setup::time(1, p(params, P::PE_HOLD)),
        release: setup::time(2, p(params, P::PE_REL)),
        rest: setup::time(3, p(params, P::PE_REST)),
        playing,
        manual: p(params, P::PAD_MAN) as u32,
        oscillator: sounds[sound(p(params, P::PADTONE), 24)].osc as u32,
    }
}
pub fn pad_identity(params: &Params, jam: u32, sounds: &[data::PadSound; 24]) -> (u32, u32) {
    let s = &sounds[sound(p(params, P::PADTONE), 24)];
    let comp = p(params, P::PAD_COMP);
    (
        if jam != 0 {
            1
        } else if comp == 1 {
            0
        } else if comp >= 2 {
            1
        } else {
            s.stabs as u32
        },
        s.osc as u32,
    )
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PadInput {
    pub playing: u32,
    pub jam: u32,
    pub dx: u32,
    pub genre: u32,
    pub filter: i32,
    pub volume: f32,
    pub level: f32,
    pub gate: f32,
    pub detune: f32,
    pub motion: f32,
    pub modulation: [f32; 6],
    pub pitch: f32,
    pub bright: f32,
    pub arc: f32,
    pub filter_mod: f32,
    pub panning: f32,
    pub tuning: f32,
    pub stab_decay: f32,
    pub swell: f32,
    pub section: f32,
    pub link: f32,
}
pub fn pad(
    params: &Params,
    r: &PadInput,
    sounds: &[data::PadSound; 24],
    sections: &[Section; 19],
    breaths: &[f32; 4],
    vowels: &[[f32; 2]; 4],
) -> pad::Config {
    let sound = &sounds[sound(p(params, P::PADTONE), 24)];
    let stabs = pad_identity(params, r.jam, sounds).0;
    pad::Config {
        breath: breath(params, r.playing, sounds),
        gain: r.volume
            * 0.5
            * r.level
            * (1.0 + 0.01 * sections[r.genre as usize].pad as f32 * max(r.section, 0.0))
            * sound.g
            * (1.0 + 0.35 * p(params, P::LK_SNARE) as f32 * 0.01 * r.link),
        gate: r.gate,
        spread: if r.jam != 0 && p(params, P::DETUNE) == 0 {
            0.0
        } else {
            r.detune
        },
        motion: r.motion,
        modulation: r.modulation,
        pitch: r.pitch,
        bright: r.bright,
        arc: r.arc,
        filter_mod: r.filter_mod,
        panning: r.panning,
        sound_cut: sound.fc,
        tuning: r.tuning,
        stab_decay: r.stab_decay,
        swell: r.swell,
        breath_scale: *breaths,
        vowels: *vowels,
        stabs,
        dx: r.dx,
        filter: r.filter,
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PadBusInput {
    pub bright: f32,
    pub arc: f32,
    pub filter_mod: f32,
    pub link: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub start: u32,
}
pub fn pad_bus(params: &Params, r: &PadBusInput, sounds: &[data::PadSound; 24]) -> pad::BusConfig {
    pad::BusConfig {
        sound_cut: sounds[sound(p(params, P::PADTONE), 24)].fc,
        bright: r.bright,
        arc: r.arc,
        filter_mod: r.filter_mod,
        link_amount: p(params, P::LK_SNARE) as f32,
        link_env: r.link,
        gain0: r.gain0,
        gain1: r.gain1,
        inverse: r.inverse,
        start: r.start,
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DrumsInput {
    pub volume: f32,
    pub drive: f32,
    pub gains: [f32; 6],
    pub hat_inc: [f32; 4],
    pub gain0: [f32; 4],
    pub gain1: [f32; 4],
    pub inverse: f32,
}
pub fn drums(r: &DrumsInput, kits: [&data::Kit; 4]) -> DrumsConfig {
    let g = r.volume * 1.1;
    let [kk, ks, kh, _] = kits;
    DrumsConfig {
        gains: [
            g * r.gains[0] * 1.15,
            g * 0.38 * r.gains[1],
            g * 0.42 * r.gains[2],
            g * 0.22 * r.gains[3],
            g * 0.18 * r.gains[4],
            g * 0.3 * r.gains[5],
        ],
        hat_inc: r.hat_inc,
        drive: r.drive * kk.k_drive,
        drop: kk.k_drop,
        click: kk.k_click,
        rumble: kk.k_rumble,
        sn_body: ks.sn_body,
        sn_noise: ks.sn_noise,
        cl_gap: ks.cl_gap,
        h_noise: kh.h_noise,
        gains0: r.gain0,
        gains1: r.gain1,
        inverse: r.inverse,
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PluckInput {
    pub volume: f32,
    pub bend: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub old_level: f32,
    pub bent: u32,
}
pub fn pluck(params: &Params, r: &PluckInput, sounds: &[data::PluckSound; 12]) -> PluckConfig {
    let index = p(params, P::CM_SND).clamp(0, 11) as usize;
    let s = &sounds[index];
    PluckConfig {
        gain: r.volume * 0.5 * s.g,
        pitch: if r.bent != 0 { r.bend } else { 1.0 },
        ratio: s.fr,
        index0: s.i0,
        index1: s.i1,
        gain0: r.gain0,
        gain1: r.gain1,
        inverse: r.inverse,
        old_level: r.old_level,
        rise: s.rise as u32,
        fm: u32::from(p(params, P::CM_SND) >= 12),
    }
}
