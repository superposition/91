// SPDX-License-Identifier: GPL-3.0-only
use super::{blep_saw, sine, sine_any};
use crate::effects::math::{clamp, cos, exp, exp2, flush, max, min, sin, svf_coefs};
use crate::sequencer::Rng;
use core::f32::consts::PI;
use core::f32::consts::TAU;
#[repr(C)]
pub struct BusConfig {
    pub sound_cut: f32,
    pub bright: f32,
    pub arc: f32,
    pub filter_mod: f32,
    pub link_amount: f32,
    pub link_env: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub start: u32,
}
pub struct Bus<'a> {
    pub left: &'a [f32],
    pub right: &'a [f32],
    pub out_left: &'a mut [f32],
    pub out_right: &'a mut [f32],
    pub dry: &'a mut [f32; 512],
}
pub fn bus(state: &mut [f32; 4], c: &BusConfig, b: Bus<'_>) {
    let frequency = clamp(
        700.0
            * c.sound_cut
            * exp2(
                c.bright * 0.03
                    + 0.6 * c.arc
                    + 1.5 * c.filter_mod
                    + 3.0 * c.link_amount * 0.01 * c.link_env,
            ),
        200.0,
        12000.0,
    );
    let coefficient = 1.0 - exp(-TAU * frequency * (1.0 / 44100.0));
    for j in 0..b.left.len() {
        state[0] += coefficient * (b.left[j] - state[0]);
        state[1] += coefficient * (state[0] - state[1]);
        state[2] += coefficient * (b.right[j] - state[2]);
        state[3] += coefficient * (state[2] - state[3]);
        let gain = c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse;
        let l = state[1] * gain;
        let r = state[3] * gain;
        b.out_left[j] = l * 0.8;
        b.out_right[j] = r * 0.8;
        b.dry[c.start.wrapping_add(j as u32) as usize & 511] = l + r;
    }
    for value in state {
        *value = flush(*value);
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub ph: [f32; 4],
    pub inc: [f32; 4],
    pub sub: f32,
    pub note: f32,
    pub target: f32,
    pub t: f32,
    pub rate: f32,
    pub peak: f32,
    pub amp: f32,
    pub fa: f32,
    pub fh: f32,
    pub fr: f32,
    pub ic: [f32; 4],
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub vib: f32,
    pub pan: f32,
    pub hf: f32,
    pub midi_note: u8,
    pub hand: u8,
    pub voices: u8,
    pub play: u8,
    pub pnote: u8,
    pub pord: u32,
    pub pamp: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Runtime {
    pub gain: f32,
    pub stab_env: f32,
    pub controls: [u32; 4],
}
#[repr(C)]
pub struct Breath {
    pub attack: f32,
    pub hold: f32,
    pub release: f32,
    pub rest: f32,
    pub playing: u32,
    pub manual: u32,
    pub oscillator: u32,
}
#[repr(C)]
pub struct Config {
    pub breath: Breath,
    pub gain: f32,
    pub gate: f32,
    pub spread: f32,
    pub motion: f32,
    pub modulation: [f32; 6],
    pub pitch: f32,
    pub bright: f32,
    pub arc: f32,
    pub filter_mod: f32,
    pub panning: f32,
    pub sound_cut: f32,
    pub tuning: f32,
    pub stab_decay: f32,
    pub swell: f32,
    pub breath_scale: [f32; 4],
    pub vowels: [[f32; 2]; 4],
    pub stabs: u32,
    pub dx: u32,
    pub filter: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Publication {
    pub mask: u32,
    pub notes: [u8; 4],
    pub amplitudes: [f32; 4],
    pub level: f32,
}
pub fn breathe(
    v: &mut State,
    c: &Breath,
    scale: f32,
    rng: &mut Rng,
    midi: &mut impl FnMut(u8, u8, u8),
) {
    let mut secs = c.attack + c.hold + c.release + c.rest;
    secs = max(secs, 0.3);
    v.fa = max(c.attack / secs, 0.002);
    v.fh = c.hold / secs;
    v.fr = max(c.release / secs, 0.002);
    secs *= scale * (0.85 + 0.3 * rng.fraction());
    v.rate = 32.0 / (secs * 44100.0);
    v.peak = 0.6 + 0.4 * rng.fraction();
    if c.playing != 0 && v.midi_note != 0 {
        midi(0x80, v.midi_note, 0);
    }
    v.note = v.target;
    v.voices = c.oscillator as u8;
    v.midi_note = v.target as u8;
    if c.playing != 0 && c.manual == 0 {
        midi(0x90, v.midi_note, (50.0 + 40.0 * v.peak) as u8);
    }
}
fn envelope(v: &State, mut t: f32) -> f32 {
    if t < v.fa {
        return 0.5 - 0.5 * cos(PI * t / v.fa);
    }
    t -= v.fa;
    if t < v.fh {
        return 1.0;
    }
    t -= v.fh;
    if t < v.fr {
        0.5 + 0.5 * cos(PI * t / v.fr)
    } else {
        0.0
    }
}
fn control(
    v: &mut State,
    c: &Config,
    index: usize,
    rng: &mut Rng,
    midi: &mut impl FnMut(u8, u8, u8),
) {
    const HAND: f32 = 32.0 / (1.0 * 44100.0);
    if v.hand == 1 {
        v.hf -= HAND;
        if v.hf <= 0.0 {
            v.hf = 0.0;
            v.hand = 2;
            v.t = 0.15;
            breathe(v, &c.breath, c.breath_scale[index], rng, midi);
        }
    } else if v.hand == 2 {
        v.hf += HAND * 0.6;
        if v.hf >= 1.0 {
            v.hf = 1.0;
            v.hand = 0;
        }
    }
    if v.play != 0 {
        let release = clamp(c.breath.release * 0.35, 0.08, 3.0);
        let attack = clamp(c.breath.attack * 0.12, 0.006, 2.0);
        if v.play == 1 {
            v.pamp = min(v.pamp + 32.0 / (attack * 44100.0), 1.0);
        } else {
            v.pamp -= 32.0 / (release * 44100.0);
            if v.pamp <= 0.0 {
                v.pamp = 0.0;
                v.play = 0;
                v.note = v.target;
                v.hand = 0;
                v.hf = 1.0;
                v.t = 0.0;
                if c.breath.manual == 0 {
                    breathe(v, &c.breath, c.breath_scale[index], rng, midi);
                }
            }
        }
    } else if c.stabs == 0 && c.breath.manual == 0 {
        v.t += v.rate;
        if v.t >= 1.0 {
            v.t -= 1.0;
            breathe(v, &c.breath, c.breath_scale[index], rng, midi);
        }
    }
    v.vib += (4.6 + 0.4 * index as f32) * 32.0 / 44100.0;
    if v.vib >= 1.0 {
        v.vib -= 1.0;
    }
    let f = 440.0
        * c.tuning
        * exp2((v.note + 0.08 * c.motion * c.modulation[index] + c.pitch - 69.0) * (1.0 / 12.0));
    v.inc[0] = f * (1.0 - c.spread) * (1.0 / 44100.0);
    v.inc[1] = f * (1.0 + c.spread) * (1.0 / 44100.0);
    v.inc[2] = f * (1.0 - 0.45 * c.spread) * (1.0 / 44100.0);
    v.inc[3] = f * (1.0 + 0.45 * c.spread) * (1.0 / 44100.0);
    let mut vw = clamp(
        c.bright * 0.01
            + 0.25 * c.arc
            + 0.2 * c.motion * c.modulation[(index + 2) % 6]
            + 0.4 * c.filter_mod,
        0.0,
        1.0,
    ) * 2.999;
    let a = vw as usize;
    vw -= a as f32;
    let f1 = c.vowels[a][0] + (c.vowels[a + 1][0] - c.vowels[a][0]) * vw;
    let f2 = c.vowels[a][1] + (c.vowels[a + 1][1] - c.vowels[a][1]) * vw;
    v.c1 = svf_coefs(f1 * (1.0 + 0.03 * index as f32), 0.45);
    v.c2 = svf_coefs(f2 * (1.0 - 0.02 * index as f32), 0.3);
}
fn pick(x: f32, v1: f32, v2: f32, k: f32, kind: i32) -> f32 {
    match kind {
        1 => x - k * v1 - v2,
        2 => k * v1,
        3 => x - k * v1,
        4 => 2.0 * v2 - x + k * v1,
        _ => v2,
    }
}
#[allow(clippy::too_many_arguments)]
pub fn process(
    voices: &mut [State; 4],
    rt: &mut Runtime,
    c: &Config,
    rng: &mut Rng,
    table: &[f32; 1025],
    left: &mut [f32],
    right: &mut [f32],
    midi: &mut impl FnMut(u8, u8, u8),
) -> Publication {
    left.fill(0.0);
    right.fill(0.0);
    let n = left.len();
    rt.gain += (c.gate - rt.gain)
        * min(
            n as f32 / ((if c.gate > 0.0 { 2.0 } else { 5.0 }) * 44100.0),
            1.0,
        );
    let mut result = Publication::default();
    for (i, v) in voices.iter_mut().enumerate() {
        let (mut p0, mut p1, mut p2, mut p3, mut ps, mut amp) =
            (v.ph[0], v.ph[1], v.ph[2], v.ph[3], v.sub, v.amp);
        let (mut s0, mut s1, mut s2, mut s3) = (v.ic[0], v.ic[1], v.ic[2], v.ic[3]);
        let (mut target, mut idx, mut se, mut rate) = (0.0, 0.0, rt.stab_env, 0.01);
        let saw = matches!(v.voices, 2 | 6 | 7);
        if amp < 1e-5 && rt.gain < 1e-4 && v.play == 0 {
            v.amp = 0.0;
            continue;
        }
        let pan = clamp(
            v.pan
                + 0.35 * c.motion * c.modulation[(i + 3) % 6]
                + c.panning * if i & 1 != 0 { 1.0 } else { -1.0 },
            -1.0,
            1.0,
        );
        let gl = cos((pan + 1.0) * (PI / 4.0)) * c.gain;
        let gr = sin((pan + 1.0) * (PI / 4.0)) * c.gain;
        for j in 0..n {
            let due = rt.controls[i] & 31 == 0;
            rt.controls[i] = rt.controls[i].wrapping_add(1);
            if due {
                control(v, c, i, rng, midi);
                rate = 0.01;
                let breath = if v.play != 0 {
                    target = v.pamp * 0.8;
                    v.pamp
                } else if c.breath.manual != 0 {
                    target = 0.0;
                    rate = 0.0004;
                    0.0
                } else if c.stabs != 0 {
                    let b = if c.dx != 0 {
                        if i < 2 {
                            c.swell
                        } else {
                            0.0
                        }
                    } else {
                        se
                    };
                    se *= c.stab_decay;
                    target = b * 0.85 * rt.gain;
                    b
                } else {
                    let b = envelope(v, v.t);
                    target = b * v.peak * rt.gain * v.hf;
                    b
                };
                idx = (0.04 + 0.1 * c.bright * 0.01 + 0.1 * breath) * (1.0 + 0.4 * c.arc);
                if saw {
                    let fc = 300.0
                        * c.sound_cut
                        * exp2(3.3 * breath + 0.02 * c.bright + 1.5 * c.filter_mod + 0.5 * c.arc);
                    v.c1 = svf_coefs(fc, 0.5);
                    v.c2 = svf_coefs(fc * 1.3, 1.0);
                }
            }
            amp += (target - amp) * rate;
            let mut x = match v.voices {
                0 => {
                    let m = sine(table, p0) * idx;
                    (sine(table, p0 + m + 1.0) + 0.12 * sine(table, 2.0 * p0)) * 1.2
                }
                2 => {
                    0.36 * (blep_saw(p0, v.inc[0])
                        + blep_saw(p1, v.inc[1])
                        + blep_saw(p2, v.inc[2])
                        + blep_saw(p3, v.inc[3]))
                }
                3 => {
                    let m = sine(table, p1) * (0.25 + 4.0 * idx);
                    (sine_any(table, p0 + m) + 0.25 * sine(table, 2.0 * p0)) * 1.1
                }
                4 => {
                    (0.62 * sine(table, p0)
                        + 0.38 * sine(table, 2.0 * p0)
                        + 0.26 * sine(table, 3.0 * p0)
                        + 0.16 * sine(table, 4.0 * p0)
                        + 0.2 * sine(table, p1))
                        * 1.05
                }
                5 => {
                    ((if p0 < 0.5 {
                        4.0 * p0 - 1.0
                    } else {
                        3.0 - 4.0 * p0
                    }) + (if p1 < 0.5 {
                        4.0 * p1 - 1.0
                    } else {
                        3.0 - 4.0 * p1
                    })) * 0.7
                        + 0.5 * sine(table, p0)
                }
                6 => {
                    let mut po = 2.0 * p0;
                    po -= po as i32 as f32;
                    0.3 * (blep_saw(p0, v.inc[0])
                        + blep_saw(p1, v.inc[1])
                        + blep_saw(p2, v.inc[2])
                        + blep_saw(p3, v.inc[3]))
                        + 0.3 * blep_saw(po, 2.0 * v.inc[0])
                }
                7 => {
                    let mut q = p1 + 0.5;
                    if q >= 1.0 {
                        q -= 1.0;
                    }
                    0.55 * (blep_saw(p0, v.inc[0]) - blep_saw(q, v.inc[1])) + 0.3 * sine(table, p0)
                }
                8 => {
                    (sine(table, p0)
                        + 0.33 * sine(table, 3.0 * p0)
                        + 0.2 * sine(table, 5.0 * p0)
                        + 0.14 * sine(table, 7.0 * p0))
                        * 1.15
                }
                9 => (sine(table, p0) + 0.18 * sine(table, 2.0 * p0)) * 1.3,
                _ => {
                    let x = (if p0 < 0.5 {
                        4.0 * p0 - 1.0
                    } else {
                        3.0 - 4.0 * p0
                    }) * 1.4
                        + 0.45 * blep_saw(p0, v.inc[0])
                        + 0.6 * sine(table, p0);
                    let v3 = x - s1;
                    let v1 = v.c1[0] * s0 + v.c1[1] * v3;
                    let v2 = s1 + v.c1[1] * s0 + v.c1[2] * v3;
                    s0 = 2.0 * v1 - s0;
                    s1 = 2.0 * v2 - s1;
                    let b1 = v1 * v.c1[3];
                    let l1 = v2;
                    let v3 = x - s3;
                    let v1 = v.c2[0] * s2 + v.c2[1] * v3;
                    let v2 = s3 + v.c2[1] * s2 + v.c2[2] * v3;
                    s2 = 2.0 * v1 - s2;
                    s3 = 2.0 * v2 - s3;
                    let b2 = v1 * v.c2[3];
                    b1 + 0.5 * b2 + 0.45 * l1
                }
            };
            if saw {
                x += 0.15 * sine(table, ps);
                let v3 = x - s1;
                let v1 = v.c1[0] * s0 + v.c1[1] * v3;
                let v2 = s1 + v.c1[1] * s0 + v.c1[2] * v3;
                s0 = 2.0 * v1 - s0;
                s1 = 2.0 * v2 - s1;
                let y1 = pick(x, v1, v2, v.c1[3], c.filter);
                let v3 = y1 - s3;
                let v1 = v.c2[0] * s2 + v.c2[1] * v3;
                let v2 = s3 + v.c2[1] * s2 + v.c2[2] * v3;
                s2 = 2.0 * v1 - s2;
                s3 = 2.0 * v2 - s3;
                x = (if c.filter <= 1 {
                    pick(y1, v1, v2, v.c2[3], c.filter)
                } else {
                    y1
                }) * 1.1;
                p2 += v.inc[2];
                if p2 >= 1.0 {
                    p2 -= 1.0;
                }
                p3 += v.inc[3];
                if p3 >= 1.0 {
                    p3 -= 1.0;
                }
                ps += 0.5 * v.inc[0];
                if ps >= 1.0 {
                    ps -= 1.0;
                }
            }
            p0 += v.inc[0];
            if p0 >= 1.0 {
                p0 -= 1.0;
            }
            p1 += if v.voices == 3 {
                v.inc[0] * 3.5
            } else {
                v.inc[1]
            };
            if p1 >= 1.0 {
                p1 -= p1 as i32 as f32;
            }
            x *= amp;
            left[j] += x * gl;
            right[j] += x * gr;
        }
        v.ic = [flush(s0), flush(s1), flush(s2), flush(s3)];
        v.amp = amp;
        v.ph = [p0, p1, p2, p3];
        v.sub = ps;
        result.mask |= 1 << i;
        result.notes[i] = v.note as u8;
        result.amplitudes[i] = amp;
        result.level = max(result.level, amp);
    }
    for _ in 0..(n + 31) >> 5 {
        rt.stab_env *= c.stab_decay;
    }
    rt.stab_env = flush(rt.stab_env);
    result
}
