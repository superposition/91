// SPDX-License-Identifier: GPL-3.0-only
//! Effect-internal grid transitions. The musical scheduler retains callback
//! order; this helper owns refresh/Flex/leave decisions and their state changes.
use super::{
    control_abi::{Config, Engage, Grid},
    engage,
    ids::*,
    math::clamp,
};
use crate::sequencer::Rng;

pub fn apply(
    s: &mut Engage,
    g: &mut Grid,
    q: &Config,
    rng: &mut Rng,
    table: &[[u8; 16]; 27],
    kind: u32,
    step: i32,
) {
    match kind {
        0 => {
            if s.flx_active != 0 && s.bmode != 9 {
                s.flx_active = 0;
                s.en_flx_state = u32::from(g.flx_arm != 0);
            }
            if s.flx_active != 0 {
                s.flx_pos = (s.flx_pos + 1) & 255;
                if s.flx_pos >= 16 {
                    if (q.fxs_v[EN_FXS_FX_MODE] == 1 && g.flx_held != 0) || g.flx_phys != 0 {
                        s.flx_pos = 0;
                    } else {
                        s.flx_active = 0;
                        s.bm_tgt = 0.0;
                        g.flx_wait_rel = g.flx_held;
                        g.en_flx_done = g.en_flx_done.wrapping_add(1);
                        s.en_flx_state = u32::from(g.flx_arm != 0);
                    }
                }
                if s.flx_active != 0 {
                    engage::apply(
                        s,
                        q,
                        rng,
                        8,
                        table[g.flx_pat as usize][(s.flx_pos & 15) as usize] as i32,
                    );
                }
            }
            if g.flx_arm != 0 {
                let quant = clamp(q.fxs_v[EN_FXS_FX_QUANT] as f32, 0.0, 2.0) as i32;
                if quant == 0 || (quant == 1 && step & 3 == 0) || (quant == 2 && step == 0) {
                    g.flx_pat = g.flx_arm - 1;
                    g.flx_arm = 0;
                    engage::apply(s, q, rng, 7, table[g.flx_pat as usize][0] as i32);
                }
            }
        }
        1 => {
            if s.bmode == 1
                && g.rp_ref != 0
                && g.rp_ok != 0
                && (if g.rp_ref == 2 {
                    step == 0
                } else {
                    step & 3 == 0
                })
            {
                let rate = s.brate;
                engage::apply(s, q, rng, 0, rate);
            }
        }
        2 => s.bm_tgt = 0.0,
        _ => {}
    }
}
