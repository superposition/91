// SPDX-License-Identifier: GPL-3.0-only
//! Exact binary32 operation order from firmware/src/dsp/fastmath.h.
#![allow(clippy::excessive_precision, clippy::approx_constant)]
pub fn abs(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}
pub fn min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}
pub fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}
pub fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
pub fn flush(x: f32) -> f32 {
    if abs(x) < 1e-20 {
        0.0
    } else {
        x
    }
}
pub fn floor(x: f32) -> f32 {
    let f = x as i32 as f32;
    if f > x {
        f - 1.0
    } else {
        f
    }
}
fn exp2_nf(n: i32, f: f32) -> f32 {
    if n < -126 {
        return 0.0;
    }
    let mut p = 1.534580891e-4;
    p = p * f + 1.339993158e-3;
    p = p * f + 9.618488972e-3;
    p = p * f + 5.550328776e-2;
    p = p * f + 2.402264689e-1;
    p = p * f + 6.931472057e-1;
    p = p * f + 1.0;
    f32::from_bits(p.to_bits().wrapping_add((n as u32) << 23))
}
pub fn exp2(mut x: f32) -> f32 {
    if x < -126.0 {
        return 0.0;
    }
    if x > 127.0 {
        x = 127.0;
    }
    let fl = floor(x + 0.5);
    exp2_nf(fl as i32, x - fl)
}
pub fn exp(mut x: f32) -> f32 {
    if x < -87.3 {
        return 0.0;
    }
    if x > 88.7 {
        x = 88.7;
    }
    let n = floor(x * 1.44269504088896 + 0.5);
    x = (x - n * 0.693145752) - n * 1.42860677e-6;
    exp2_nf(n as i32, x * 1.44269504088896)
}
pub fn log2(x: f32) -> f32 {
    let bits = x.to_bits();
    if bits as i32 <= 0 {
        return -127.0;
    }
    let mut e = ((bits >> 23) & 255) as i32 - 127;
    let mut m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    if m > 1.41421356 {
        m *= 0.5;
        e += 1;
    }
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let mut p = 3.428278367e-1;
    p = p * t2 + 4.115337810e-1;
    p = p * t2 + 5.770866525e-1;
    p = p * t2 + 9.617966483e-1;
    p = p * t2 + 2.885390082;
    e as f32 + p * t
}
pub fn db2lin(db: f32) -> f32 {
    exp2(db * 0.166096405)
}
pub fn tanh(x: f32) -> f32 {
    let a = abs(x);
    if a < 0.0004 {
        return x;
    }
    if a < 0.5 {
        let z = x * x;
        return x
            * (1.0
                + z * (-0.333333333
                    + z * (0.133333333
                        + z * (-0.0539682540
                            + z * (0.0218694885 + z * (-0.00886323552 + z * 0.00359212803))))));
    }
    if a > 9.0 {
        return if x > 0.0 { 1.0 } else { -1.0 };
    }
    let e = exp2(2.0 * 1.44269504088896 * a);
    let r = 1.0 - 2.0 / (e + 1.0);
    if x > 0.0 {
        r
    } else {
        -r
    }
}
pub fn soft_sat(x: f32) -> f32 {
    let x = clamp(x, -3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}
pub fn sin_pi(x: f32) -> f32 {
    let x2 = x * x;
    let mut p = 1.345176048e-10;
    p = p * x2 - 2.467706523e-8;
    p = p * x2 + 2.752946668e-6;
    p = p * x2 - 1.984015614e-4;
    p = p * x2 + 8.333310417e-3;
    p = p * x2 - 1.666666459e-1;
    p = p * x2 + 9.999999946e-1;
    p * x
}
pub fn sin_turns(phase: f32) -> f32 {
    sin_pi((phase - floor(phase + 0.5)) * 6.28318530717959)
}
pub fn sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut y = f32::from_bits(0x5f375a86 - (x.to_bits() >> 1));
    y = y * (1.5 - 0.5 * x * y * y);
    y = y * (1.5 - 0.5 * x * y * y);
    y = y * (1.5 - 0.5 * x * y * y);
    x * y
}
fn wrap_pi(x: f32) -> f32 {
    let k = floor(x * (1.0 / 6.28318530717959) + 0.5);
    ((x - k * 6.28125) - k * 1.9353071693331003e-3) - k * 1.0253131561401567e-11
}
pub fn sin(x: f32) -> f32 {
    sin_pi(wrap_pi(x))
}
pub fn cos(x: f32) -> f32 {
    let mut r = wrap_pi(x) + 0.5 * 3.14159265358979;
    if r >= 3.14159265358979 {
        r -= 6.28318530717959;
    }
    sin_pi(r)
}
pub fn tan(x: f32) -> f32 {
    let a = abs(x);
    let c = (1.57079637 - a) - 4.37113883e-8;
    sin_pi(x) / sin_pi(c)
}
pub fn svf_coefs(frequency: f32, k: f32) -> [f32; 4] {
    let g = tan(3.14159265358979 * clamp(frequency, 40.0, 16000.0) * (1.0 / 44100.0));
    let a = 1.0 / (1.0 + g * (g + k));
    let b = g * a;
    [a, b, g * b, k]
}
pub fn svf_lp(x: f32, c: &[f32; 4], state: &mut [f32]) -> f32 {
    let v3 = x - state[1];
    let v1 = c[0] * state[0] + c[1] * v3;
    let v2 = state[1] + c[1] * state[0] + c[2] * v3;
    state[0] = 2.0 * v1 - state[0];
    state[1] = 2.0 * v2 - state[1];
    v2
}
