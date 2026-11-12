// SPDX-License-Identifier: GPL-3.0-only
//! Allocation-free effects over caller-owned state and fixed delay storage.
pub mod bus;
pub mod chorus;
pub mod control;
#[rustfmt::skip]
pub mod control_abi;
pub mod echo;
pub mod engage;
pub mod master;
pub mod math;
pub mod mix;
#[rustfmt::skip]
mod ids;
pub mod reverb;
pub mod sampler;
pub mod tone;
pub mod vinyl;

pub fn mixer_gains(pan: i32, send: i32) -> [f32; 3] {
    let p = pan as f32 * 0.01;
    [
        math::min(1.0, 2.0 * (1.0 - p)),
        math::min(1.0, 2.0 * p),
        send as f32 * 0.02,
    ]
}

pub fn mixer_end(mix: &mut [f32], before: &[f32], gain: f32) {
    assert_eq!(mix.len(), before.len());
    for (m, b) in mix.iter_mut().zip(before) {
        *m = *b + (*m - *b) * gain;
    }
}

pub fn output(
    left: &[f32],
    right: &[f32],
    out: &mut [i32],
    usb: Option<&mut [i16]>,
    gain: f32,
    usb_gain: f32,
    shed: bool,
) {
    assert_eq!(left.len(), right.len());
    assert_eq!(out.len(), left.len() * 2);
    for j in 0..left.len() {
        let clip = if shed { math::soft_sat } else { math::tanh };
        out[2 * j] = (clip(left[j] * gain) * 8_300_000.0) as i32;
        out[2 * j + 1] = (clip(right[j] * gain) * 8_300_000.0) as i32;
    }
    if let Some(usb) = usb {
        assert_eq!(usb.len(), left.len() * 2);
        for j in 0..left.len() {
            usb[2 * j] = (math::tanh(left[j] * usb_gain) * 32_421.0) as i16;
            usb[2 * j + 1] = (math::tanh(right[j] * usb_gain) * 32_421.0) as i16;
        }
    }
}
