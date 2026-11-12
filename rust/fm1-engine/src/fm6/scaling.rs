// SPDX-License-Identifier: GPL-3.0-only
// FM6 scaling/ROM math from the attributed MSFA/Dexed-based C implementation.
use super::{tables::*, *};
pub fn carrier(algorithm: u32, number: u32) -> bool {
    FM6_ALG[(algorithm & 31) as usize][6 - number as usize] & 3 == 0
}
pub fn carriers(algorithm: u32) -> u32 {
    let mut c = 0;
    for (k, flag) in FM6_ALG[(algorithm & 31) as usize].iter().enumerate() {
        c |= u32::from(flag & 3 == 0) << k;
    }
    c
}
pub fn frequency(lf: i32) -> i32 {
    let i = ((lf as u32 & 0xffffff) >> 14) as usize;
    let f = lf as u32 & 0x3fff;
    let delta = (FM6_FREQ[i + 1] - FM6_FREQ[i]) as u32;
    let y = FM6_FREQ[i] + ((delta * (f >> 7) + ((delta * (f & 127)) >> 7)) >> 7) as i32;
    let shift = 20 - (lf >> 24);
    if shift <= 0 {
        y
    } else if shift > 31 {
        0
    } else {
        y >> shift
    }
}
pub fn phase(ph: u32, mut modulation: f32) -> u32 {
    modulation -= modulation as i32 as f32;
    let offset = (modulation * 16777216.0) as i32;
    ph.wrapping_add(offset as u32) & 0xffffff
}
pub fn sine(table: &[f32; 1025], phase: u32) -> f32 {
    let i = ((phase >> 14) & 1023) as usize;
    let f = (phase & 0x3fff) as f32 * (1.0 / 16384.0);
    table[i] + (table[i + 1] - table[i]) * f
}
pub fn mark1(t: &Lookup<'_>, ph: i32, env: i32) -> i32 {
    let e = (t.mki_log[((ph as u32 >> 12) & 2047) as usize] as u32 + ((ph as u32 >> 8) & 0x8000))
        .wrapping_add(env as u32)
        & 0xffff;
    let y = ((t.mki_exp[(e & 0x3ff) as usize] as u32 >> ((e & 0x7fff) >> 10)) << 13) as i32;
    if e & 0x8000 != 0 {
        -y - 8192
    } else {
        y
    }
}
pub fn opl(t: &Lookup<'_>, ph: i32, env: i32) -> i32 {
    let e = (t.opl_log[((ph as u32 >> 14) & 511) as usize] as u32 + ((ph as u32 >> 8) & 0x8000))
        .wrapping_add((env as u32) << 3)
        & 0xffff;
    let shift = (e & 0x7fff) >> 8;
    let y = if shift > 31 {
        0
    } else {
        (t.opl_exp[(e & 0xff) as usize] as u32 >> shift) as i32
    };
    (if e & 0x8000 != 0 { -y - 1 } else { y }) * (1 << 14)
}
pub fn output_scale(level: i32) -> i32 {
    if level >= 20 {
        28 + level
    } else {
        FM6_LEVELLUT[level.max(0) as usize] as i32
    }
}
pub fn curve(group: i32, depth: i32, curve: i32) -> i32 {
    let s = if curve == 0 || curve == 3 {
        (group * depth * 329) >> 12
    } else {
        (FM6_EXPSCALE[group.min(32) as usize] as i32 * depth * 329) >> 15
    };
    if curve < 2 {
        -s
    } else {
        s
    }
}
pub fn output_level(op: &[u8], note: u32, velocity: u32) -> i32 {
    let offset = note as i32 - op[BP] as i32 - 17;
    let mut l = output_scale(op[OL] as i32)
        + if offset >= 0 {
            curve((offset + 1) / 3, op[RD] as i32, op[RC] as i32)
        } else {
            curve(-(offset - 1) / 3, op[LD] as i32, op[LC] as i32)
        };
    let v = FM6_VELOCITY[(velocity.min(127) >> 1) as usize] as i32 - 239;
    l = l.min(127) * 32 + ((op[KVS] as i32 * v + 7) >> 3) * 16;
    l.max(0)
}
pub fn rate_scale(op: &[u8], note: u32) -> i32 {
    let x = note as i32 / 3 - 7;
    (op[RS] as i32 * x.clamp(0, 31)) >> 3
}
pub fn log_frequency(op: &[u8], note: u32) -> i32 {
    if op[MODE] != 0 {
        let lf = (4458616 * ((op[FC] as i32 & 3) * 100 + op[FF] as i32)) >> 3;
        return lf
            + if op[DET] > 7 {
                13457 * (op[DET] as i32 - 7)
            } else {
                0
            };
    }
    let mut lf = 50857777 + ((1 << 24) / 12) * note as i32;
    lf += ((FM6_DETUNE[note as usize] * (op[DET] as i64 - 7)) >> 24) as i32
        + FM6_COARSE[(op[FC] & 31) as usize];
    lf + FM6_FINE[op[FF] as usize]
}
