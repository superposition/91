// SPDX-License-Identifier: GPL-3.0-only
//! Independent L/R vinyl generators, held in pairs at the original half rate.
use super::math::flush;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Generator {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub br: f32,
    pub pw: f32,
    pub pp: f32,
    pub y1: f32,
    pub y2: f32,
    pub crack: f32,
    pub lp: f32,
    pub r1: f32,
    pub r2: f32,
    pub rng: u32,
    pub cnt: u32,
}
#[repr(C)]
pub struct State {
    pub generators: [Generator; 2],
    pub held: [f32; 2],
    pub phase: u32,
}
#[repr(C)]
pub struct Config {
    pub fade: f32,
    pub level: f32,
    pub lowpass: f32,
    pub hiss: f32,
    pub pop: f32,
    pub amplitude: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub playing: u32,
    pub shed: u32,
}
const _: [(); 124] = [(); core::mem::size_of::<State>()];
const _: [(); 44] = [(); core::mem::size_of::<Config>()];
fn sample(z: &mut Generator, c: &Config) -> f32 {
    z.rng = z.rng.wrapping_mul(1664525).wrapping_add(1013904223);
    let w = z.rng as i32 as f32 * (1.0 / 2147483648.0);
    let mut pk = 0.0;
    if c.hiss > 0.0 {
        z.b0 = 0.99765 * z.b0 + w * 0.0990460;
        z.b1 = 0.96300 * z.b1 + w * 0.2965164;
        z.b2 = 0.57000 * z.b2 + w * 1.0526913;
        pk = (z.b0 + z.b1 + z.b2 + w * 0.1848) * 0.22 * c.hiss;
    }
    z.cnt = z.cnt.wrapping_add(1);
    if z.cnt & 3 == 0 {
        let mut h = (z.rng ^ (z.rng >> 15)).wrapping_mul(2246822519);
        if (h >> 8) as f32 * (1.0 / 16777216.0) < 4.0 * c.pop {
            h = (h ^ (h >> 13)).wrapping_mul(3266489917);
            let a = (h >> 8) as f32 * (1.0 / 16777216.0);
            z.crack = (0.1 + 1.2 * a * a) * c.amplitude;
            z.r1 = 0.90 + 0.075 * a;
            if a > 0.85 {
                z.y2 += 0.45 * c.amplitude * (a - 0.85) * 6.0;
            }
        } else {
            h = (h ^ (h >> 13)).wrapping_mul(3266489917);
            if (h >> 8) as f32 * (1.0 / 16777216.0) < 20.0 * c.pop {
                z.r2 = 0.09 * c.amplitude;
            }
        }
    }
    z.pp += 0.55 * (w - z.pp);
    let d = z.pp * 0.9;
    let out = d * (z.crack + z.r2) + z.y2 + pk;
    z.crack *= z.r1;
    z.r2 *= 0.80;
    z.y2 *= 0.9965;
    z.pw = w;
    out
}
pub fn process(s: &mut State, c: &Config, l: &mut [f32], r: &mut [f32]) {
    assert_eq!(l.len(), r.len());
    let gain = c.fade * c.level;
    if gain <= 0.0 || c.playing == 0 || c.shed != 0 {
        l.fill(0.0);
        r.fill(0.0);
        return;
    }
    for j in 0..l.len() {
        let tg = (c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse) * gain;
        let old = s.phase;
        s.phase = s.phase.wrapping_add(1);
        if old & 1 == 0 {
            s.held[0] = sample(&mut s.generators[0], c);
            s.held[1] = sample(&mut s.generators[1], c);
        }
        s.generators[0].lp += c.lowpass * (s.held[0] - s.generators[0].lp);
        s.generators[1].lp += c.lowpass * (s.held[1] - s.generators[1].lp);
        l[j] = s.generators[0].lp * tg;
        r[j] = s.generators[1].lp * tg;
    }
    for z in &mut s.generators {
        z.lp = flush(z.lp);
        z.br = flush(z.br);
    }
}
