// SPDX-License-Identifier: GPL-3.0-only
use super::{blep_saw, sine, sine_any, svf_mm};
use crate::effects::math::{clamp, exp2, flush, min, svf_coefs, tanh};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub ph: [f32; 3],
    pub sph: f32,
    pub mph: f32,
    pub note: f32,
    pub target: f32,
    pub amp: f32,
    pub s: [f32; 4],
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub env: f32,
    pub gtarget: f32,
    pub midi_note: u8,
    pub rest: u8,
    pub env_up: u8,
    pub play: u8,
    pub velocity: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Line {
    pub env: f32,
    pub gate: f32,
    pub up: u8,
}
#[repr(C)]
pub struct Config {
    pub oscillator: u32,
    pub filter: i32,
    pub line: u32,
    pub playing: u32,
    pub drive: f32,
    pub k1: f32,
    pub k2: f32,
    pub input: f32,
    pub gain: f32,
    pub saw: f32,
    pub sub: f32,
    pub target: f32,
    pub rate: f32,
    pub fm_index: f32,
    pub pulse_width: f32,
    pub glide: f32,
    pub pitch: f32,
    pub bend: f32,
    pub tuning: f32,
    pub fm_ratio: f32,
    pub cutoff: f32,
    pub envelope: f32,
    pub motion: f32,
    pub arc: f32,
    pub break_pos: f32,
    pub break_neg: f32,
    pub duck: f32,
    pub links: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
}
#[allow(clippy::approx_constant)] // Preserve Metal's original 1.4142 ratio.
pub fn process(
    s: &mut State,
    c: &Config,
    line: &mut Line,
    control: &mut u32,
    table: &[f32; 1025],
    out: &mut [f32],
) -> f32 {
    let (mut p0, mut p1, mut p2, mut sp, mut mp, mut amp) =
        (s.ph[0], s.ph[1], s.ph[2], s.sph, s.mph, s.amp);
    if amp < 1e-5 && c.playing == 0 && s.play == 0 {
        s.amp = 0.0;
        out.fill(0.0);
        return 0.0;
    }
    s.note += (s.target - s.note) * min(out.len() as f32 * c.glide, 1.0);
    let f = 440.0 * c.tuning * exp2((s.note + c.pitch + c.bend - 69.0) * (1.0 / 12.0));
    let osc = c.oscillator;
    let i0 =
        f * (if osc == 8 {
            1.49831
        } else if osc == 1 || osc == 7 {
            0.9862
        } else {
            0.99481
        }) * (1.0 / 44100.0);
    let i1 = f * (1.0 / 44100.0);
    let i2 =
        f * (if osc == 1 || osc == 7 {
            1.0140
        } else {
            1.00521
        }) * (1.0 / 44100.0);
    let mi = if osc == 9 {
        i1 * 1.4142
    } else if osc == 11 {
        i1 * c.fm_ratio
    } else {
        i1 * (1.5 + 2.5 * s.env)
    };
    for (j, output) in out.iter_mut().enumerate() {
        let due = *control & 31 == 0;
        *control = control.wrapping_add(1);
        if due {
            let oct = c.cutoff + c.envelope * s.env + c.motion + c.arc
                - c.break_pos
                - c.break_neg
                - c.duck
                + c.links;
            let fc = clamp(f * 2.0 * exp2(oct), 45.0, 9000.0);
            s.c1 = svf_coefs(fc, c.k1);
            s.c2 = svf_coefs(fc * 1.3, c.k2);
        }
        let mut x = match osc {
            1 => (blep_saw(p0, i0) + blep_saw(p2, i2)) * c.saw * 1.35,
            2 => {
                let mut q = p1 + c.pulse_width;
                if q >= 1.0 {
                    q -= 1.0;
                }
                (blep_saw(p1, i1) - blep_saw(q, i1)) * c.saw * 1.4
            }
            3 => sine_any(table, p1 + c.fm_index * sine(table, p0)) * c.saw * 2.2,
            4 => {
                (sine(table, p1)
                    + 0.5 * sine(table, 2.0 * p1)
                    + 0.33 * sine(table, 3.0 * p1)
                    + 0.2 * sine(table, 4.0 * p1))
                    * c.saw
                    * 1.5
            }
            5 => (sine(table, p1) + 0.15 * sine(table, 2.0 * p1)) * c.saw * 1.8,
            6 => {
                let mut q = p1 + 0.5;
                if q >= 1.0 {
                    q -= 1.0;
                }
                (blep_saw(p1, i1) - blep_saw(q, i1)) * c.saw * 1.2
            }
            7 => {
                let (mut q0, mut q2) = (p0 + 0.3, p2 + 0.3);
                if q0 >= 1.0 {
                    q0 -= 1.0;
                }
                if q2 >= 1.0 {
                    q2 -= 1.0;
                }
                (blep_saw(p0, i0) - blep_saw(q0, i0) + blep_saw(p2, i2) - blep_saw(q2, i2))
                    * c.saw
                    * 0.9
                    + 0.5 * c.saw * (2.0 * sp - 1.0)
            }
            8 => (blep_saw(p1, i1) + 0.75 * blep_saw(p0, i0)) * c.saw * 1.1,
            9 => sine_any(table, p1 + (0.3 + 1.6 * s.env) * sine(table, mp)) * c.saw * 2.0,
            10 => blep_saw(mp, mi) * c.saw * 1.3,
            11 => sine_any(table, p1 + (0.25 + 1.5 * s.env) * sine(table, mp)) * c.saw * 2.0,
            _ => (blep_saw(p0, i0) + blep_saw(p1, i1) + blep_saw(p2, i2)) * c.saw,
        };
        x += c.sub * sine(table, sp);
        p0 += i0;
        p1 += i1;
        p2 += i2;
        sp += i1 * 0.5;
        mp += mi;
        if mp >= 1.0 {
            mp -= mp as i32 as f32;
        }
        if osc == 10 && p1 >= 1.0 {
            mp = (p1 - 1.0) * (mi / i1);
        }
        if p0 >= 1.0 {
            p0 -= 1.0;
        }
        if p1 >= 1.0 {
            p1 -= 1.0;
        }
        if p2 >= 1.0 {
            p2 -= 1.0;
        }
        if sp >= 1.0 {
            sp -= 1.0;
        }
        x = svf_mm(x * c.input, &s.c1, &mut s.s[..2], c.filter);
        if c.filter <= 1 {
            x = svf_mm(x, &s.c2, &mut s.s[2..], c.filter);
        }
        amp += (c.target - amp) * c.rate;
        if c.line != 0 {
            if line.up != 0 {
                line.env += 1.0 / (0.004 * 44100.0);
                if line.env >= 1.0 {
                    line.env = 1.0;
                    line.up = 0;
                }
            } else if line.gate > 0.0 {
                line.gate -= 1.0;
                line.env *= 0.99994785;
            } else {
                line.env *= 0.9948;
            }
            x *= line.env;
        }
        *output = tanh(x * c.drive)
            * amp
            * c.gain
            * (c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse);
    }
    for v in &mut s.s {
        *v = flush(*v);
    }
    s.ph = [p0, p1, p2];
    s.sph = sp;
    s.mph = mp;
    s.amp = amp;
    if c.line != 0 {
        amp * line.env
    } else {
        amp
    }
}
