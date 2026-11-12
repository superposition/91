// SPDX-License-Identifier: GPL-3.0-only
//! Fixed voice state and block-scoped sample kernels over caller-owned buffers.
use crate::effects::math::{flush, max, min, tanh};
pub mod acid;
pub mod drone;

pub fn blep_saw(t: f32, dt: f32) -> f32 {
    let mut y = 2.0 * t - 1.0;
    if t < dt {
        let x = t / dt;
        y -= x + x - x * x - 1.0;
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        y -= x * x + x + x + 1.0;
    }
    y
}
pub fn svf_mm(x: f32, c: &[f32; 4], s: &mut [f32], kind: i32) -> f32 {
    let v3 = x - s[1];
    let v1 = c[0] * s[0] + c[1] * v3;
    let v2 = s[1] + c[1] * s[0] + c[2] * v3;
    s[0] = 2.0 * v1 - s[0];
    s[1] = 2.0 * v2 - s[1];
    match kind {
        1 => x - c[3] * v1 - v2,
        2 => c[3] * v1,
        3 => x - c[3] * v1,
        4 => 2.0 * v2 - x + c[3] * v1,
        _ => v2,
    }
}

pub fn sine(table: &[f32; 1025], phase: f32) -> f32 {
    let x = (phase - phase as i32 as f32) * 1024.0;
    let i = x as usize;
    let f = x - i as f32;
    table[i] + (table[i + 1] - table[i]) * f
}
pub fn sine_any(table: &[f32; 1025], phase: f32) -> f32 {
    let mut floor = phase as i32 as f32;
    if phase < floor {
        floor -= 1.0;
    }
    sine(table, phase - floor)
}
fn random_word(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed as i32 as f32
}
fn random(seed: &mut u32) -> f32 {
    random_word(seed) * (1.0 / 2147483648.0)
}
pub struct Mix<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
    pub reverb: &'a mut [f32],
    pub echo: &'a mut [f32],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Drum {
    pub ph: f32,
    pub inc: f32,
    pub inc_end: f32,
    pub amp: f32,
    pub adec: f32,
    pub idx: f32,
    pub x1: f32,
    pub x2: f32,
    pub att: f32,
    pub mph: [f32; 4],
    pub pend: f32,
    pub t: f32,
    pub hp: f32,
    pub ph2: f32,
    pub inc2: f32,
    pub wait: i32,
    pub wait2: i32,
    pub pend2: f32,
    pub rgap: i32,
    pub rmul: f32,
    pub xl: f32,
    pub fo: f32,
    pub rn: u8,
    pub on: u8,
    pub note: u8,
}
#[repr(C)]
pub struct DrumsConfig {
    pub gains: [f32; 6],
    pub hat_inc: [f32; 4],
    pub drive: f32,
    pub drop: f32,
    pub click: f32,
    pub rumble: f32,
    pub sn_body: f32,
    pub sn_noise: f32,
    pub cl_gap: f32,
    pub h_noise: f32,
    pub gains0: [f32; 4],
    pub gains1: [f32; 4],
    pub inverse: f32,
}
fn ramp(c: &DrumsConfig, track: usize, frame: usize) -> f32 {
    c.gains0[track] + (c.gains1[track] - c.gains0[track]) * frame as f32 * c.inverse
}
const FADE: f32 = 1.0 - 1.0 / (0.0008 * 44100.0);
fn hat(v: &mut Drum, c: &DrumsConfig, seed: &mut u32, g: f32, pl: f32, pr: f32, b: &mut Mix<'_>) {
    if v.on == 0 {
        return;
    }
    let (mut x1, mut x2, mut amp, mut att, mut fo) = (v.x1, v.x2, v.amp, v.att, v.fo);
    let mut x = 0.0;
    for j in 0..b.left.len() {
        let mut input = 0.0;
        for k in 0..4 {
            v.mph[k] += c.hat_inc[k];
            if v.mph[k] >= 1.0 {
                v.mph[k] -= 1.0;
            }
            input += if v.mph[k] < 0.5 { 0.25 } else { -0.25 };
        }
        input = 0.5 * input + c.h_noise * random_word(seed) * (1.0 / 2147483648.0);
        let hp = input - x2 - 0.8 * x1;
        x1 += v.hp * hp;
        x2 += v.hp * x1;
        x = hp * amp * att * g * ramp(c, 2, j) + fo;
        fo *= FADE;
        b.left[j] += x * pl;
        b.right[j] += x * pr;
        b.reverb[j] += x * 0.2;
        b.echo[j] += x * 0.08;
        amp *= v.adec;
        att += (1.0 - att) * 0.3;
    }
    v.x1 = flush(x1);
    v.x2 = flush(x2);
    v.amp = amp;
    v.att = att;
    v.fo = flush(fo);
    v.xl = x;
    v.on = u8::from(amp > 0.0005);
}
fn clap(v: &mut Drum, c: &DrumsConfig, table: &[f32; 1025], seed: &mut u32, b: &mut Mix<'_>) {
    if v.on == 0 {
        return;
    }
    let (mut s1, mut s2, mut amp, mut t, mut ph, mut fo) = (v.x1, v.x2, v.amp, v.t, v.ph, v.fo);
    let mut x = 0.0;
    for j in 0..b.left.len() {
        if t == c.cl_gap || t == 2.0 * c.cl_gap {
            amp = v.pend;
        }
        let input = random(seed);
        let bp = input - s2 - 0.7 * s1;
        s1 += v.hp * bp;
        s2 += v.hp * s1;
        x = (s1 * 1.6 + 0.35 * sine(table, ph) * max(1.0 - t * (1.0 / 2500.0), 0.0))
            * amp
            * c.gains[2]
            * ramp(c, 1, j)
            + fo;
        fo *= FADE;
        ph += v.inc;
        if ph >= 1.0 {
            ph -= 1.0;
        }
        b.left[j] += x * 0.95;
        b.right[j] += x;
        b.reverb[j] += x * 0.45;
        b.echo[j] += x * 0.12;
        amp *= if t < 2.0 * c.cl_gap {
            v.inc_end
        } else {
            v.adec
        };
        t += 1.0;
    }
    v.x1 = flush(s1);
    v.x2 = flush(s2);
    v.amp = amp;
    v.t = t;
    v.ph = ph;
    v.fo = flush(fo);
    v.xl = x;
    v.on = u8::from(t < 2.0 * c.cl_gap || amp > 0.0005);
}
fn snare(v: &mut Drum, c: &DrumsConfig, table: &[f32; 1025], seed: &mut u32, b: &mut Mix<'_>) {
    if v.on == 0 {
        return;
    }
    let (mut s1, mut s2, mut amp, mut ph, mut ph2, mut att, mut body, mut fo) =
        (v.x1, v.x2, v.amp, v.ph, v.ph2, v.att, v.idx, v.fo);
    let mut x = 0.0;
    if v.t == 0.0 {
        body = 1.0;
    }
    for j in 0..b.left.len() {
        let input = random(seed);
        let bp = input - s2 - 0.5 * s1;
        s1 += v.hp * bp;
        s2 += v.hp * s1;
        x = ((sine(table, ph) + 0.6 * sine(table, ph2)) * body * c.sn_body + s1 * c.sn_noise)
            * amp
            * att
            * c.gains[1]
            * ramp(c, 1, j)
            + fo;
        fo *= FADE;
        ph += v.inc;
        ph2 += v.inc2;
        if ph >= 1.0 {
            ph -= 1.0;
        }
        if ph2 >= 1.0 {
            ph2 -= 1.0;
        }
        b.left[j] += x;
        b.right[j] += x * 0.95;
        b.reverb[j] += x * 0.35;
        b.echo[j] += x * 0.1;
        amp *= v.adec;
        body *= 0.9992;
        att += (1.0 - att) * 0.4;
    }
    v.x1 = flush(s1);
    v.x2 = flush(s2);
    v.amp = amp;
    v.ph = ph;
    v.ph2 = ph2;
    v.att = att;
    v.idx = body;
    v.t = 1.0;
    v.fo = flush(fo);
    v.xl = x;
    v.on = u8::from(amp > 0.0005);
}
fn perc(v: &mut Drum, c: &DrumsConfig, table: &[f32; 1025], b: &mut Mix<'_>) {
    if v.on == 0 {
        return;
    }
    let (mut ph, mut ph2, mut inc, mut amp, mut idx, mut att, mut fo) =
        (v.ph, v.ph2, v.inc, v.amp, v.idx, v.att, v.fo);
    let mut x = 0.0;
    for j in 0..b.left.len() {
        let m = sine(table, ph2) * idx;
        x = sine(table, ph + m + 4.0) * amp * att * c.gains[5] * ramp(c, 3, j) + fo;
        fo *= FADE;
        ph += inc;
        ph2 += inc * v.inc2;
        if ph >= 1.0 {
            ph -= 1.0;
        }
        if ph2 >= 1.0 {
            ph2 -= 1.0;
        }
        inc += (v.inc_end - inc) * 0.004;
        b.left[j] += x * 0.8;
        b.right[j] += x;
        b.reverb[j] += x * 0.3;
        b.echo[j] += x * 0.25;
        amp *= v.adec;
        idx *= 0.9995;
        att += (1.0 - att) * 0.3;
    }
    v.ph = ph;
    v.ph2 = ph2;
    v.inc = inc;
    v.amp = amp;
    v.idx = idx;
    v.att = att;
    v.fo = flush(fo);
    v.xl = x;
    v.on = u8::from(amp > 0.0005);
}
pub fn drums(
    voices: &mut [Drum; 6],
    c: &DrumsConfig,
    table: &[f32; 1025],
    seed: &mut u32,
    mut b: Mix<'_>,
) {
    let v = &mut voices[0];
    if v.on != 0 {
        let (mut ph, mut inc, mut amp, mut att, mut ck, mut fo) =
            (v.ph, v.inc, v.amp, v.att, v.idx, v.fo);
        let mut x = 0.0;
        let (mut r1, mut r2, mut rl1, mut rl2, mut rp) = (v.x1, v.x2, v.mph[0], v.mph[1], v.ph2);
        let norm = 0.85 / min(1.0 + 0.25 * (c.drive - 1.0), 1.6);
        for j in 0..b.left.len() {
            let nz = random(seed);
            x = sine(table, ph) * amp * att + ck * c.click * nz;
            if c.rumble > 0.0 {
                rl1 += 0.012 * (nz - rl1);
                rl2 += 0.012 * (rl1 - rl2);
                x += (r1 - r2) * (3.0 * rl2 + 0.5 * sine(table, rp));
                rp += v.inc_end;
                if rp >= 1.0 {
                    rp -= 1.0;
                }
                r1 *= 0.99985;
                r2 *= 0.9985;
            }
            x = tanh(x * 1.3 * c.drive) * norm * c.gains[0] * ramp(c, 0, j) + fo;
            fo *= FADE;
            b.left[j] += x;
            b.right[j] += x;
            ph += inc;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            inc += (v.inc_end - inc) * c.drop;
            amp *= v.adec;
            att += (1.0 - att) * 0.25;
            ck *= 0.985;
        }
        v.ph = ph;
        v.inc = inc;
        v.amp = amp;
        v.att = att;
        v.idx = ck;
        v.x1 = r1;
        v.x2 = r2;
        v.mph[0] = flush(rl1);
        v.mph[1] = flush(rl2);
        v.ph2 = rp;
        v.fo = flush(fo);
        v.xl = x;
        v.on = u8::from(amp > 0.0005 || r1 - r2 > 0.002);
    }
    snare(&mut voices[1], c, table, seed, &mut b);
    clap(&mut voices[2], c, table, seed, &mut b);
    hat(&mut voices[3], c, seed, c.gains[3], 0.75, 1.0, &mut b);
    hat(&mut voices[4], c, seed, c.gains[4], 1.0, 0.75, &mut b);
    perc(&mut voices[5], c, table, &mut b);
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Pluck {
    pub ph: f32,
    pub ps: f32,
    pub inc: f32,
    pub env: f32,
    pub dm: f32,
    pub vel: f32,
    pub age: u32,
}
#[repr(C)]
pub struct PluckConfig {
    pub gain: f32,
    pub pitch: f32,
    pub ratio: f32,
    pub index0: f32,
    pub index1: f32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
    pub old_level: f32,
    pub rise: u32,
    pub fm: u32,
}
pub fn pluck(voices: &mut [Pluck; 2], c: &PluckConfig, table: &[f32; 1025], b: Mix<'_>) -> f32 {
    if c.fm != 0 || c.gain <= 0.0 {
        voices[0].env = 0.0;
        voices[1].env = 0.0;
        return if c.fm != 0 { c.old_level } else { 0.0 };
    }
    let mut level = 0.0;
    for (v, voice) in voices.iter_mut().enumerate() {
        let pl = if v != 0 { 0.65 } else { 0.35 };
        let pr = 1.0 - pl;
        if voice.env < 0.0008 {
            continue;
        }
        for j in 0..b.left.len() {
            let e = voice.env;
            let idx = c.index0 + c.index1 * if c.rise != 0 { 1.0 - e } else { e };
            let y = sine_any(table, voice.ph + idx * sine(table, voice.ps))
                * e
                * min(voice.age as f32 * (1.0 / 96.0), 1.0)
                * voice.vel
                * c.gain
                * (c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse);
            b.left[j] += y * pl * 2.0;
            b.right[j] += y * pr * 2.0;
            b.reverb[j] += y * 0.35;
            b.echo[j] += y * 0.5;
            voice.ph += voice.inc * c.pitch;
            if voice.ph >= 1.0 {
                voice.ph -= 1.0;
            }
            voice.ps += voice.inc * c.pitch * c.ratio;
            if voice.ps >= 1.0 {
                voice.ps -= voice.ps as i32 as f32;
            }
            voice.env *= voice.dm;
            voice.age = voice.age.wrapping_add(1);
            level = max(level, e * voice.vel);
        }
        voice.env = flush(voice.env);
    }
    level
}
