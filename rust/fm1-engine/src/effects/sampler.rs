// SPDX-License-Identifier: GPL-3.0-only
//! The shared performance ring: repeat, reverse, scatter, scratch, grains and varispeed.
use super::math::*;
use crate::sequencer::Rng;
pub const FRAMES: usize = 21000;
pub const WORDS: usize = FRAMES * 2;
const LIVE: i32 = 0;
const REPEAT: i32 = 1;
const REVERSE: i32 = 2;
const STOP: i32 = 3;
const SCATTER: i32 = 4;
const VAR: i32 = 5;
const SCRATCH: i32 = 6;
const GRAIN: i32 = 7;
const SWELL: i32 = 8;
const FLEX: i32 = 9;
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub rb_w: u32,
    pub rb_wi: u32,
    pub bm_sm: u32,
    pub rb_odd: u32,
    pub rb_hl: f32,
    pub rb_hr: f32,
    pub bmode: i32,
    pub brate: i32,
    pub rp_rate: f32,
    pub rp_gain: f32,
    pub rp_pos: f32,
    pub bm_start: u32,
    pub bm_off: f32,
    pub bm_speed: f32,
    pub bm_mix: f32,
    pub bm_tgt: f32,
    pub sc_pos: f32,
    pub sc_off: f32,
    pub sc_rev: i32,
    pub sc_k: u32,
    pub scr_t: f32,
    pub flx_p: f32,
    pub gn_pos: [f32; 4],
    pub gn_st: [f32; 4],
    pub rv_pos: f32,
    pub vr_rate: f32,
    pub vr_lp: [f32; 2],
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub rp_step: f32,
    pub rp_gatef: f32,
    pub rp_t: f32,
    pub rp_pit: f32,
    pub rp_ref: u32,
    pub rp_ok: u32,
    pub bm_len: f32,
    pub sc_len: f32,
    pub sc_n: i32,
    pub sc_chaos: f32,
    pub sc_reverse: f32,
    pub scr_w: f32,
    pub scr_per: f32,
    pub scr_style: i32,
    pub flx_live: u32,
    pub flx_mute: u32,
    pub flx_rev: u32,
    pub flx_off: f32,
    pub flx_sub: f32,
    pub gn_len: f32,
    pub gn_region: f32,
    pub gn_scale: f32,
    pub gn_n: i32,
    pub rv_len: f32,
    pub vr_tgt: f32,
    pub vr_c: f32,
    pub vr_dull: f32,
    pub rl_gain: f32,
    pub step_length: f32,
}
const _: [(); 136] = [(); core::mem::size_of::<State>()];
const _: [(); 116] = [(); core::mem::size_of::<Config>()];
pub fn read(buffer: &[i16; WORDS], start: u32, off: f32) -> [f32; 2] {
    let f = off * 0.5;
    let mut k = f as i32;
    if f < 0.0 && k as f32 != f {
        k -= 1;
    }
    let t = f - k as f32;
    let mut i = start as i32 + k;
    i += if i < 0 {
        FRAMES as i32
    } else if i >= FRAMES as i32 {
        -(FRAMES as i32)
    } else {
        0
    };
    let a = i as usize;
    let b = if a + 1 == FRAMES { 0 } else { a + 1 };
    [
        (buffer[2 * a] as f32 + (buffer[2 * b] as f32 - buffer[2 * a] as f32) * t)
            * (1.0 / 12000.0),
        (buffer[2 * a + 1] as f32 + (buffer[2 * b + 1] as f32 - buffer[2 * a + 1] as f32) * t)
            * (1.0 / 12000.0),
    ]
}
fn write(buffer: &mut [i16; WORDS], s: &mut State, l: f32, r: f32) {
    if s.rb_odd == 0 {
        s.rb_hl = l;
        s.rb_hr = r;
        s.rb_odd = 1;
        return;
    }
    s.rb_odd = 0;
    let i = s.rb_wi as usize;
    buffer[2 * i] = super::echo::quantize(0.5 * (s.rb_hl + l));
    buffer[2 * i + 1] = super::echo::quantize(0.5 * (s.rb_hr + r));
    s.rb_w = s.rb_w.wrapping_add(1);
    s.rb_wi = if i + 1 == FRAMES { 0 } else { i as u32 + 1 };
}
fn scatter(s: &mut State, c: &Config, rng: &mut Rng) {
    let mut source = (s.sc_k % c.sc_n as u32) as i32;
    if source != 0 && rng.fraction() < c.sc_chaos {
        source = (rng.fraction() * c.sc_n as f32) as i32 % c.sc_n;
    }
    s.sc_rev = i32::from(source != 0 && rng.fraction() < c.sc_reverse);
    s.sc_off = source as f32 * c.sc_len;
    s.sc_pos = 0.0;
    s.sc_k = s.sc_k.wrapping_add(1);
}
pub fn sample(
    s: &mut State,
    c: &Config,
    buffer: &mut [i16; WORDS],
    rng: &mut Rng,
    mut l: f32,
    mut r: f32,
) -> [f32; 2] {
    if s.bmode == LIVE
        || s.bmode == STOP
        || s.bmode == VAR
        || (s.bmode == REPEAT && c.rp_ref != 0 && c.rp_ok != 0)
    {
        write(buffer, s, l, r);
    }
    if s.bmode != LIVE {
        let mut bl = 0.0;
        let mut br = 0.0;
        let mut w = 1.0;
        if s.bmode == REPEAT {
            if s.bm_off < c.bm_len {
                [bl, br] = read(buffer, s.bm_sm, s.bm_off);
                w = min(
                    min(min(s.bm_off, c.bm_len - s.bm_off), c.bm_len - s.rp_pos) * (1.0 / 64.0),
                    1.0,
                ) * s.rp_gain;
                if c.rp_gatef < 1.0 {
                    w = min(
                        w,
                        max((c.bm_len * c.rp_gatef - s.rp_pos) * (1.0 / 64.0), 0.0),
                    );
                }
                s.bm_off += s.rp_rate;
            } else {
                w = 0.0;
            }
            s.rp_pos += 1.0;
            if s.rp_pos >= c.bm_len {
                s.rp_pos -= c.bm_len;
                s.bm_off = 0.0;
                s.rp_gain *= c.rp_step;
                s.rp_rate = if c.rp_pit != 0.0 {
                    exp2(c.rp_pit * min(c.rp_t * 0.25, 1.0) * (1.0 / 12.0))
                } else {
                    1.0
                };
            }
        } else if s.bmode == REVERSE {
            [bl, br] = read(buffer, s.bm_sm, s.bm_off);
            w = min(min(s.bm_off, c.bm_len - s.bm_off) * (1.0 / 64.0), 1.0);
            s.bm_off -= 1.0;
            if s.bm_off < 0.0 {
                s.bm_off += c.bm_len;
            }
        } else if s.bmode == SCATTER {
            [bl, br] = read(
                buffer,
                s.bm_sm,
                s.sc_off
                    + if s.sc_rev != 0 {
                        c.sc_len - 1.0 - s.sc_pos
                    } else {
                        s.sc_pos
                    },
            );
            w = min(min(s.sc_pos, c.sc_len - 1.0 - s.sc_pos) * (1.0 / 48.0), 1.0);
            s.sc_pos += 1.0;
            if s.sc_pos >= c.sc_len {
                scatter(s, c, rng);
            }
        } else if s.bmode == SCRATCH {
            let ph = s.scr_t / c.scr_per;
            let off;
            if c.scr_style == 0 {
                off = c.scr_w * (0.5 - 0.5 * sin_turns(ph + 0.25));
                w = 1.0;
            } else {
                let u = if ph < 0.6 {
                    ph * (1.0 / 0.6)
                } else {
                    1.0 - (ph - 0.6) * 2.5
                };
                let e = u * u * (3.0 - 2.0 * u);
                off = c.scr_w * e;
                w = if ph < 0.6 { 1.0 } else { 0.2 };
            }
            [bl, br] = read(buffer, s.bm_sm, off);
            s.scr_t += 1.0;
            if s.scr_t >= c.scr_per {
                s.scr_t -= c.scr_per;
            }
        } else if s.bmode == FLEX {
            if c.flx_live != 0 {
                bl = l;
                br = r;
            } else if c.flx_mute != 0 {
                w = 0.0;
            } else {
                [bl, br] = read(
                    buffer,
                    s.bm_sm,
                    c.flx_off
                        + if c.flx_rev != 0 {
                            c.flx_sub - 1.0 - s.flx_p
                        } else {
                            s.flx_p
                        },
                );
                w = max(
                    min(min(s.flx_p, c.flx_sub - 1.0 - s.flx_p) * (1.0 / 48.0), 1.0),
                    0.0,
                );
                s.flx_p += 1.0;
                if s.flx_p >= c.flx_sub {
                    s.flx_p = 0.0;
                }
            }
        } else if s.bmode == GRAIN {
            for i in 0..c.gn_n as usize {
                let p = s.gn_pos[i] / c.gn_len;
                let win = 4.0 * p * (1.0 - p);
                let [gl, gr] = read(buffer, s.bm_sm, s.gn_st[i] + s.gn_pos[i]);
                bl += gl * win;
                br += gr * win;
                s.gn_pos[i] += 1.0;
                if s.gn_pos[i] >= c.gn_len {
                    s.gn_st[i] = rng.fraction() * max(c.gn_region - c.gn_len, 1.0);
                    s.gn_pos[i] = 0.0;
                }
            }
            w = c.gn_scale;
        } else if s.bmode == SWELL {
            if s.rv_pos >= 0.0 {
                let g = 1.0 - s.rv_pos / c.rv_len;
                [bl, br] = read(buffer, s.bm_sm, s.rv_pos);
                w = g * g * min(s.rv_pos * (1.0 / 48.0) + 0.0, 1.0);
                s.rv_pos -= 1.0;
            } else {
                w = 0.0;
                s.bm_tgt = 0.0;
            }
        } else if s.bmode == VAR {
            s.vr_rate += (c.vr_tgt - s.vr_rate) * c.vr_c;
            [bl, br] = read(buffer, s.bm_sm, s.bm_off);
            s.bm_off += s.vr_rate;
            if s.bm_off >= 16384.0 {
                s.bm_off -= 8192.0;
                s.bm_start = s.bm_start.wrapping_add(4096);
                s.bm_sm += 4096;
                if s.bm_sm >= FRAMES as u32 {
                    s.bm_sm -= FRAMES as u32;
                }
            } else if s.bm_off <= -16384.0 {
                s.bm_off += 8192.0;
                s.bm_start = s.bm_start.wrapping_sub(4096);
                s.bm_sm = if s.bm_sm >= 4096 {
                    s.bm_sm - 4096
                } else {
                    s.bm_sm + FRAMES as u32 - 4096
                };
            }
            let ar = if s.vr_rate < 0.0 {
                -s.vr_rate
            } else {
                s.vr_rate
            };
            let k = clamp(1.0 - c.vr_dull * (1.0 - min(ar, 1.0)), 0.04, 1.0);
            s.vr_lp[0] += k * (bl - s.vr_lp[0]);
            s.vr_lp[1] += k * (br - s.vr_lp[1]);
            bl = s.vr_lp[0];
            br = s.vr_lp[1];
            w = min(ar * 8.0, 1.0);
            let lag = 2.0 * s.rb_w.wrapping_sub(s.bm_start) as f32 - s.bm_off;
            if lag > (FRAMES - 16) as f32 * 2.0 {
                s.bm_tgt = 0.0;
            }
        } else if s.bm_speed > 0.0 {
            [bl, br] = read(buffer, s.bm_sm, s.bm_off);
            w = s.bm_speed;
            s.bm_off += s.bm_speed;
            s.bm_speed -= 1.0 / (4.0 * c.step_length);
        }
        s.bm_mix += (s.bm_tgt - s.bm_mix) * 0.02;
        l += (bl * w * c.rl_gain - l) * s.bm_mix;
        r += (br * w * c.rl_gain - r) * s.bm_mix;
        if s.bm_tgt <= 0.0 && s.bm_mix < 0.0005 {
            s.bm_mix = 0.0;
            s.bmode = LIVE;
            s.brate = -1;
        }
    }
    [l, r]
}
