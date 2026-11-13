// SPDX-License-Identifier: GPL-3.0-only
//! Allocation-free effects over caller-owned state and fixed delay storage.
pub mod bus;
pub mod chorus;
pub mod config;
pub mod control;
#[rustfmt::skip]
pub mod control_abi;
pub mod echo;
pub mod engage;
pub mod grid;
pub mod master;
pub mod math;
pub mod mix;
#[rustfmt::skip]
pub mod ids;
pub mod render;
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
        let words = output_frame(left[j], right[j], gain, shed);
        out[2 * j] = words[0];
        out[2 * j + 1] = words[1];
    }
    if let Some(usb) = usb {
        assert_eq!(usb.len(), left.len() * 2);
        for j in 0..left.len() {
            let words = usb_frame(left[j], right[j], usb_gain);
            usb[2 * j] = words[0];
            usb[2 * j + 1] = words[1];
        }
    }
}

/// Quantized values, not a reference to the final DMA allocation.
pub fn output_frame(left: f32, right: f32, gain: f32, shed: bool) -> [i32; 2] {
    let clip = if shed { math::soft_sat } else { math::tanh };
    [
        (clip(left * gain) * 8_300_000.0) as i32,
        (clip(right * gain) * 8_300_000.0) as i32,
    ]
}
pub fn usb_frame(left: f32, right: f32, gain: f32) -> [i16; 2] {
    [
        (math::tanh(left * gain) * 32_421.0) as i16,
        (math::tanh(right * gain) * 32_421.0) as i16,
    ]
}
