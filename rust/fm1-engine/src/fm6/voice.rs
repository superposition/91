// SPDX-License-Identifier: GPL-3.0-only
//! A full 64-frame FM stage, preserving slot traversal and bass-last output.
use super::{envelopes, lfo, operators, plan, scaling, tables::*, types::*, *};
use crate::effects::math::{exp, max};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Config {
    pub jam: u32,
    pub current: i32,
    pub engine: u32,
    pub float_path: u32,
    pub tune: i32,
    pub bend: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AmsCache {
    pub amd: u32,
    pub points: [u32; 4],
    pub valid: u8,
}
pub struct Programs<'a> {
    pub keys: &'a [u8; 155],
    pub bass: &'a [u8; 155],
}
struct Control<'a> {
    program: &'a [u8; 155],
    lfo: &'a Lfo,
    config: &'a Config,
    cache: &'a mut AmsCache,
}
fn control(x: &mut Slot, c: Control<'_>) {
    let s = &mut x.s;
    let ed = c.program;
    let algorithm = (ed[ALG] & 31) as u32;
    let mut dead = true;
    let mut levels = [0; 6];
    let mut lfo = c.lfo.val;
    let delay = c.lfo.depth;
    let pmd = (ed[LPMD] as u32 * 165) >> 6;
    if x.gate == 0 && s.down != 0 {
        s.down = 0;
        for k in 0..6 {
            envelopes::advance(
                &mut s.eg[k],
                &ed[k * OP..(k + 1) * OP],
                s.ol[k] as i32,
                s.rs[k] as i32,
                3,
            );
        }
        envelopes::pitch_advance(&mut s.pe, 3);
    }
    let sensitivity = FM6_PMS[ed[LPMS] as usize] as i32 * (lfo - (1 << 23));
    let mut pitch =
        (pmd as f32 * delay as f32 * sensitivity as f32 * (1.0 / 549755813888.0)) as i32;
    pitch = if pitch < 0 { -pitch } else { pitch };
    pitch = if sensitivity < 0 { -pitch } else { pitch };
    pitch += c.config.tune + c.config.bend;
    lfo = (1 << 24) - lfo;
    let amplitude =
        (((ed[LAMD] as u32 * 165) >> 6) as f32 * delay as f32 * lfo as f32 * (1.0 / 4294967296.0))
            as i32;
    let amd = amplitude.max(0);
    pitch += envelopes::pitch_step(&mut s.pe, s.down != 0);
    for (k, output_level) in levels.iter_mut().enumerate() {
        let op = &ed[k * OP..(k + 1) * OP];
        let frequency = s.base[k] + if op[MODE] != 0 { c.config.tune } else { pitch };
        if s.freq_valid >> k & 1 == 0 || s.freq_log[k] != frequency {
            s.freq_log[k] = frequency;
            s.fq[k] = scaling::frequency(frequency);
            s.freq_valid |= 1 << k;
        }
        let mut level = envelopes::step(
            &mut s.eg[k],
            op,
            s.ol[k] as i32,
            s.rs[k] as i32,
            s.down != 0,
        );
        if FM6_AMS[op[AMS] as usize] != 0 {
            let a = op[AMS] as usize;
            if c.cache.amd != amd as u32 {
                c.cache.amd = amd as u32;
                c.cache.valid = 0;
            }
            if c.cache.valid >> a & 1 == 0 {
                let scaled = (amd as f32 * (FM6_AMS[a] as f32 * (1.0 / 16777216.0))) as u32;
                c.cache.points[a] = if scaled != 0 {
                    exp(scaled as f32 * (0.07 / 262144.0) + 12.2) as u32
                } else {
                    198789
                };
                c.cache.valid |= 1 << a;
            }
            level -= (level as f32 * (c.cache.points[a] as f32 * (1.0 / 16777216.0))) as i32;
        }
        *output_level = level;
    }
    for k in 0..6 {
        let e = &s.eg[k];
        if (scaling::carrier(algorithm, 6 - k as u32)
            || (FM6_ALG[algorithm as usize][k] & 0xc0 == 0xc0 && ed[FB] != 0))
            && (s.down != 0
                || ed[k * OP + L1 + 3] > 0
                || e.level >= SILENT
                || e.ix < 3
                || (e.ix == 3 && e.rising != 0))
        {
            dead = false;
        }
    }
    s.quiet = if dead {
        s.quiet + u8::from(s.quiet < 2)
    } else {
        0
    };
    let feedback = ed[FB];
    plan::prepare(
        s,
        &levels,
        algorithm,
        c.config.engine,
        if feedback != 0 {
            8 - feedback as u32
        } else {
            16
        },
    );
}
pub fn block(
    pool: pool::Pool<'_>,
    programs: Programs<'_>,
    c: &Config,
    cache: &mut AmsCache,
    t: &Lookup<'_>,
    scratch: &mut Scratch<'_>,
    output: &mut [f32; 64],
) {
    output.fill(0.0);
    if c.current < 0 {
        return;
    }
    lfo::step(pool.keys_lfo, programs.keys, t);
    if c.jam != 0 {
        lfo::step(pool.bass_lfo, programs.bass, t);
    }
    for k in 0..6 {
        let index = if c.jam != 0 {
            if k == 4 {
                5
            } else if k == 5 {
                4
            } else {
                k
            }
        } else {
            k
        };
        let x = &mut pool.slots[index];
        if x.used == 0 {
            if c.jam != 0 && index == 4 {
                scratch.sum.fill(0.0);
            }
            continue;
        }
        let bass = c.jam != 0 && x.part != 0;
        control(
            x,
            Control {
                program: if bass { programs.bass } else { programs.keys },
                lfo: if bass { pool.bass_lfo } else { pool.keys_lfo },
                config: c,
                cache,
            },
        );
        if x.s.quiet >= 2 && x.gate == 0 {
            x.used = 0;
            if c.jam != 0 && index == 4 {
                scratch.sum.fill(0.0);
            }
            continue;
        }
        operators::run(&mut x.s, t, scratch, 64, c.float_path != 0);
        if bass {
            if x.fade < 1.0 {
                for sample in scratch.sum.iter_mut() {
                    *sample *= max(x.fade, 0.0);
                }
                x.fade -= 1.0 / 103.0;
                if x.fade <= 0.0 {
                    x.used = 0;
                }
            }
            continue;
        }
        if x.fade < 1.0 {
            let g0 = x.fade;
            let g1 = g0 - if c.jam != 0 { 1.0 / 35.0 } else { 1.0 / 103.0 };
            for (j, out) in output.iter_mut().enumerate() {
                *out += scratch.sum[j] * (g0 + (g1 - g0) * j as f32 * (1.0 / 64.0));
            }
            x.fade = g1;
            if g1 <= 0.0 {
                x.used = 0;
            }
            continue;
        }
        for (out, sample) in output.iter_mut().zip(scratch.sum.iter()) {
            *out += *sample;
        }
    }
}
const _: [(); 24] = [(); core::mem::size_of::<Config>()];
const _: [(); 24] = [(); core::mem::size_of::<AmsCache>()];
