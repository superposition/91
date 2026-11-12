// SPDX-License-Identifier: GPL-3.0-only
use super::{scaling, tables::*, types::Lfo, Lookup, LFD, LFS, LFW};
pub fn step(l: &mut Lfo, patch: &[u8; 155], t: &Lookup<'_>) {
    let increment = FM6_LFO_INC[patch[LFS] as usize];
    let phase = l.ph.wrapping_add(increment);
    let mut a = 99 - patch[LFD] as u32;
    l.ph = phase;
    l.val = match patch[LFW] {
        0 => (((phase >> 7) ^ 0u32.wrapping_sub(phase >> 31)) as i32) & ((1 << 24) - 1),
        1 => ((!phase ^ (1 << 31)) >> 8) as i32,
        2 => ((phase ^ (1 << 31)) >> 8) as i32,
        3 => ((!phase >> 7) & (1 << 24)) as i32,
        4 => ((0.5 + scaling::sine(t.sin, phase >> 8) * 0.5) * 16777216.0) as i32,
        _ => {
            if phase < increment {
                l.rnd = l.rnd.wrapping_mul(179).wrapping_add(17);
            }
            ((l.rnd ^ 0x80) as i32 + 1) << 16
        }
    };
    let (d1, d2) = if a == 99 {
        (u32::MAX, u32::MAX)
    } else {
        a = (16 + (a & 15)) << (1 + (a >> 4));
        (
            36558 * a,
            36558 * if a & 0xff80 != 0 { a & 0xff80 } else { 0x80 },
        )
    };
    let delay = l.dly.wrapping_add(if l.dly < (1 << 31) { d1 } else { d2 });
    if delay < l.dly {
        l.depth = 1 << 24;
        return;
    }
    l.dly = delay;
    l.depth = if delay < (1 << 31) {
        0
    } else {
        ((delay >> 7) & ((1 << 24) - 1)) as i32
    };
}
