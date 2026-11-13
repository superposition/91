// SPDX-License-Identifier: GPL-3.0-only
//! Bounded legacy/Songbook allocation. Preserve numerical age ordering, caps,
//! first-key LFO reset and stolen-voice phase continuity from the frozen engine.
use super::{envelopes, scaling, tables::FM_CAP, types::*, *};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NoteContext {
    pub jam: u32,
    pub part: u32,
    pub current: i32,
    pub shed: u32,
}
pub struct Pool<'a> {
    pub slots: &'a mut [Slot; 6],
    pub keys_lfo: &'a mut Lfo,
    pub bass_lfo: &'a mut Lfo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub slot: usize,
    pub stolen: bool,
    pub previous: Option<held::Note>,
}
pub fn keyed(s: &mut Voice, patch: &[u8; 155], note: u32) {
    for k in 0..6 {
        let op = &patch[k * OP..(k + 1) * OP];
        s.ol[k] = scaling::output_level(op, note, s.vel as u32) as i16;
        s.rs[k] = scaling::rate_scale(op, note) as i8;
        s.base[k] = scaling::log_frequency(op, note);
    }
}
pub fn note_on(
    pool: Pool<'_>,
    patch: &[u8; 155],
    context: &NoteContext,
    age: &mut u32,
    note: i32,
    velocity: f32,
    gate: i32,
) -> Option<Allocation> {
    if context.current < 0 {
        return None;
    }
    let jam = context.jam != 0;
    let mut cap = if context.shed == 0 {
        6
    } else if context.shed == 1 {
        4
    } else {
        2
    };
    if !jam {
        cap = cap.min(FM_CAP[context.current as usize] as usize);
    } else {
        cap = if context.part != 0 { 1 } else { 5 };
    }
    let used = pool
        .slots
        .iter()
        .filter(|x| x.used != 0 && (!jam || x.part as u32 == context.part))
        .count();
    // Compatibility quirk: keeping best=0 and strict '<' reproduces frozen C
    // when a protected bass candidate has age UINT32_MAX. Do not silently fix
    // the resulting slot-0 steal in a parity port; a separate fix is scoped.
    let mut best = 0;
    let mut stolen = true;
    let mut oldest = u32::MAX;
    for (k, x) in pool.slots.iter().enumerate() {
        if jam && ((k == 4) != (context.part != 0)) {
            continue;
        }
        if x.used == 0 && used < cap {
            best = k;
            stolen = false;
            break;
        }
        if x.used != 0 && x.age < oldest {
            oldest = x.age;
            best = k;
        }
    }
    let previous = held::note(&pool.slots[best]);
    pool.slots[best].part = if jam { context.part as u8 } else { 0 };
    pool.slots[best].owner = 0;
    if !pool
        .slots
        .iter()
        .any(|x| x.used != 0 && x.gate != 0 && (!jam || x.part as u32 == context.part))
    {
        let lfo = if jam && context.part != 0 {
            pool.bass_lfo
        } else {
            pool.keys_lfo
        };
        if patch[LKS] != 0 {
            lfo.ph = (1 << 31) - 1;
        }
        lfo.dly = 0;
    }
    let x = &mut pool.slots[best];
    let s = &mut x.s;
    s.vel = ((velocity * 127.0) as i32).clamp(1, 127) as u8;
    s.note = note as u8;
    keyed(
        s,
        patch,
        (note + patch[TRNSP] as i32 - 24).clamp(0, 127) as u32,
    );
    for k in 0..6 {
        s.eg[k].level = 0;
        envelopes::advance(
            &mut s.eg[k],
            &patch[k * OP..(k + 1) * OP],
            s.ol[k] as i32,
            s.rs[k] as i32,
            0,
        );
        s.porta[k] = s.base[k];
    }
    envelopes::pitch_set(&mut s.pe, patch);
    s.down = 1;
    if patch[OKS] != 0 && !stolen {
        s.ph.fill(0);
        s.gout.fill(0.0);
    }
    s.played = 1;
    s.sub = 0;
    s.quiet = 0;
    s.frozen = 0;
    s.still = 0;
    x.used = 1;
    x.gate = 1;
    x.gate_left = gate;
    x.kill_left = i32::MAX;
    x.fade = 1.0;
    *age = age.wrapping_add(1);
    x.age = *age;
    Some(Allocation {
        slot: best,
        stolen,
        previous,
    })
}
pub fn gates(slots: &mut [Slot; 6], frames: u32, tail: i32) -> u32 {
    let mut released = 0;
    for x in slots {
        if x.used == 0 {
            continue;
        }
        if x.gate != 0 {
            if x.owner == 0 {
                x.gate_left = x.gate_left.wrapping_sub(frames as i32);
                if x.gate_left <= 0 {
                    x.gate = 0;
                    x.kill_left = tail;
                    released += 1;
                }
            }
        } else if x.kill_left > 0x40000000 {
            x.kill_left = tail;
        } else {
            x.kill_left = x.kill_left.wrapping_sub(frames as i32);
            if x.kill_left <= 0 {
                x.fade = if x.fade < 1.0 { x.fade } else { 0.9999 };
            }
        }
    }
    released
}
pub fn reset(slots: &mut [Slot; 6], lfo: &mut Lfo) {
    for x in slots {
        x.used = 0;
        x.owner = 0;
        x.s = Voice::default();
    }
    *lfo = Lfo::default();
}
pub fn retire_all(slots: &mut [Slot; 6]) {
    for x in slots {
        x.used = 0;
        x.owner = 0;
    }
}
pub fn retire_part(slots: &mut [Slot; 6], part: u8) {
    for x in slots {
        if x.part == part {
            x.used = 0;
            x.owner = 0;
        }
    }
}
pub fn part_busy(slots: &[Slot; 6], part: u8) -> bool {
    slots.iter().any(|x| x.used != 0 && x.part == part)
}
pub fn fade_previous_keys(slots: &mut [Slot; 6], old_age: u32) {
    for x in slots {
        if x.used != 0 && x.part == 0 && x.age <= old_age {
            x.fade = 0.9999;
        }
    }
}
const _: [(); 16] = [(); core::mem::size_of::<NoteContext>()];
