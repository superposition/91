// SPDX-License-Identifier: GPL-3.0-only
//! One render chunk's effect masks, gains, fades and coefficient selection.
//! Returned bounded action describes the single final engagement callback;
//! no callback/pointer is retained and no delay payload is copied.
#![allow(clippy::excessive_precision, clippy::approx_constant)]
use super::{
    control_abi::{Config, State},
    ids::*,
    math::*,
};
const SR: f32 = 44100.0;
// Stereo ring: 21,000 frames, 42,000 i16 words.
const RB_N: u32 = 21000;
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
fn fade(value: &mut f32, held: bool, sm: f32) {
    *value += (u32::from(held) as f32 - *value) * sm;
    if *value < 0.002 && !held {
        *value = 0.0;
    }
}
pub fn chunk(s: &mut State, q: &Config, n: u32) -> u32 {
    // A control interval may carry 127 skipped frames plus the next 256-frame
    // chunk. It is not the sample-buffer length.
    assert!(n > 0 && n <= 383);
    let m = (q.mask_hi as u64) << 32 | q.mask_lo as u64;
    let held = |bit: usize| (m >> bit & 1) != 0;
    let v = |id: usize| q.fxs_v[id] as f32;
    let mut mute = q.trk_mute;
    for (fx, tr) in [
        (FX_MUTE_KICK, EN_TR_KICK),
        (FX_MUTE_HATS, EN_TR_HATS),
        (FX_MUTE_SNARE, EN_TR_SNARE),
        (FX_MUTE_PERC, EN_TR_PERC),
    ] {
        mute |= u32::from(held(fx)) << tr;
    }
    mute |= u32::from(held(FX_MDRUMS)) * 15;
    let dt = n as f32 * (1.0 / SR);
    let len = clamp(v(EN_FXS_GX_LEN), 2.0, 16.0) as u32;
    let tg = q.fxs_v[EN_FXS_GX_TGT] as u32;
    let step = q.sixteenth.wrapping_sub(1);
    let open = q.gx_pat >> (step % len) & 1;
    let closed = 1.0 - clamp(v(EN_FXS_GX_DEPTH) * 0.01, 0.0, 1.0);
    for t in 0..EN_NTRACK {
        s.gate_f[t] = if held(FX_GATEX) && tg >> t & 1 != 0 && open == 0 {
            closed
        } else {
            1.0
        };
    }
    s.kick_age += n as f32;
    if held(FX_PUMP) {
        let per = [8, 4, 2, 1][clamp(v(EN_FXS_PM_RATE), 0.0, 3.0) as usize];
        let ph = if q.fxs_v[EN_FXS_PM_RATE] >= 4 {
            s.kick_age / (4.0 * q.step_len)
        } else {
            (step.wrapping_rem(per) as f32 * q.step_len + q.step_t as f32)
                / (per as f32 * q.step_len)
        };
        let rel = 0.15 + 0.0085 * v(EN_FXS_PM_REL);
        let dp = v(EN_FXS_PM_DEPTH) * 0.01;
        let f = (1.0 - dp) + dp * min(ph / rel, 1.0);
        for t in 0..EN_NTRACK {
            if (q.fxs_v[EN_FXS_PM_TGT] as u32) >> t & 1 != 0 {
                s.gate_f[t] *= f;
            }
        }
    }
    let mut k = 1.0;
    if q.en_jam_on == 0 && q.intro_len > 0 && q.en_playing != 0 {
        if s.intro_t < q.intro_len {
            s.intro_t += n as i32;
        }
        k = if s.intro_t >= q.intro_len {
            1.0
        } else {
            s.intro_t as f32 / q.intro_len as f32
        };
    }
    s.en_intro_k = k;
    for (t, at) in [40, 6, 52, 20, 62, 86, 0, 92].iter().enumerate() {
        let mut tgt = if mute >> t & 1 == 0 && (q.trk_solo == 0 || q.trk_solo >> t & 1 != 0) {
            s.gate_f[t]
        } else {
            0.0
        };
        let d = n as f32 * (1.0 / 220.0);
        if k < 1.0 && *at != 0 {
            tgt *= clamp((k * 100.0 - *at as f32) * (1.0 / 14.0), 0.0, 1.0);
        }
        s.trk_g[t] = s.trk_g1[t];
        s.trk_g1[t] = s.trk_g[t] + clamp(tgt - s.trk_g[t], -d, d);
    }
    s.tg_inv = 1.0 / n as f32;
    s.lp_amt = clamp(
        s.lp_amt + if held(FX_LP) { dt * 2.0 } else { -dt * 6.0 },
        0.0,
        1.0,
    );
    s.hp_amt = clamp(
        s.hp_amt + if held(FX_HP) { dt * 2.0 } else { -dt * 6.0 },
        0.0,
        1.0,
    );
    s.throw_key = clamp(
        s.throw_key
            + if held(FX_THROW) {
                dt * 60.0
            } else {
                -dt * 60.0
            },
        0.0,
        1.0,
    );
    s.vin_m += (u32::from(held(FX_VINYL)) as f32 - s.vin_m) * min(dt * 8.0, 1.0);
    s.rs_inv = 1.0 / exp2(v(EN_FXS_RS_TIME) * 0.036);
    if s.riser_cut != 0 {
        s.riser_amt = 0.0;
        s.riser_cut = 0;
    }
    s.riser_amt = clamp(
        s.riser_amt
            + if held(FX_RISER) {
                dt * s.rs_inv
            } else {
                -dt * 2.0
            },
        0.0,
        1.0,
    );
    s.rs_filt = v(EN_FXS_RS_FILT) * 0.009;
    s.rs_wash = v(EN_FXS_RS_WASH) * 0.005;
    s.rs_echo = v(EN_FXS_RS_ECHO) * 0.007;
    s.roll_t = if held(FX_ROLL) {
        s.roll_t + dt * SR / (4.0 * q.step_len)
    } else {
        0.0
    };
    s.rp_pit = clamp(v(EN_FXS_RP_PITCH), 0.0, 24.0) - 12.0;
    if s.bmode == 1 {
        s.rp_t += dt * SR / (4.0 * q.step_len);
    }
    s.rp_step = exp2(-clamp(v(EN_FXS_RP_DECAY), 0.0, 100.0) * 0.06 * (1.0 / 6.0206));
    s.rp_gatef = 1.0 - 0.25 * clamp(v(EN_FXS_RP_GATE), 0.0, 3.0);
    s.rp_ref = clamp(v(EN_FXS_RP_REFRESH), 0.0, 2.0) as u32;
    s.rp_ok = u32::from(
        (RB_N as f32 - q.bm_len * 0.5) * (1.0 / 22050.0)
            > (if s.rp_ref == 2 { 16.0 } else { 4.0 }) * q.step_len * (1.0 / SR),
    );
    let sm = min(dt * 12.0, 1.0);
    fade(&mut s.rm_m, held(FX_RING), sm);
    s.rm_inc = 40.0 * exp2(v(EN_FXS_RM_FREQ) * 0.0433) * (1.0 / SR);
    s.rm_wet = v(EN_FXS_RM_MIX) * 0.01;
    fade(&mut s.sw_m, held(FX_SWEEP), sm);
    if s.sw_m > 0.0 {
        let per = [128, 64, 32, 16, 8, 4, 2][clamp(v(EN_FXS_SW_RATE), 0.0, 6.0) as usize];
        let ph = ((step % per) as f32 * q.step_len + q.step_t as f32) / (per as f32 * q.step_len);
        let tri = if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 };
        let fc = 120.0 * exp2(v(EN_FXS_SW_DEPTH) * 0.07 * tri);
        if q.fxs_v[EN_FXS_SW_TYPE] == 2 {
            s.sw_c = svf_coefs(fc, 1.0);
            s.sw_c2 = svf_coefs(fc, 1.4 - 0.0128 * v(EN_FXS_SW_RESO));
        } else {
            s.sw_c = svf_coefs(fc, 1.4 - 0.0128 * v(EN_FXS_SW_RESO));
        }
    }
    fade(&mut s.tk_m, held(FX_TALK), sm);
    if s.tk_m > 0.0 {
        let per = [32, 16, 8, 4, 2][clamp(v(EN_FXS_TK_RATE), 0.0, 4.0) as usize];
        let ph = ((step % per) as f32 * q.step_len + q.step_t as f32) / (per as f32 * q.step_len);
        let tri = if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 };
        let pos = clamp(
            tri * 4.0 * (0.15 + 0.85 * v(EN_FXS_TK_DEPTH) * 0.01),
            0.0,
            4.0,
        );
        let k = 0.5 - 0.0042 * v(EN_FXS_TK_RESO);
        let i = if pos >= 4.0 { 3 } else { pos as usize };
        let fr = pos - i as f32;
        let f1 = [700.0, 530.0, 270.0, 570.0, 300.0];
        let f2 = [1220.0, 1840.0, 2290.0, 840.0, 870.0];
        s.tk_c[0] = svf_coefs(f1[i] + (f1[i + 1] - f1[i]) * fr, k);
        s.tk_c[1] = svf_coefs(f2[i] + (f2[i + 1] - f2[i]) * fr, k);
    }
    fade(&mut s.wh_m, held(FX_WAH), sm);
    fade(&mut s.tp_m, held(FX_TAPE), sm);
    fade(&mut s.fl_m, held(FX_FLANGER), sm);
    fade(&mut s.ph_m, held(FX_PHASER), sm);
    fade(&mut s.tfx_m, held(FX_TFILT), sm);
    fade(&mut s.tdx_m, held(FX_TDELAY), sm);
    fade(&mut s.tpx_m, held(FX_TPHASE), sm);
    let pt = |id: usize| {
        const PT: [u32; 10] = [255, 15, 240, 64, 16, 32, 128, 160, 192, 48];
        PT[if (q.fxs_v[id] as u32) < 10 {
            q.fxs_v[id] as usize
        } else {
            0
        }]
    };
    let bm = (if s.tfx_m > 0.0 { pt(EN_FXS_TF_TGT) } else { 0 })
        | (if s.tdx_m > 0.0 { pt(EN_FXS_TD_TGT) } else { 0 })
        | (if s.tpx_m > 0.0 { pt(EN_FXS_TP_TGT) } else { 0 });
    if bm != 0 && s.bus_m == 0 {
        s.tf_s.fill(0.0);
        s.tp_fb.fill(0.0);
        s.tp_z.fill([0.0; 4]);
    }
    s.bus_m = bm;
    if s.tfx_m > 0.0 {
        s.tf_c = svf_coefs(
            40.0 * exp2(v(EN_FXS_TF_CUT) * 0.0864 + q.lfo_tcut),
            2.0 - 0.0195 * v(EN_FXS_TF_RESO),
        );
        s.tf_type = if (q.fxs_v[EN_FXS_TF_TYPE] as u32) < 5 {
            q.fxs_v[EN_FXS_TF_TYPE] as u32
        } else {
            0
        };
    }
    if s.tpx_m > 0.0 {
        let oct = v(EN_FXS_TP_DEPTH) * 0.045;
        s.tp_ph += dt * 0.08 * exp2(v(EN_FXS_TP_RATE) * 0.05);
        s.tp_ph -= s.tp_ph as i32 as f32;
        for c in 0..2 {
            let x = sin_turns(s.tp_ph + 0.25 * c as f32) * 0.5 + 0.5;
            let w = min(3.14159265 * 250.0 * exp2(oct * x) * (1.0 / SR), 0.9);
            s.tp_a[c] = (1.0 - w) / (1.0 + w);
        }
    }
    s.ph_fbk = v(EN_FXS_PH_FB) * 0.009;
    s.ph_wet = v(EN_FXS_PH_MIX) * 0.005;
    s.ph_k0 = (1.0 - s.ph_wet) * 0.88;
    s.ph_k1 = s.ph_wet * 0.88 / (1.0 + 0.6 * s.ph_fbk);
    for c in 0..2 {
        s.ph_fb[c] = flush(s.ph_fb[c]);
        for k in 0..4 {
            s.ph_z[c][k] = flush(s.ph_z[c][k]);
        }
    }
    if s.ph_m > 0.0 {
        let oct = v(EN_FXS_PH_DEPTH) * 0.045;
        s.ph_ph += dt * 0.08 * exp2(v(EN_FXS_PH_RATE) * 0.05);
        s.ph_ph -= s.ph_ph as i32 as f32;
        for c in 0..2 {
            let x = sin_turns(s.ph_ph + 0.25 * c as f32) * 0.5 + 0.5;
            let w = min(3.14159265 * 250.0 * exp2(oct * x) * (1.0 / SR), 0.9);
            s.ph_a[c] = (1.0 - w) / (1.0 + w);
        }
    }
    s.fl_inc = 0.05 * exp2(v(EN_FXS_FL_RATE) * 0.06) * (1.0 / SR);
    s.fl_rng = v(EN_FXS_FL_DEPTH) * 1.1;
    s.fl_fbk = v(EN_FXS_FL_FB) * 0.0095;
    s.fl_wet = v(EN_FXS_FL_MIX) * 0.009;
    s.tp_wd = v(EN_FXS_TP_WOW);
    s.tp_fd = v(EN_FXS_TP_FLUT) * 0.2;
    s.tp_drive = 1.0 + v(EN_FXS_TP_DRIVE) * 0.025;
    s.tp_idrive = 1.0 / s.tp_drive;
    s.fl_inv = 1.0 / (1.0 + 0.85 * s.fl_wet * s.fl_m);
    s.tp_k = 1.0 - v(EN_FXS_TP_DULL) * 0.009;
    s.vn_g = v(EN_FXS_VN_LEVEL) * 0.0144;
    let pps = exp2(v(EN_FXS_VN_CRACK) * 0.07);
    s.vn_pop = pps * (1.0 / SR);
    s.vn_amp = if pps > 10.0 { sqrt(10.0 / pps) } else { 1.0 };
    s.vn_hiss = v(EN_FXS_VN_HISS) * 0.0012;
    s.nz_lp_k = 1.0 - exp(-6.28318530717959 * 300.0 * exp2(v(EN_FXS_VN_TONE) * 0.06) * (1.0 / SR));
    let sm = min(dt * 20.0, 1.0);
    let mut f = q.fxk_v[EN_FXK_FILTER] as f32 * 0.02;
    if f > -0.06 && f < 0.06 {
        f = 0.0;
    }
    s.lpk_s += ((if f < 0.0 { -f } else { 0.0 }) - s.lpk_s) * sm;
    s.hpk_s += ((if f > 0.0 { f } else { 0.0 }) - s.hpk_s) * sm;
    s.crk_s += (q.fxk_v[EN_FXK_CRUSH] as f32 * 0.01 - s.crk_s) * sm;
    s.thk_s += (q.fxk_v[EN_FXK_THROW] as f32 * 0.009 - s.thk_s) * sm;
    s.wash_s += (q.fxk_v[EN_FXK_WASH] as f32 * 0.01 - s.wash_s) * sm;
    s.lpk_s = flush(s.lpk_s);
    s.hpk_s = flush(s.hpk_s);
    s.crk_s = flush(s.crk_s);
    s.thk_s = flush(s.thk_s);
    s.wash_s = flush(s.wash_s);
    s.throw_amt = max(max(s.throw_key, s.thk_s), s.rs_echo * s.riser_amt);
    s.wash_e = max(s.wash_s, s.rs_wash * s.riser_amt);
    s.cr_hold = 1 + (7.0 * s.crk_s) as i32;
    s.cr_lv = exp2(11.0 - 7.0 * s.crk_s);
    s.cr_wet = min(s.crk_s * 3.0, 1.0);
    s.lp_amt_e = max(max(s.lp_amt, s.lpk_s), 0.35 * max(q.brk, 0.0));
    s.hp_amt_e = max(max(s.hp_amt, s.hpk_s), s.rs_filt * s.riser_amt);
    s.echo_fbx =
        q.echo_fb + (0.86 - q.echo_fb) * max(s.throw_amt, s.tdx_m * v(EN_FXS_TD_FB) * 0.0095);
    let mut action = 0;
    if s.fill_hold != u32::from(held(FX_FILL)) {
        s.fill_hold = u32::from(held(FX_FILL));
        action = 0x100;
    }
    if s.lp_amt_e > 0.0 {
        s.lp_c = svf_coefs(16000.0 * exp2(-6.0 * s.lp_amt_e), 0.8);
    }
    if s.hp_amt_e > 0.0 {
        s.hp_c = svf_coefs(20.0 * exp2(6.3 * s.hp_amt_e), 0.8);
    }
    for t in 0..4 {
        s.lp_s[t] = flush(s.lp_s[t]);
        s.hp_s[t] = flush(s.hp_s[t]);
    }
    let mut want: Option<usize> = None;
    for (t, bit) in [
        FX_RPT2, FX_RPT4, FX_RPT6, FX_RPT8, FX_RPT16, FX_RPT32, FX_RPT8T, FX_RPT16T,
    ]
    .iter()
    .enumerate()
    {
        if held(*bit) && want.is_none_or(|w| RPT[t] < RPT[w]) {
            want = Some(t);
        }
    }
    if !held(FX_RVSWELL) {
        s.rv_held = 0;
        s.rvs_arm = 0;
    }
    if s.roll_t > 0.0 {
        let start = clamp(v(EN_FXS_RL_START), 0.0, 6.0) as i32;
        let end = clamp(v(EN_FXS_RL_END), 0.0, 6.0) as i32;
        let stage =
            (s.roll_t / [0.5, 1.0, 2.0, 4.0][clamp(v(EN_FXS_RL_TIME), 0.0, 3.0) as usize]) as i32;
        let rr = if start < end {
            (start + stage).min(end)
        } else {
            (start - stage).max(end)
        };
        let r = FX_RPT2 + if rr == 6 { 8 } else { rr as usize };
        if want.is_none_or(|w| RPT[r] < RPT[w]) {
            want = Some(r);
        }
        s.rl_gain = 1.0 + v(EN_FXS_RL_SWELL) * 0.008 * min(s.roll_t * 0.25, 1.0);
    } else {
        s.rl_gain = 1.0;
    }
    if !held(FX_FLEX) {
        s.flx_held = 0;
        s.flx_wait_rel = 0;
        if s.flx_arm != 0 {
            s.flx_arm = 0;
            s.en_flx_state = if q.flx_active != 0 { 2 } else { 0 };
        }
    }
    let no_grid = q.en_playing == 0 || q.en_ext != 0;
    if held(FX_STOP) {
        s.rpt_arm = 0;
        if s.bmode != 3 || s.bm_tgt < 1.0 {
            s.bmode = 3;
            s.bm_start = q.rb_w.wrapping_sub(2);
            s.bm_sm = s.bm_start % RB_N;
            s.bm_off = 0.0;
            s.bm_speed = 1.0;
            s.bm_tgt = 1.0;
        }
    } else if held(FX_REV) {
        s.rpt_arm = 0;
        if s.bmode != 2 || s.bm_tgt < 1.0 {
            s.bmode = 2;
            s.bm_len = (RB_N - 8) as f32 * 2.0;
            s.bm_start = q.rb_w.wrapping_sub(RB_N - 8);
            s.bm_sm = s.bm_start % RB_N;
            s.bm_off = s.bm_len - 1.0;
            s.bm_tgt = 1.0;
        }
    } else if held(FX_FLEX) {
        s.rpt_arm = 0;
        s.scat_arm = 0;
        s.var_exit = 0;
        s.flx_held = 1;
        if q.flx_active != 0 {
            s.bm_tgt = 0.01 * clamp(v(EN_FXS_FX_MIX), 0.0, 100.0);
        }
        if s.flx_wait_rel == 0
            && (if q.flx_active != 0 {
                q.flx_pat != q.flx_sel && s.flx_arm != q.flx_sel as u32 + 1
            } else {
                s.flx_arm != q.flx_sel as u32 + 1
            })
        {
            s.flx_arm = q.flx_sel as u32 + 1;
            s.en_flx_state = 1;
            if no_grid {
                action |= 1;
            }
        }
    } else if held(FX_SCATTER) {
        s.rpt_arm = 0;
        s.var_exit = 0;
        if s.bmode != 4 || s.bm_tgt < 1.0 {
            if no_grid {
                action |= 2;
            } else {
                s.scat_arm = 1;
            }
        }
    } else if held(FX_BRAKE) || held(FX_SPIN) || held(FX_HALF) {
        let wh = if held(FX_BRAKE) {
            FX_BRAKE
        } else if held(FX_SPIN) {
            FX_SPIN
        } else {
            FX_HALF
        };
        s.rpt_arm = 0;
        s.scat_arm = 0;
        s.var_exit = 0;
        action |= if s.bmode != 5 || s.bm_tgt < 1.0 || q.vr_which != wh as i32 {
            3
        } else {
            4
        };
        action |= (wh as u32) << 16;
    } else if held(FX_SCRATCH) {
        s.rpt_arm = 0;
        s.scat_arm = 0;
        s.var_exit = 0;
        if s.bmode != 6 || s.bm_tgt < 1.0 {
            action |= 5;
        }
    } else if held(FX_FREEZE) {
        s.rpt_arm = 0;
        s.scat_arm = 0;
        s.var_exit = 0;
        if s.bmode != 7 || s.bm_tgt < 1.0 {
            action |= 6;
        }
    } else if held(FX_RVSWELL) {
        s.rpt_arm = 0;
        s.scat_arm = 0;
        s.var_exit = 0;
        if s.rv_held == 0 {
            s.rv_held = 1;
            if no_grid {
                action |= 7;
            } else {
                s.rvs_arm = 1;
            }
        }
    } else if s.bmode == 5 && s.bm_tgt >= 1.0 {
        s.vr_tgt = 1.0;
        s.vr_c = 1.0 - exp(-3.0 / (0.02 * exp2(v(EN_FXS_BR_RET) * 0.06) * SR));
        if no_grid {
            s.bm_tgt = 0.0;
        } else {
            s.var_exit = 1;
        }
    } else if let Some(want) = want {
        if s.bmode != 1 || q.brate != want as i32 || s.bm_tgt < 1.0 {
            if no_grid {
                action |= 8 | ((want as u32) << 16);
            } else {
                s.rpt_arm = want as u32 + 1;
            }
        }
    } else if q.flx_active == 0 {
        s.rpt_arm = 0;
        s.bm_tgt = 0.0;
    }
    action
}
