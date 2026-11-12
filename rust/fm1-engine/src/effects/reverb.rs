// SPDX-License-Identifier: GPL-3.0-only
//! Dattorro tank and shimmer. The shared predelay is deliberately outside this borrow.
use super::math::*;
const fn ds(n: usize) -> usize {
    (n * 14818 + 5000) / 10000
}
pub const LENGTHS: [usize; 12] = [
    ds(142),
    ds(107),
    ds(379),
    ds(277),
    ds(672) + ds(16) + 2,
    ds(4453),
    ds(1800),
    ds(3720),
    ds(908) + ds(16) + 2,
    ds(4217),
    ds(2656),
    ds(3163),
];
pub const WORDS: usize = 33383;
pub const SHIMMER: usize = 2048;
/// Only the exact retained predelay read span is borrowed. The foreground may
/// concurrently write its disjoint span elsewhere in the same C allocation.
pub fn process(
    s: &mut State,
    c: &Config,
    tank: &mut [i16; WORDS],
    shimmer: &mut [f32; SHIMMER],
    input: &[i16],
    left: &mut [f32],
    right: &mut [f32],
) {
    assert_eq!(input.len(), left.len());
    assert_eq!(input.len(), right.len());
    for j in 0..input.len() {
        let x = sample(input[j] as f32 * (1.0 / 16384.0), s, c, tank, shimmer);
        left[j] = x[0];
        right[j] = x[1];
    }
}
/// This is deliberately the historical partial reset, including leaving the
/// shimmer payload/write position/phase and gated envelope untouched.
pub fn reset(s: &mut State, tank: &mut [i16; WORDS]) {
    tank.fill(0);
    s.positions.fill(0);
    s.bandwidth = 0.0;
    s.damp_left = 0.0;
    s.damp_right = 0.0;
    s.shimmer_dc = 0.0;
    s.sine = 0.0;
    s.cosine = 1.0;
}
#[repr(C)]
pub struct Settings {
    pub kind: i32,
    pub pre: i32,
    pub damping: f32,
    pub hold: f32,
    pub shimmer: f32,
}
const _: [(); 20] = [(); core::mem::size_of::<Settings>()];
pub fn settings(kind: i32, damping: i32, pre: i32, x: i32) -> Settings {
    Settings {
        kind: kind.clamp(0, 2),
        pre: 256 + (1322 - 256) * pre / 100,
        damping: 0.85 - 0.0085 * damping as f32,
        shimmer: 0.05 + 0.30 * x as f32 * 0.01,
        hold: (0.08 + 0.32 * x as f32 * 0.01) * 44100.0,
    }
}
pub fn decay(state: &mut [f32; 3], target: f32, frozen: bool) {
    state[0] += ((if frozen { 0.9995 } else { target }) - state[0]) * 0.002;
    state[1] = clamp(state[0] + 0.15, 0.25, 0.5);
    state[2] += ((if frozen { 0.0 } else { 1.0 }) - state[2]) * 0.01;
}
const OFFSETS: [usize; 12] = {
    let mut a = [0; 12];
    let mut i = 1;
    while i < 12 {
        a[i] = a[i - 1] + LENGTHS[i - 1];
        i += 1;
    }
    a
};
const IN1: usize = 0;
const IN2: usize = 1;
const IN3: usize = 2;
const IN4: usize = 3;
const APL: usize = 4;
const D1L: usize = 5;
const AP2L: usize = 6;
const D2L: usize = 7;
const APR: usize = 8;
const D1R: usize = 9;
const AP2R: usize = 10;
const D2R: usize = 11;
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub positions: [i32; 12],
    pub gate_envelope: f32,
    pub gate_gain: f32,
    pub gate_hold: f32,
    pub shimmer_feedback: f32,
    pub shimmer_dc: f32,
    pub shimmer_phase: f32,
    pub bandwidth: f32,
    pub damp_left: f32,
    pub damp_right: f32,
    pub sine: f32,
    pub cosine: f32,
    pub shimmer_write: u32,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub kind: i32,
    pub damping: f32,
    pub hold: f32,
    pub shimmer: f32,
    pub decay: f32,
    pub diffusion: f32,
}
const _: [(); 96] = [(); core::mem::size_of::<State>()];
const _: [(); 24] = [(); core::mem::size_of::<Config>()];
fn tap(data: &[i16; WORDS], s: &State, line: usize, delay: usize) -> f32 {
    let mut index = s.positions[line] - delay as i32;
    if index < 0 {
        index += LENGTHS[line] as i32;
    }
    data[OFFSETS[line] + index as usize] as f32 * (1.0 / 16384.0)
}
pub fn quantize(x: f32) -> i16 {
    let x = clamp(x * 16384.0, -32767.0, 32767.0);
    (x + if x < 0.0 { -0.5 } else { 0.5 }) as i32 as i16
}
fn push(data: &mut [i16; WORDS], s: &mut State, line: usize, x: f32) {
    data[OFFSETS[line] + s.positions[line] as usize] = quantize(x);
    s.positions[line] += 1;
    if s.positions[line] >= LENGTHS[line] as i32 {
        s.positions[line] = 0;
    }
}
fn allpass(data: &mut [i16; WORDS], s: &mut State, line: usize, len: usize, x: f32, g: f32) -> f32 {
    let z = tap(data, s, line, len);
    let v = x - g * z;
    push(data, s, line, flush(v));
    z + g * v
}
fn allpass_mod(
    data: &mut [i16; WORDS],
    s: &mut State,
    line: usize,
    len: f32,
    x: f32,
    g: f32,
) -> f32 {
    let k = len as usize;
    let f = len - k as f32;
    let z = tap(data, s, line, k) + (tap(data, s, line, k + 1) - tap(data, s, line, k)) * f;
    let v = x - g * z;
    push(data, s, line, flush(v));
    z + g * v
}
fn shimmer_tap(data: &[f32; SHIMMER], write: u32, delay: f32) -> f32 {
    let mut rp = write as f32 - delay;
    if rp < 0.0 {
        rp += 2048.0;
    }
    let i = rp as usize;
    let f = rp - i as f32;
    let next = if i + 1 >= 2048 { 0 } else { i + 1 };
    data[i] + (data[next] - data[i]) * f
}
pub fn sample(
    mut x: f32,
    s: &mut State,
    c: &Config,
    data: &mut [i16; WORDS],
    shimmer: &mut [f32; SHIMMER],
) -> [f32; 2] {
    let fb_l = tap(data, s, D2R, ds(3163));
    let fb_r = tap(data, s, D2L, ds(3720));
    if c.kind == 2 {
        s.gate_envelope = flush(max(abs(x), s.gate_envelope * 0.9995));
        if s.gate_envelope > 0.004 {
            s.gate_hold = c.hold;
        } else if s.gate_hold > 0.0 {
            s.gate_hold -= 1.0;
        }
        s.gate_gain = flush(
            s.gate_gain + ((if s.gate_hold > 0.0 { 1.0 } else { 0.0 }) - s.gate_gain) * 0.002,
        );
    } else if c.kind == 1 {
        x += s.shimmer_feedback;
    }
    s.bandwidth += 0.6 * (x - s.bandwidth);
    x = allpass(data, s, IN1, ds(142), s.bandwidth, 0.75);
    x = allpass(data, s, IN2, ds(107), x, 0.75);
    x = allpass(data, s, IN3, ds(379), x, 0.625);
    x = allpass(data, s, IN4, ds(277), x, 0.625);
    let sine = s.sine + 0.0001 * s.cosine;
    let cosine = s.cosine - 0.0001 * sine;
    s.sine = sine;
    s.cosine = cosine;
    let a = allpass_mod(
        data,
        s,
        APL,
        ds(672) as f32 + ds(16) as f32 * s.sine,
        x + c.decay * fb_l,
        -0.7,
    );
    push(data, s, D1L, a);
    let mut b = tap(data, s, D1L, ds(4453));
    s.damp_left += c.damping * (b - s.damp_left);
    b = allpass(data, s, AP2L, ds(1800), s.damp_left * c.decay, c.diffusion);
    push(data, s, D2L, b);
    let a = allpass_mod(
        data,
        s,
        APR,
        ds(908) as f32 + ds(16) as f32 * s.cosine,
        x + c.decay * fb_r,
        -0.7,
    );
    push(data, s, D1R, a);
    let mut b = tap(data, s, D1R, ds(4217));
    s.damp_right += c.damping * (b - s.damp_right);
    b = allpass(data, s, AP2R, ds(2656), s.damp_right * c.decay, c.diffusion);
    push(data, s, D2R, b);
    let mut left = 0.6
        * (tap(data, s, D1R, ds(266)) + tap(data, s, D1R, ds(2974)) - tap(data, s, AP2R, ds(1913))
            + tap(data, s, D2R, ds(1996))
            - tap(data, s, D1L, ds(1990))
            - tap(data, s, AP2L, ds(187))
            - tap(data, s, D2L, ds(1066)));
    let mut right = 0.6
        * (tap(data, s, D1L, ds(353)) + tap(data, s, D1L, ds(3627)) - tap(data, s, AP2L, ds(1228))
            + tap(data, s, D2L, ds(2673))
            - tap(data, s, D1R, ds(2111))
            - tap(data, s, AP2R, ds(335))
            - tap(data, s, D2R, ds(121)));
    if c.kind == 2 {
        left *= s.gate_gain;
        right *= s.gate_gain;
    } else if c.kind == 1 {
        let m = 0.5 * (left + right);
        let p2 = if s.shimmer_phase + 0.5 >= 1.0 {
            s.shimmer_phase - 0.5
        } else {
            s.shimmer_phase + 0.5
        };
        let w1 = 1.0 - abs(2.0 * s.shimmer_phase - 1.0);
        let w2 = 1.0 - abs(2.0 * p2 - 1.0);
        s.shimmer_dc += 0.0142 * (m - s.shimmer_dc);
        shimmer[s.shimmer_write as usize] = m - s.shimmer_dc;
        s.shimmer_feedback = flush(
            (shimmer_tap(
                shimmer,
                s.shimmer_write,
                (1.0 - s.shimmer_phase) * 1024.0 + 2.0,
            ) * w1
                + shimmer_tap(shimmer, s.shimmer_write, (1.0 - p2) * 1024.0 + 2.0) * w2)
                * c.shimmer,
        );
        s.shimmer_phase += 1.0 / 1024.0;
        if s.shimmer_phase >= 1.0 {
            s.shimmer_phase -= 1.0;
        }
        s.shimmer_write += 1;
        if s.shimmer_write >= 2048 {
            s.shimmer_write = 0;
        }
        left += s.shimmer_feedback * 0.7;
        right += s.shimmer_feedback * 0.7;
    }
    [left, right]
}
