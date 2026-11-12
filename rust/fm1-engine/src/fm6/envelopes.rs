// SPDX-License-Identifier: GPL-3.0-only
// Discrete DX7 envelope transitions, preserving block timing and signed shifts.
use super::{scaling::output_scale, tables::*, types::*, *};
pub fn advance(e: &mut Envelope, op: &[u8], level: i32, rate: i32, index: u32) {
    e.ix = index as u8;
    if index < 4 {
        let lv = op[L1 + index as usize] as i32;
        let a = ((output_scale(lv) >> 1) << 6) + level - 4256;
        let q = (((op[R1 + index as usize] as i32 * 41) >> 6) + rate).clamp(0, 63);
        e.target = a.max(16) << 16;
        e.rising = u8::from(e.target > e.level);
        e.hold = 0;
        if e.target == e.level || (index == 0 && lv == 0) {
            let r = (op[R1 + index as usize] as i32 + rate).clamp(0, 99);
            e.hold = if r < 77 {
                FM6_STATICS[r as usize]
            } else {
                20 * (99 - r)
            };
            if r < 77 && index == 0 && lv == 0 {
                e.hold /= 20;
            }
        }
        e.inc = (4 + (q & 3)) << (2 + 6 + (q >> 2));
    }
}
pub fn step(e: &mut Envelope, op: &[u8], level: i32, rate: i32, down: bool) -> i32 {
    if e.hold != 0 {
        e.hold -= 64;
        if e.hold <= 0 {
            e.hold = 0;
            advance(e, op, level, rate, e.ix as u32 + 1);
        }
    }
    if (e.ix < 3 || (e.ix < 4 && !down)) && e.hold == 0 {
        if e.rising != 0 {
            if e.level < (1716 << 16) {
                e.level = 1716 << 16;
            }
            e.level += (((17 << 24) - e.level) >> 24) * e.inc;
            if e.level >= e.target {
                e.level = e.target;
                advance(e, op, level, rate, e.ix as u32 + 1);
            }
        } else {
            e.level -= e.inc;
            if e.level <= e.target {
                e.level = e.target;
                advance(e, op, level, rate, e.ix as u32 + 1);
            }
        }
    }
    e.level
}
pub fn pitch_advance(e: &mut PitchEnvelope, index: u32) {
    e.pix = index as u8;
    if index < 4 {
        e.pt = FM6_PEG_STEP[e.pv[index as usize] as usize] as i32 * (1 << 19);
        e.prise = u8::from(e.pt > e.pl);
        e.pi = FM6_PEG_RATE[e.pr[index as usize] as usize] as i32 * 1143;
    }
}
pub fn pitch_set(e: &mut PitchEnvelope, p: &[u8; 155]) {
    e.pr.copy_from_slice(&p[PR1..PR1 + 4]);
    e.pv.copy_from_slice(&p[PL1..PL1 + 4]);
    e.pl = FM6_PEG_STEP[e.pv[3] as usize] as i32 * (1 << 19);
    pitch_advance(e, 0);
}
pub fn pitch_step(e: &mut PitchEnvelope, down: bool) -> i32 {
    if e.pix < 3 || (e.pix < 4 && !down) {
        if e.prise != 0 {
            e.pl += e.pi;
            if e.pl >= e.pt {
                e.pl = e.pt;
                pitch_advance(e, e.pix as u32 + 1);
            }
        } else {
            e.pl -= e.pi;
            if e.pl <= e.pt {
                e.pl = e.pt;
                pitch_advance(e, e.pix as u32 + 1);
            }
        }
    }
    e.pl
}
