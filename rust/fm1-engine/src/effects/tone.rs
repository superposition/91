// SPDX-License-Identifier: GPL-3.0-only
//! Performance filters, formants, tape colour, flanger/phaser, ring mod and crush.
use super::math::*;
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub lp_s: [f32; 4],
    pub hp_s: [f32; 4],
    pub sw_s: [[f32; 2]; 2],
    pub sw_s2: [[f32; 2]; 2],
    pub tk_s: [[[f32; 2]; 2]; 2],
    pub wh_s: [[f32; 2]; 2],
    pub wh_c: [f32; 4],
    pub wh_env: f32,
    pub wh_cnt: u32,
    pub mod_w: u32,
    pub tp_ph: f32,
    pub tp_ph2: f32,
    pub tp_d: f32,
    pub tp_dd: f32,
    pub fl_ph: f32,
    pub fl_d: f32,
    pub fl_dd: f32,
    pub tp_lp: [f32; 2],
    pub ph_z: [[f32; 4]; 2],
    pub ph_fb: [f32; 2],
    pub rm_ph: f32,
    pub crush_n: i32,
    pub crush_l: f32,
    pub crush_r: f32,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub lp_amt_e: f32,
    pub hp_amt_e: f32,
    pub lp_c: [f32; 4],
    pub hp_c: [f32; 4],
    pub sw_c: [f32; 4],
    pub sw_c2: [f32; 4],
    pub tk_c: [[f32; 4]; 2],
    pub sw_m: f32,
    pub tk_m: f32,
    pub wh_m: f32,
    pub sw_type: i32,
    pub sw_reso: i32,
    pub tk_mix: i32,
    pub wh_mix: i32,
    pub wh_sens: i32,
    pub wh_range: i32,
    pub wh_reso: i32,
    pub tp_m: f32,
    pub fl_m: f32,
    pub ph_m: f32,
    pub tp_wd: f32,
    pub tp_fd: f32,
    pub tp_drive: f32,
    pub tp_idrive: f32,
    pub tp_k: f32,
    pub fl_inc: f32,
    pub fl_rng: f32,
    pub fl_fbk: f32,
    pub fl_wet: f32,
    pub fl_inv: f32,
    pub ph_fbk: f32,
    pub ph_k0: f32,
    pub ph_k1: f32,
    pub ph_a: [f32; 2],
    pub rm_m: f32,
    pub rm_inc: f32,
    pub rm_wet: f32,
    pub cr_wet: f32,
    pub cr_lv: f32,
    pub cr_hold: i32,
}
const _: [(); 232] = [(); core::mem::size_of::<State>()];
const _: [(); 240] = [(); core::mem::size_of::<Config>()];
fn filter(input: f32, c: &[f32; 4], s: &mut [f32; 2]) -> (f32, f32) {
    let v3 = input - s[1];
    let v1 = c[0] * s[0] + c[1] * v3;
    let v2 = s[1] + c[1] * s[0] + c[2] * v3;
    s[0] = flush(2.0 * v1 - s[0]);
    s[1] = flush(2.0 * v2 - s[1]);
    (v1, v2)
}
fn read_indices(write: usize, delay: f32) -> (usize, usize, f32) {
    let mut rd = write as f32 - delay;
    rd += if rd < 0.0 { 512.0 } else { 0.0 };
    let a = rd as usize & 511;
    let b = (a + 1) & 511;
    let f = rd - rd as i32 as f32;
    (a, b, f)
}
pub fn sample(
    s: &mut State,
    c: &Config,
    tape: &mut [[f32; 512]; 2],
    flanger: &mut [[f32; 512]; 2],
    mut l: f32,
    mut r: f32,
) -> [f32; 2] {
    if c.lp_amt_e > 0.0 {
        l = svf_lp(l, &c.lp_c, &mut s.lp_s[0..2]);
        r = svf_lp(r, &c.lp_c, &mut s.lp_s[2..4]);
    }
    if c.hp_amt_e > 0.0 {
        l -= svf_lp(l, &c.hp_c, &mut s.hp_s[0..2]);
        r -= svf_lp(r, &c.hp_c, &mut s.hp_s[2..4]);
    }
    if c.sw_m > 0.0 {
        let kind = clamp(c.sw_type as f32, 0.0, 2.0) as i32;
        let input = [l, r];
        let mut out = [0.0; 2];
        let k = c.sw_c[3];
        for channel in 0..2 {
            let x = input[channel];
            let (v1, v2) = filter(x, &c.sw_c, &mut s.sw_s[channel]);
            out[channel] = if kind == 2 {
                let (u1, _) = filter(x, &c.sw_c2, &mut s.sw_s2[channel]);
                k * v1 * 1.3 + c.sw_c2[3] * u1 * c.sw_reso as f32 * 0.012
            } else if kind == 0 {
                v2
            } else {
                x - k * v1 - v2
            };
        }
        l += (out[0] - l) * c.sw_m;
        r += (out[1] - r) * c.sw_m;
    }
    if c.tk_m > 0.0 {
        let w = c.tk_m * 0.01 * clamp(c.tk_mix as f32, 0.0, 100.0);
        let input = [l, r];
        let mut out = [0.0; 2];
        for channel in 0..2 {
            let mut y = 0.0;
            for f in 0..2 {
                let (v1, _) = filter(input[channel], &c.tk_c[f], &mut s.tk_s[channel][f]);
                y += c.tk_c[f][3] * v1 * if f != 0 { 0.7 } else { 1.0 };
            }
            out[channel] = y * 1.8;
        }
        l += (out[0] - l) * w;
        r += (out[1] - r) * w;
    }
    if c.wh_m > 0.0 {
        let w = c.wh_m * 0.01 * clamp(c.wh_mix as f32, 0.0, 100.0);
        let x = 0.5 * (abs(l) + abs(r));
        let input = [l, r];
        let mut out = [0.0; 2];
        s.wh_env += (x - s.wh_env) * if x > s.wh_env { 0.02 } else { 0.0015 };
        let count = s.wh_cnt;
        s.wh_cnt = s.wh_cnt.wrapping_add(1);
        if count & 31 == 0 {
            let e = clamp(s.wh_env * (2.0 + 14.0 * c.wh_sens as f32 * 0.01), 0.0, 1.0);
            s.wh_c = svf_coefs(
                280.0 * exp2(4.0 * c.wh_range as f32 * 0.01 * e),
                0.7 - 0.0058 * c.wh_reso as f32,
            );
        }
        for channel in 0..2 {
            let (v1, _) = filter(input[channel], &s.wh_c, &mut s.wh_s[channel]);
            out[channel] = s.wh_c[3] * v1 * 2.0;
        }
        l += (out[0] - l) * w;
        r += (out[1] - r) * w;
    }
    if c.tp_m > 0.0 || c.fl_m > 0.0 || c.ph_m > 0.0 {
        let write = (s.mod_w & 511) as usize;
        let mut input = [l, r];
        if c.tp_m > 0.0 {
            if s.mod_w & 15 == 0 {
                s.tp_ph += 16.0 * 0.7 * (1.0 / 44100.0);
                s.tp_ph2 += 16.0 * 8.0 * (1.0 / 44100.0);
                s.tp_ph -= s.tp_ph as i32 as f32;
                s.tp_ph2 -= s.tp_ph2 as i32 as f32;
                let dn = 150.0 + c.tp_wd * sin_turns(s.tp_ph) + c.tp_fd * sin_turns(s.tp_ph2);
                s.tp_dd = (dn - s.tp_d) * (1.0 / 16.0);
            }
            s.tp_d += s.tp_dd;
            let (a, b, f) = read_indices(write, s.tp_d);
            for channel in 0..2 {
                tape[channel][write] = input[channel];
                let mut y = tape[channel][a] + (tape[channel][b] - tape[channel][a]) * f;
                s.tp_lp[channel] += c.tp_k * (y - s.tp_lp[channel]);
                y = soft_sat(s.tp_lp[channel] * c.tp_drive) * c.tp_idrive;
                input[channel] += (y - input[channel]) * c.tp_m;
            }
        }
        if c.fl_m > 0.0 {
            if s.mod_w & 15 == 0 {
                s.fl_ph += 16.0 * c.fl_inc;
                s.fl_ph -= s.fl_ph as i32 as f32;
                let dn = 10.0 + c.fl_rng * (0.5 + 0.5 * sin_turns(s.fl_ph));
                s.fl_dd = (dn - s.fl_d) * (1.0 / 16.0);
            }
            s.fl_d += s.fl_dd;
            let (a, b, f) = read_indices(write, s.fl_d);
            for channel in 0..2 {
                let y = flanger[channel][a] + (flanger[channel][b] - flanger[channel][a]) * f;
                flanger[channel][write] =
                    flush(input[channel] + c.fl_fbk * if channel != 0 { -y } else { y });
                input[channel] = (input[channel] + c.fl_wet * y * c.fl_m) * c.fl_inv;
            }
        }
        let mut out = input;
        if c.ph_m > 0.0 {
            for channel in 0..2 {
                let x = input[channel] + c.ph_fbk * s.ph_fb[channel];
                let mut y = x;
                let a = c.ph_a[channel];
                for z in &mut s.ph_z[channel] {
                    let v = a * y + *z;
                    *z = y - a * v;
                    y = v;
                }
                s.ph_fb[channel] = y;
                out[channel] = input[channel]
                    + (input[channel] * c.ph_k0 + y * c.ph_k1 - input[channel]) * c.ph_m;
            }
        }
        s.mod_w = s.mod_w.wrapping_add(1);
        l = out[0];
        r = out[1];
    }
    if c.rm_m > 0.0 {
        s.rm_ph += c.rm_inc;
        s.rm_ph -= s.rm_ph as i32 as f32;
        let v = sin_turns(s.rm_ph);
        l += (l * v - l) * c.rm_m * c.rm_wet;
        r += (r * v - r) * c.rm_m * c.rm_wet;
    }
    if c.cr_wet > 0.0 {
        s.crush_n += 1;
        if s.crush_n >= c.cr_hold {
            let iv = 1.0 / c.cr_lv;
            s.crush_n = 0;
            s.crush_l = (l * c.cr_lv + if l < 0.0 { -0.5 } else { 0.5 }) as i32 as f32 * iv;
            s.crush_r = (r * c.cr_lv + if r < 0.0 { -0.5 } else { 0.5 }) as i32 as f32 * iv;
        }
        l += (s.crush_l - l) * c.cr_wet;
        r += (s.crush_r - r) * c.cr_wet;
    }
    [l, r]
}
