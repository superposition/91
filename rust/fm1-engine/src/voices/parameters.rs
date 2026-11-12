// SPDX-License-Identifier: GPL-3.0-only
//! Voice parameter curves and per-note duration/velocity conversion.
use crate::effects::math::{clamp, exp2, min};
use fm1_types::ParamId as P;
pub fn value(id: usize, v: i32) -> Option<f32> {
    if (P::LANE.index()..P::ECHO.index()).contains(&id) {
        return Some(match (id - P::LANE.index()) % 3 {
            0 => (v * v) as f32 * 0.0001,
            1 => v as f32 * 0.24,
            _ => v as f32 * 0.01,
        });
    }
    if [P::PAD, P::ACID, P::CM_LVL, P::BASS, P::PULSE, P::NOISE]
        .iter()
        .any(|p| p.index() == id)
    {
        return Some((v * v) as f32 * 0.0001);
    }
    if id == P::DETUNE.index() {
        Some(0.00144 * exp2(v as f32 * 0.0332))
    } else if id == P::DR_RES.index() {
        Some(1.5 - 0.0138 * v as f32)
    } else if id == P::DRIVE.index() {
        Some(1.0 + 0.04 * v as f32)
    } else if [P::MOTION, P::NZ_PUMP].iter().any(|p| p.index() == id) {
        Some(v as f32 * 0.01)
    } else if id == P::BRIGHT.index() {
        Some(v as f32)
    } else if id == P::TUNE.index() {
        Some(exp2(v as f32 * (1.0 / 1200.0)))
    } else {
        None
    }
}
pub fn acid_decay(knob: i32) -> f32 {
    0.06 * exp2(knob as f32 * 0.05)
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PluckNote {
    pub slot: i32,
    pub decay: f32,
    pub key_samples: i32,
    pub midi_samples: i32,
    pub velocity: u32,
}
pub fn pluck_note(slot: &mut i32, decay: i32, fm: bool, velocity: f32) -> PluckNote {
    *slot ^= 1;
    let dec = 0.1 * exp2(decay as f32 * 0.045);
    let dk = decay as f32 * 0.01;
    let key = 0.12 + 1.6 * dk * dk;
    PluckNote {
        slot: *slot,
        decay: dec,
        key_samples: (key * 44100.0) as i32,
        midi_samples: (if fm { key } else { dec } * 44100.0) as i32,
        velocity: clamp(40.0 + 87.0 * velocity, 1.0, 127.0) as u32,
    }
}
pub fn hat_increments(metal: f32, increments: &[f32; 4]) -> [f32; 4] {
    increments.map(|v| v * metal)
}
#[repr(C)]
pub struct ManualPluck {
    pub note: i32,
    pub velocity: f32,
}
pub fn manual_pluck(params: &super::config::Params, key: i32, velocity: f32) -> ManualPluck {
    let octave = params[P::OCT.index()] as i32;
    let part_octave = params[P::CM_OCT.index()] as i32;
    let mut note = key + 12 * (octave + part_octave);
    while note > 100 {
        note -= 12;
    }
    while note < 36 {
        note += 12;
    }
    ManualPluck {
        note,
        velocity: min(0.95 * velocity, 1.15),
    }
}
