// SPDX-License-Identifier: GPL-3.0-only
//! Track punch-in filter/phaser, applied before echo sends and the master.
use super::math::*;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub filter: [f32; 4],
    pub feedback: [f32; 2],
    pub stages: [[f32; 4]; 2],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Config {
    pub filter: [f32; 4],
    pub phase: [f32; 2],
    pub filter_mix: f32,
    pub phase_mix: f32,
    pub wet: f32,
    pub kind: u32,
}
const _: [(); 56] = [(); core::mem::size_of::<State>()];
const _: [(); 40] = [(); core::mem::size_of::<Config>()];
pub struct Buffers<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
    pub mix_left: &'a mut [f32],
    pub mix_right: &'a mut [f32],
}
pub fn process(s: &mut State, c: &Config, b: Buffers<'_>) {
    let n = b.left.len();
    assert_eq!(b.right.len(), n);
    assert_eq!(b.mix_left.len(), n);
    assert_eq!(b.mix_right.len(), n);
    for j in 0..n {
        let mut x = [b.left[j], b.right[j]];
        for (ch, x) in x.iter_mut().enumerate() {
            if c.filter_mix > 0.0 {
                let v3 = *x - s.filter[ch * 2 + 1];
                let v1 = c.filter[0] * s.filter[ch * 2] + c.filter[1] * v3;
                let v2 = s.filter[ch * 2 + 1] + c.filter[1] * s.filter[ch * 2] + c.filter[2] * v3;
                s.filter[ch * 2] = 2.0 * v1 - s.filter[ch * 2];
                s.filter[ch * 2 + 1] = 2.0 * v2 - s.filter[ch * 2 + 1];
                let y = match c.kind {
                    1 => *x - c.filter[3] * v1 - v2,
                    2 => c.filter[3] * v1,
                    3 => *x - c.filter[3] * v1,
                    4 => 2.0 * v2 - *x + c.filter[3] * v1,
                    _ => v2,
                };
                *x += (y - *x) * c.filter_mix;
            }
            if c.phase_mix > 0.0 {
                let mut y = *x + 0.5 * s.feedback[ch];
                let a = c.phase[ch];
                for stage in &mut s.stages[ch] {
                    let v = a * y + *stage;
                    *stage = y - a * v;
                    y = v;
                }
                s.feedback[ch] = flush(y);
                *x += c.wet * (y - *x) * c.phase_mix;
            }
        }
        b.left[j] = x[0];
        b.right[j] = x[1];
        b.mix_left[j] += x[0];
        b.mix_right[j] += x[1];
    }
    for x in &mut s.filter {
        *x = flush(*x);
    }
}
