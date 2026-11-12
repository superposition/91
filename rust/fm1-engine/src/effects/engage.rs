// SPDX-License-Identifier: GPL-3.0-only
//! Scoped parameter/initial-state transitions, used by direct requests and
//! sixteenth callbacks. The caller supplies a single immutable Flex table byte.
use super::{
    control_abi::{Config, Engage},
    ids::*,
    math::*,
};
use crate::sequencer::Rng;
const RING: u32 = 21000;
const AVAIL: f32 = (RING - 8) as f32 * 2.0;
fn start(s: &mut Engage, frames: u32) {
    s.bm_start = s.rb_w.wrapping_sub(frames);
    s.bm_sm = s.bm_start % RING;
}
fn flex_load(s: &mut Engage, b: u8) {
    let e = b & 15;
    let div = b >> 6 & 3;
    s.flx_mute = (b >> 5 & 1) as u32;
    s.flx_rev = (b >> 4 & 1) as u32;
    s.flx_live = u32::from(e == 15);
    if s.flx_live == 0 {
        s.flx_off = (s.flx_n - 1 - ((e as i32 * s.flx_n) >> 3)) as f32 * s.flx_slice;
        s.flx_sub = max(s.flx_slice / (1u32 << div) as f32, 32.0);
    }
    s.flx_p = 0.0;
}
pub fn apply(s: &mut Engage, q: &Config, rng: &mut Rng, kind: u32, arg: i32) {
    let v = |id: usize| q.fxs_v[id] as f32;
    let time = |value: f32, base: f32| 1.0 - exp(-3.0 / (base * exp2(value * 0.06) * 44100.0));
    match kind {
        0 => {
            const RPT: [f32; 9] = [
                2.0,
                1.0,
                2.0 / 3.0,
                0.5,
                0.25,
                0.125,
                1.0 / 3.0,
                1.0 / 6.0,
                0.0625,
            ];
            let mut r = arg as usize;
            let mut len = RPT[r] * 4.0 * q.step_len;
            while len > AVAIL && r < 5 {
                r += 1;
                len = RPT[r] * 4.0 * q.step_len;
            }
            if len > AVAIL {
                len = AVAIL;
            }
            if s.bmode != 1 {
                s.rp_t = 0.0;
            }
            s.bmode = 1;
            s.brate = r as i32;
            s.bm_len = len;
            start(s, (len * 0.5 + 0.5) as u32);
            s.bm_off = 0.0;
            s.bm_tgt = 1.0;
            s.rp_gain = 1.0;
            s.rp_pos = 0.0;
            s.rp_rate = if s.rp_pit != 0.0 {
                exp2(s.rp_pit * min(s.rp_t * 0.25, 1.0) * (1.0 / 12.0))
            } else {
                1.0
            };
        }
        1 => {
            let sl = max(
                q.step_len * [0.5, 1.0, 2.0][clamp(v(EN_FXS_SC_SIZE), 0.0, 2.0) as usize],
                64.0,
            );
            let mut n = clamp(v(EN_FXS_SC_LEN), 2.0, 8.0) as i32;
            while n > 2 && n as f32 * sl > AVAIL {
                n -= 1;
            }
            s.sc_n = n;
            s.sc_len = sl;
            s.sc_k = 1;
            s.bmode = 4;
            start(s, (n as f32 * sl * 0.5 + 0.5) as u32);
            s.bm_tgt = 1.0;
            // First slice is always source zero: scat_pick draws no random word.
            s.sc_rev = 0;
            s.sc_off = 0.0;
            s.sc_pos = 0.0;
        }
        2 => {
            s.bmode = 5;
            start(s, 2);
            s.bm_off = 0.0;
            s.bm_tgt = 1.0;
            s.vr_rate = 1.0;
            s.vr_which = arg;
            s.vr_lp = [0.0; 2];
        }
        3 => {
            if arg == FX_HALF as i32 {
                s.vr_tgt = [0.667, 0.5, 0.25][clamp(v(EN_FXS_HF_SPEED), 0.0, 2.0) as usize];
                s.vr_c = time(v(EN_FXS_HF_SLIDE), 0.02);
                s.vr_dull = 0.0;
            } else {
                s.vr_tgt = if arg == FX_SPIN as i32 {
                    -v(EN_FXS_BR_BACK) * 0.04
                } else {
                    0.0
                };
                s.vr_c = time(v(EN_FXS_BR_TIME), 0.03);
                s.vr_dull = v(EN_FXS_BR_DULL) * 0.009;
            }
        }
        4 => {
            s.scr_w = min(
                max(
                    q.step_len * [1.0, 2.0, 4.0, 8.0][clamp(v(EN_FXS_SR_REACH), 0.0, 3.0) as usize],
                    64.0,
                ),
                AVAIL,
            );
            s.scr_per = max(
                q.step_len * [4.0, 2.0, 1.0, 0.5][clamp(v(EN_FXS_SR_EVERY), 0.0, 3.0) as usize],
                32.0,
            );
            s.scr_w = min(
                s.scr_w,
                s.scr_per
                    * if q.fxs_v[EN_FXS_SR_STYLE] == 0 {
                        0.8
                    } else {
                        0.6
                    },
            );
            s.scr_t = 0.0;
            s.bmode = 6;
            start(s, (s.scr_w * 0.5 + 0.5) as u32);
            s.bm_tgt = 1.0;
        }
        5 => {
            let frames = min(
                (0.1 + 0.0075 * v(EN_FXS_FZ_SPREAD)) * 22050.0,
                (RING - 8) as f32,
            );
            s.gn_region = frames * 2.0;
            s.gn_len = min(20.0 * exp2(v(EN_FXS_FZ_SIZE) * 0.033) * 44.1, s.gn_region);
            s.gn_n = 1 + clamp(v(EN_FXS_FZ_GRAINS), 0.0, 3.0) as i32;
            s.gn_scale = 1.5 / sqrt(s.gn_n as f32);
            s.bmode = 7;
            start(s, frames as u32);
            s.bm_tgt = 1.0;
            for i in 0..s.gn_n as usize {
                s.gn_st[i] = rng.fraction() * max(s.gn_region - s.gn_len, 1.0);
                s.gn_pos[i] = s.gn_len * i as f32 / s.gn_n as f32;
            }
        }
        6 => {
            s.rv_len = min(
                q.step_len * [2.0, 4.0, 8.0][clamp(v(EN_FXS_RV_LEN), 0.0, 2.0) as usize],
                AVAIL,
            );
            s.rv_pos = s.rv_len - 1.0;
            s.bmode = 8;
            start(s, (s.rv_len * 0.5 + 0.5) as u32);
            s.bm_tgt = 0.3 + 0.007 * v(EN_FXS_RV_MIX);
        }
        7 => {
            if s.flx_active != 0 && s.bmode == 9 {
                s.flx_pos = 0;
                flex_load(s, arg as u8);
                return;
            }
            s.flx_slice = max(q.step_len, 64.0);
            s.flx_n = (AVAIL / s.flx_slice) as i32;
            s.flx_n = s.flx_n.clamp(1, 8);
            s.bmode = 9;
            start(s, (s.flx_n as f32 * s.flx_slice * 0.5 + 0.5) as u32);
            s.bm_tgt = 0.01 * clamp(v(EN_FXS_FX_MIX), 0.0, 100.0);
            s.flx_pos = 0;
            s.flx_active = 1;
            s.en_flx_state = 2;
            flex_load(s, arg as u8);
        }
        8 => flex_load(s, arg as u8),
        _ => {}
    }
}
