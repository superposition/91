// SPDX-License-Identifier: GPL-3.0-only
use super::math::*;
pub const LOOKAHEAD: usize = 64;
pub const SUBBLOCKS: usize = 8;

/// Only scalar state crosses the scoped ABI; the fixed delay arrays stay in place.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub cur: f32,
    pub sum: f32,
    pub release: f32,
    pub envelope: f32,
    pub gain: f32,
    pub sub_min: f32,
    pub check: u32,
    pub index: u32,
    pub count: u32,
    pub sub_index: u32,
    pub threshold: f32,
    pub slope: f32,
    pub makeup: f32,
    pub attack: f32,
    pub decay: f32,
    pub ceiling: f32,
    pub reduction_hold: f32,
    pub peak_hold: f32,
}
const _: [(); 72] = [(); core::mem::size_of::<State>()];

pub struct Buffers<'a> {
    pub left: &'a mut [f32; LOOKAHEAD],
    pub right: &'a mut [f32; LOOKAHEAD],
    pub gains: &'a mut [f32; LOOKAHEAD],
    pub sub: &'a mut [f32; SUBBLOCKS],
}

pub fn set(state: &mut State, ratio: i32, threshold: i32, makeup: i32, ceiling: i32) {
    const RATIO: [f32; 7] = [1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 10.0];
    let r = ratio.clamp(0, 6) as usize;
    state.slope = if r != 0 { 1.0 - 1.0 / RATIO[r] } else { 0.0 };
    state.threshold = -30.0 + 0.3 * threshold as f32;
    state.makeup = db2lin(0.12 * makeup as f32);
    state.attack = 1.0 - exp(-1.0 / (0.004 * 44_100.0));
    state.decay = 1.0 - exp(-1.0 / ((0.06 + 0.04 * state.envelope) * 44_100.0));
    state.ceiling = db2lin(-12.0 + 0.12 * ceiling as f32);
}

pub fn reset(s: &mut State, b: Buffers<'_>) {
    b.left.fill(0.0);
    b.right.fill(0.0);
    b.gains.fill(1.0);
    b.sub.fill(1.0);
    s.cur = 1.0;
    s.release = 1.0;
    s.sub_min = 1.0;
    s.gain = 1.0;
    s.sum = 64.0;
    s.index = 0;
    s.count = 0;
    s.sub_index = 0;
    s.envelope = 0.0;
    s.reduction_hold = 0.0;
    s.peak_hold = 0.0;
    // Preserve the checksum phase and coefficient values, as legacy reset does.
}

// Match the original guard: unordered/NaN comparisons must reject the sample.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn process(s: &mut State, b: Buffers<'_>, left: &mut [f32], right: &mut [f32]) {
    assert_eq!(left.len(), right.len());
    const INV: [f32; 5] = [0.0, 1.0, 0.5, 0.33333334, 0.25];
    let mut j = 0;
    while j < left.len() {
        let m = (left.len() - j).min(4);
        let mut g1 = 1.0;
        let mut g0 = s.gain;
        if s.slope > 0.0 {
            let mut d = 0.0;
            let mut gr = 0.0;
            for i in 0..m {
                d = max(d, max(abs(left[j + i]), abs(right[j + i])));
            }
            let lv = 6.0206 * log2(d + 1e-9) - s.threshold;
            if lv > 3.0 {
                gr = s.slope * lv;
            } else if lv > -3.0 {
                gr = s.slope * (lv + 3.0) * (lv + 3.0) * (1.0 / 12.0);
            }
            s.envelope = flush(
                s.envelope
                    + (gr - s.envelope)
                        * ((if gr > s.envelope { s.attack } else { s.decay }) * m as f32),
            );
            g1 = db2lin(-s.envelope);
        } else {
            s.envelope = 0.0;
        }
        let gs = (g1 - g0) * INV[m];
        for i in 0..m {
            g0 += gs;
            let mut l = left[j + i] * g0 * s.makeup;
            let mut r = right[j + i] * g0 * s.makeup;
            if !(abs(l) < 64.0) {
                l = 0.0;
            }
            if !(abs(r) < 64.0) {
                r = 0.0;
            }
            let p = max(abs(l), abs(r));
            let gq = if p > s.ceiling { s.ceiling / p } else { 1.0 };
            let index = s.index as usize;
            b.left[index] = l;
            b.right[index] = r;
            s.cur = min(s.cur, gq);
            let avg = min(s.cur, s.sub_min);
            s.sum += avg - b.gains[index];
            b.gains[index] = avg;
            s.count += 1;
            if s.count >= 8 {
                s.count = 0;
                b.sub[s.sub_index as usize] = s.cur;
                s.sub_index = (s.sub_index + 1) & 7;
                s.cur = 1.0;
                let mut mn = b.sub[0];
                for k in 1..8 {
                    mn = min(mn, b.sub[k]);
                }
                s.sub_min = mn;
            }
            s.index = (s.index + 1) & 63;
            s.check = s.check.wrapping_add(1);
            if s.check & 1023 == 0 {
                let mut sm = 0.0;
                for k in 0..64 {
                    sm += b.gains[k];
                }
                s.sum = sm;
            }
            if !(s.sum > 0.0 && s.sum < 64.0 * 1.001 && s.release > 0.0 && s.release < 1.001) {
                b.gains.fill(1.0);
                s.sum = 64.0;
                s.release = 1.0;
                s.cur = 1.0;
                s.sub_min = 1.0;
            }
            let avg = s.sum * (1.0 / 64.0);
            if avg < s.release {
                s.release = avg;
            } else {
                s.release += (avg - s.release) * (1.0 / (0.06 * 44_100.0));
            }
            l = b.left[s.index as usize] * s.release;
            r = b.right[s.index as usize] * s.release;
            left[j + i] = l;
            right[j + i] = r;
            s.reduction_hold = flush(max(s.reduction_hold * 0.9998, 1.0 - s.release));
            s.peak_hold = flush(max(s.peak_hold * 0.9997, max(abs(l), abs(r))));
        }
        s.gain = g1;
        j += m;
    }
}
