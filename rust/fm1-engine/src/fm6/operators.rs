// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Kerem Kilic (Ellic Studio)
// MSFA/Dexed-based operator kernels: see module/table attribution.
//! Bus selection stays inside one mutable scratch view: the input and output
//! may be the same bus, so separate mutable/immutable slice borrows would alias.
use super::{scaling, types::Voice, Lookup, Scratch, MARK1, MODERN};
#[derive(Clone, Copy)]
pub struct Route {
    pub input: u8,
    pub output: u8,
    pub add: bool,
}
fn read(b: &Scratch<'_>, bus: u8, i: usize) -> f32 {
    match bus {
        0 => 0.0,
        1 => b.bus[0][i],
        2 => b.bus[1][i],
        _ => panic!("invalid FM input bus"),
    }
}
fn write(b: &mut Scratch<'_>, bus: u8, i: usize, add: bool, value: f32) {
    let out = match bus {
        0 => &mut b.sum[i],
        1 => &mut b.bus[0][i],
        2 => &mut b.bus[1][i],
        _ => panic!("invalid FM output bus"),
    };
    if add {
        *out += value;
    } else {
        *out = value;
    }
}
fn iread(b: &Scratch<'_>, bus: u8, i: usize) -> i32 {
    match bus {
        0 => 0,
        1 => b.ibus[0][i],
        2 => b.ibus[1][i],
        _ => panic!("invalid FM input bus"),
    }
}
fn iwrite(b: &mut Scratch<'_>, bus: u8, i: usize, add: bool, value: i32) {
    let out = match bus {
        0 => &mut b.isum[i],
        1 => &mut b.ibus[0][i],
        2 => &mut b.ibus[1][i],
        _ => panic!("invalid FM output bus"),
    };
    if add {
        *out = out.wrapping_add(value);
    } else {
        *out = value;
    }
}
fn wave(t: &Lookup<'_>, eng: u32, phase: u32, gain: f32) -> f32 {
    if eng == MODERN {
        scaling::sine(t.sin, phase) * gain
    } else if eng == MARK1 {
        (1.0 / 16777216.0) * scaling::mark1(t, phase as i32, gain as i32) as f32
    } else {
        (1.0 / 16777216.0) * scaling::opl(t, phase as i32, gain as i32) as f32
    }
}
pub fn operator(
    s: &mut Voice,
    t: &Lookup<'_>,
    b: &mut Scratch<'_>,
    k: usize,
    r: Route,
    eng: u32,
    n: usize,
) {
    assert!(k < 6 && n <= 64);
    let mut phase = s.ph[k];
    let frequency = s.fq[k] as u32;
    let mut gain = s.g[k];
    let delta = s.dg[k];
    for i in 0..n {
        gain += delta;
        let ph = if r.input != 0 {
            scaling::phase(phase, read(b, r.input, i))
        } else {
            phase & 0xffffff
        };
        let y = wave(t, eng, ph, gain);
        write(b, r.output, i, r.add, y);
        phase = phase.wrapping_add(frequency);
    }
    s.ph[k] = phase;
    s.g[k] = gain;
}
pub fn feedback_operator(
    s: &mut Voice,
    t: &Lookup<'_>,
    b: &mut Scratch<'_>,
    k: usize,
    r: Route,
    eng: u32,
    n: usize,
) {
    assert!(k < 6 && n <= 64);
    let mut phase = s.ph[k];
    let frequency = s.fq[k] as u32;
    let mut gain = s.g[k];
    let delta = s.dg[k];
    let mut y0 = s.fb[0];
    let mut y = s.fb[1];
    let feedback = 1.0 / (1u32 << (s.fbs as u32 + 1)) as f32;
    for i in 0..n {
        gain += delta;
        let m = (y0 + y) * feedback;
        y0 = y;
        y = wave(t, eng, scaling::phase(phase, m), gain);
        write(b, r.output, i, r.add, y);
        phase = phase.wrapping_add(frequency);
    }
    s.ph[k] = phase;
    s.g[k] = gain;
    s.fb[0] = y0;
    s.fb[1] = y;
}
pub fn loop_operator(s: &mut Voice, t: &Lookup<'_>, b: &mut Scratch<'_>, output: u8, n: usize) {
    assert!(n <= 64);
    let count = s.loop_ as usize;
    assert!((2..=3).contains(&count));
    let mut y0 = s.fb[0];
    let mut y = s.fb[1];
    let feedback = 1.0 / (1u32 << (s.fbs as u32 + 1)) as f32;
    for i in 0..n {
        let m = (y0 + y) * feedback;
        s.g[0] += s.dg[0];
        y0 = y;
        y = (1.0 / 16777216.0)
            * scaling::mark1(t, scaling::phase(s.ph[0], m) as i32, s.g[0] as i32) as f32;
        s.ph[0] = s.ph[0].wrapping_add(s.fq[0] as u32);
        for j in 1..count {
            y = (1.0 / 16777216.0)
                * scaling::mark1(t, scaling::phase(s.ph[j], y) as i32, s.g[j] as i32) as f32;
            s.ph[j] = s.ph[j].wrapping_add(s.fq[j] as u32);
        }
        write(b, output, i, false, y);
    }
    s.fb[0] = y0;
    s.fb[1] = y;
}
pub fn integer_operator(
    s: &mut Voice,
    t: &Lookup<'_>,
    b: &mut Scratch<'_>,
    k: usize,
    r: Route,
    n: usize,
) {
    assert!(k < 6 && n <= 64);
    let mut phase = s.ph[k];
    let frequency = s.fq[k] as u32;
    let mut gain = s.g[k] as i32;
    let delta = s.dg[k] as i32;
    for i in 0..n {
        gain = gain.wrapping_add(delta);
        let ph = if r.input != 0 {
            phase.wrapping_add(iread(b, r.input, i) as u32)
        } else {
            phase
        } & 0xffffff;
        let y = scaling::mark1(t, ph as i32, gain);
        iwrite(b, r.output, i, r.add, y);
        phase = phase.wrapping_add(frequency);
    }
    s.ph[k] = phase;
    s.g[k] = gain as f32;
}
pub fn feedback_integer(value: i32, shift: u32) -> i32 {
    value.wrapping_add((value >> 31) & ((1u32 << shift) - 1) as i32) >> shift
}
pub fn integer_feedback_operator(
    s: &mut Voice,
    t: &Lookup<'_>,
    b: &mut Scratch<'_>,
    k: usize,
    r: Route,
    n: usize,
) {
    assert!(k < 6 && n <= 64);
    let mut phase = s.ph[k];
    let frequency = s.fq[k] as u32;
    let mut gain = s.g[k] as i32;
    let delta = s.dg[k] as i32;
    let mut y0 = (s.fb[0] * 16777216.0) as i32;
    let mut y = (s.fb[1] * 16777216.0) as i32;
    let shift = s.fbs as u32 + 1;
    for i in 0..n {
        let m = feedback_integer(y0.wrapping_add(y), shift);
        gain = gain.wrapping_add(delta);
        y0 = y;
        y = scaling::mark1(t, (phase.wrapping_add(m as u32) & 0xffffff) as i32, gain);
        iwrite(b, r.output, i, r.add, y);
        phase = phase.wrapping_add(frequency);
    }
    s.ph[k] = phase;
    s.g[k] = gain as f32;
    s.fb[0] = y0 as f32 * (1.0 / 16777216.0);
    s.fb[1] = y as f32 * (1.0 / 16777216.0);
}
pub fn integer_loop_operator(
    s: &mut Voice,
    t: &Lookup<'_>,
    b: &mut Scratch<'_>,
    output: u8,
    n: usize,
) {
    assert!(n <= 64);
    let count = s.loop_ as usize;
    assert!((2..=3).contains(&count));
    let mut y0 = (s.fb[0] * 16777216.0) as i32;
    let mut y = (s.fb[1] * 16777216.0) as i32;
    let shift = s.fbs as u32 + 1;
    let mut gain = s.g[0] as i32;
    let delta = s.dg[0] as i32;
    for i in 0..n {
        let m = feedback_integer(y0.wrapping_add(y), shift);
        gain = gain.wrapping_add(delta);
        y0 = y;
        y = scaling::mark1(t, (s.ph[0].wrapping_add(m as u32) & 0xffffff) as i32, gain);
        s.ph[0] = s.ph[0].wrapping_add(s.fq[0] as u32);
        for j in 1..count {
            y = scaling::mark1(
                t,
                (s.ph[j].wrapping_add(y as u32) & 0xffffff) as i32,
                s.g[j] as i32,
            );
            s.ph[j] = s.ph[j].wrapping_add(s.fq[j] as u32);
        }
        iwrite(b, output, i, false, y);
    }
    s.g[0] = gain as f32;
    s.fb[0] = y0 as f32 * (1.0 / 16777216.0);
    s.fb[1] = y as f32 * (1.0 / 16777216.0);
}
pub fn run(s: &mut Voice, t: &Lookup<'_>, b: &mut Scratch<'_>, n: usize, float_path: bool) {
    assert!(n <= 64);
    let eng = s.eng as u32;
    let integer = eng == MARK1 && !float_path;
    if integer {
        b.isum[..n].fill(0);
    } else {
        b.sum[..n].fill(0.0);
    }
    for k in 0..6 {
        let plan = s.plan[k];
        if plan & 0x80 == 0 {
            continue;
        }
        let route = Route {
            output: plan & 3,
            input: (plan >> 4) & 3,
            add: plan & 4 != 0,
        };
        if integer {
            if k == 0 && s.loop_ != 0 {
                integer_loop_operator(s, t, b, route.output, n);
            } else if plan & 0x40 != 0 {
                integer_feedback_operator(s, t, b, k, route, n);
            } else {
                integer_operator(s, t, b, k, route, n);
            }
        } else if k == 0 && s.loop_ != 0 {
            loop_operator(s, t, b, route.output, n);
        } else if plan & 0x40 != 0 {
            feedback_operator(s, t, b, k, route, eng, n);
        } else {
            operator(s, t, b, k, route, eng, n);
        }
    }
    if integer {
        for k in 0..n {
            b.sum[k] = b.isum[k] as f32 * (1.0 / 16777216.0);
        }
    }
}
