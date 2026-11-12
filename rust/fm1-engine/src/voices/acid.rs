// SPDX-License-Identifier: GPL-3.0-only
use super::{blep_saw, sine, sine_any, svf_mm};
use crate::effects::math::{clamp, exp2, flush, max, min, svf_coefs, tanh};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub ph: f32,
    pub inc: f32,
    pub inc_t: f32,
    pub amp: f32,
    pub amp_t: f32,
    pub env: f32,
    pub edec: f32,
    pub acc: f32,
    pub s: [f32; 4],
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub gate_left: i32,
    pub note: u8,
    pub sliding: u8,
    pub play: u8,
    pub ps: f32,
    pub p3: f32,
    pub p4: f32,
    pub p5: f32,
    pub lph: f32,
}
#[repr(C)]
pub struct Config {
    pub oscillator: u32,
    pub filter: i32,
    pub gain: f32,
    pub drive: f32,
    pub resonance: f32,
    pub envelope: f32,
    pub accent: f32,
    pub base: f32,
    pub k1: f32,
    pub k2: f32,
    pub input: f32,
    pub wow: f32,
    pub velocity: f32,
    pub glide: f32,
    pub bend: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub fm_ratio: f32,
    pub fm_index0: f32,
    pub fm_index1: f32,
    pub fm_rise: u32,
}
pub fn process(
    s: &mut State,
    c: &Config,
    control: &mut u32,
    table: &[f32; 1025],
    out: &mut [f32],
) -> f32 {
    let (mut ph, mut inc, mut amp, mut ps, mut p3, mut p4, mut p5) =
        (s.ph, s.inc, s.amp, s.ps, s.p3, s.p4, s.p5);
    let (mut ratio, mut pw, mut laser, mut input, mut level) = (1.0, 0.5, 1.0, c.input, 0.0);
    if amp < 1e-5 && s.amp_t <= 0.0 {
        s.amp = 0.0;
        out.fill(0.0);
        return 0.0;
    }
    for (j, output) in out.iter_mut().enumerate() {
        let due = *control & 31 == 0;
        *control = control.wrapping_add(1);
        if due {
            let oct = c.base
                + c.envelope * s.env * (1.0 + c.accent * s.acc)
                + 1.2 * c.accent * s.acc * s.env;
            let fc = clamp(60.0 * exp2(oct), 50.0, 14000.0);
            let wow = c.wow * s.acc * s.env;
            input = c.input + 2.0 * wow;
            s.c1 = svf_coefs(fc, c.k1 * (1.0 - 0.8 * wow));
            s.c2 = svf_coefs(fc * 1.15, c.k2);
            ratio = 1.6 + 2.6 * s.env;
            s.lph += 32.0 * 0.35 / 44100.0;
            if s.lph >= 1.0 {
                s.lph -= 1.0;
            }
            pw = 0.5 + 0.38 * sine(table, s.lph);
            laser = 1.0 + 7.0 * s.env * s.env;
            s.env *= s.edec;
        }
        if s.gate_left > 0 {
            s.gate_left -= 1;
            if s.gate_left == 0 {
                s.amp_t = 0.0;
            }
        }
        inc += (s.inc_t * c.bend - inc) * c.glide;
        let x = match c.oscillator {
            1 | 4 => {
                let mut p = ph + if c.oscillator == 4 { pw } else { 0.5 };
                if p >= 1.0 {
                    p -= 1.0;
                }
                0.7 * (blep_saw(ph, inc) - blep_saw(p, inc))
            }
            2 => {
                let x = blep_saw(ps, inc * ratio);
                ps += inc * ratio;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            3 => sine_any(table, ph + (0.2 + 2.2 * s.env) * sine(table, 2.0 * ph)) * 1.2,
            5 => {
                if ph < 0.5 {
                    0.75
                } else {
                    -0.75
                }
            }
            6 => {
                let x = blep_saw(ph, inc) + 0.7 * if ps < 0.5 { 1.0 } else { -1.0 };
                ps += inc * 0.5;
                if ps >= 1.0 {
                    ps -= 1.0;
                }
                x
            }
            7 => {
                let x = 0.5
                    * (blep_saw(ph, inc) + blep_saw(ps, inc * 1.0087) + blep_saw(p3, inc * 0.9913));
                ps += inc * 1.0087;
                p3 += inc * 0.9913;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                if p3 >= 1.0 {
                    p3 -= p3 as i32 as f32;
                }
                x
            }
            8 => sine(table, ph) * 1.3,
            9 => {
                let x = 0.75 * (blep_saw(ph, inc) + 0.75 * blep_saw(ps, inc * 1.49831));
                ps += inc * 1.49831;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            10 => {
                let x = sine_any(table, ph + (0.15 + 1.8 * s.env) * sine(table, ps)) * 1.2;
                ps += inc * 3.5;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            11 => {
                (sine(table, ph)
                    + 0.5 * sine(table, 2.0 * ph)
                    + 0.3 * sine(table, 3.0 * ph)
                    + 0.2 * sine(table, 4.0 * ph))
                    * 0.95
            }
            12 => {
                let x = sine(table, ps) * 1.25;
                ps += inc * laser;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            13 => sine_any(table, (0.3 + 1.2 * s.env) * sine(table, ph)) * 1.1,
            14 => {
                (if ph < 0.5 {
                    4.0 * ph - 1.0
                } else {
                    3.0 - 4.0 * ph
                }) * 1.2
            }
            15 => {
                let idx =
                    c.fm_index0 + c.fm_index1 * if c.fm_rise != 0 { 1.0 - s.env } else { s.env };
                let x = sine_any(table, ph + idx * sine(table, ps)) * 1.2;
                ps += inc * c.fm_ratio;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            16 | 19 => {
                let (r1, r2, r3, r4) = if c.oscillator == 16 {
                    (1.0139, 0.9863, 1.0278, 0.9728)
                } else {
                    (1.0285, 0.9720, 1.0581, 0.9460)
                };
                let x = 0.38
                    * (blep_saw(ph, inc)
                        + blep_saw(ps, inc * r1)
                        + blep_saw(p3, inc * r2)
                        + blep_saw(p4, inc * r3)
                        + blep_saw(p5, inc * r4));
                ps += inc * r1;
                p3 += inc * r2;
                p4 += inc * r3;
                p5 += inc * r4;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                if p3 >= 1.0 {
                    p3 -= p3 as i32 as f32;
                }
                if p4 >= 1.0 {
                    p4 -= p4 as i32 as f32;
                }
                if p5 >= 1.0 {
                    p5 -= p5 as i32 as f32;
                }
                x
            }
            17 => {
                let d = 0.5 - 0.46 * s.env;
                let pp = if ph < d {
                    ph * (0.5 / d)
                } else {
                    0.5 + (ph - d) * (0.5 / (1.0 - d))
                };
                let rs = sine_any(table, ph * (1.0 + 5.0 * s.env)) * (1.0 - ph);
                (sine_any(table, pp + 0.25) * 0.9 + 0.5 * rs) * 0.95
            }
            18 => {
                let x = 0.55 * (blep_saw(ph, inc) + blep_saw(ps, inc * 1.0041))
                    + 0.6 * if p3 < 0.4 { 1.0 } else { -1.0 };
                ps += inc * 1.0041;
                p3 += inc * 0.5;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                if p3 >= 1.0 {
                    p3 -= 1.0;
                }
                x
            }
            20 => {
                let f = 1.0 + 5.0 * (1.0 - s.env);
                let pp = if ph < 0.5 {
                    min(ph * f, 0.5)
                } else {
                    1.0 - min((1.0 - ph) * f, 0.5)
                };
                sine_any(table, pp - 0.25) * 1.05
            }
            21 => {
                (0.8 * sine_any(table, ph)
                    + 0.55 * s.env * sine_any(table, ph * 3.5 + 0.1) * (1.0 - ph))
                    * 1.05
            }
            22 => {
                let mut q = ph + pw;
                if q >= 1.0 {
                    q -= 1.0;
                }
                let x = 0.42 * (blep_saw(ph, inc) + blep_saw(ps, inc * 1.0041))
                    + 0.45 * (blep_saw(ph, inc) - blep_saw(q, inc));
                ps += inc * 1.0041;
                if ps >= 1.0 {
                    ps -= ps as i32 as f32;
                }
                x
            }
            _ => blep_saw(ph, inc),
        };
        ph += inc;
        if ph >= 1.0 {
            ph -= 1.0;
            if c.oscillator == 2 {
                ps = ph * ratio;
            }
        }
        let mut y = svf_mm(x * c.input, &s.c1, &mut s.s[..2], c.filter);
        y = tanh(y * (1.6 + 1.5 * c.resonance) * c.drive * input) / min(c.drive, 1.3);
        if c.filter <= 1 {
            y = svf_mm(y, &s.c2, &mut s.s[2..], c.filter);
        }
        amp += (s.amp_t - amp) * 0.006;
        *output =
            y * amp * c.velocity * c.gain * (c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse);
        level = max(level, amp);
    }
    for value in &mut s.s {
        *value = flush(*value);
    }
    s.ph = ph;
    s.ps = ps;
    s.p3 = p3;
    s.p4 = p4;
    s.p5 = p5;
    s.inc = inc;
    s.amp = amp;
    level * (0.6 + 0.4 * s.env)
}
