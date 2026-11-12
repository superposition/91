// SPDX-License-Identifier: GPL-3.0-only
//! Effect settings and final integer-domain fades. Capture/dispatch stays outside
//! this module; all arithmetic retains the original binary32 evaluation order.
use super::math::*;

pub fn send(value: i32) -> f32 {
    (value * value) as f32 * 0.0001
}
pub fn size(value: i32) -> [f32; 2] {
    [
        0.6 + 0.375 * value as f32 * 0.01,
        0.5 + 0.32 * value as f32 * 0.01,
    ]
}
pub fn pump(value: i32) -> f32 {
    value as f32 * 0.01
}
pub fn chorus(value: i32, shed: u32) -> f32 {
    if shed >= 2 {
        0.0
    } else {
        value as f32 * 0.01 * 0.55
    }
}
pub fn duck_decay(step: f32) -> f32 {
    exp(-4.0 / (step * 3.0))
}
pub fn trim(half_db: i32) -> f32 {
    exp2(-0.0830482 * half_db as f32)
}
pub fn reduction(hold: f32) -> f32 {
    8.6859 * hold
}
pub fn hold(start: &mut [f32; 8], end: &[f32; 8], frames: u32) -> f32 {
    start.copy_from_slice(end);
    1.0 / frames as f32
}

/// [smoothed trim, master gain, fixed USB gain]. Smooth once per render call,
/// including zero-frame calls, exactly as the existing engine did.
pub fn gains(previous: f32, target: f32, muted: bool, q12: u32) -> [f32; 3] {
    let mut gain = if muted {
        0.0
    } else {
        q12 as f32 * (1.0 / 4096.0) * 2.0
    };
    let mut usb = if muted { 0.0 } else { 1.0 };
    let trim = previous + (target - previous) * 0.15;
    gain *= trim;
    usb *= trim;
    [trim, gain, usb]
}

/// Apply the gap pass before the hush pass, preserving the separate integer
/// truncation after each. Combining gains before conversion changes PCM.
pub fn fades(
    state: &mut [f32; 2],
    gap: bool,
    hush: bool,
    out: &mut [i32],
    usb: Option<&mut [i16]>,
) -> bool {
    assert_eq!(out.len() & 1, 0);
    let mut usb = usb;
    if let Some(b) = &usb {
        assert_eq!(out.len(), b.len());
    }
    if gap || state[0] < 1.0 {
        for j in 0..out.len() / 2 {
            state[0] = if gap {
                max(state[0] - 1.0 / (0.006 * 44100.0), 0.0)
            } else {
                min(state[0] + 1.0 / (0.001 * 44100.0), 1.0)
            };
            for ch in 0..2 {
                out[2 * j + ch] = (out[2 * j + ch] as f32 * state[0]) as i32;
            }
            if let Some(b) = &mut usb {
                for ch in 0..2 {
                    b[2 * j + ch] = (b[2 * j + ch] as f32 * state[0]) as i16;
                }
            }
        }
    }
    if hush || state[1] < 1.0 {
        let step = 1.0 / (0.006 * 44100.0);
        let target = if hush { 0.0 } else { 1.0 };
        for j in 0..out.len() / 2 {
            state[1] = if target > state[1] {
                min(state[1] + step, 1.0)
            } else {
                max(state[1] - step, 0.0)
            };
            for ch in 0..2 {
                out[2 * j + ch] = (out[2 * j + ch] as f32 * state[1]) as i32;
            }
            if let Some(b) = &mut usb {
                for ch in 0..2 {
                    b[2 * j + ch] = (b[2 * j + ch] as f32 * state[1]) as i16;
                }
            }
        }
    }
    hush && state[1] <= 0.0
}
