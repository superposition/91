/* SPDX-License-Identifier: GPL-3.0-only */
#define FX_SAMPLER_STATE(X) X(rb_w) X(rb_wi) X(bm_sm) X(rb_odd) X(rb_hl) X(rb_hr) X(bmode) X(brate) X(rp_rate) X(rp_gain) X(rp_pos) X(bm_start) X(bm_off) X(bm_speed) X(bm_mix) X(bm_tgt) X(sc_pos) X(sc_off) X(sc_rev) X(sc_k) X(scr_t) X(flx_p) X(rv_pos) X(vr_rate)
#define FX_SAMPLER_CONFIG(X) X(rp_step) X(rp_gatef) X(rp_t) X(rp_pit) X(rp_ref) X(rp_ok) X(bm_len) X(sc_len) X(sc_n) X(scr_w) X(scr_per) X(flx_live) X(flx_mute) X(flx_rev) X(flx_off) X(flx_sub) X(gn_len) X(gn_region) X(gn_scale) X(gn_n) X(rv_len) X(vr_tgt) X(vr_c) X(vr_dull) X(rl_gain)
static void fx_sampler(float *left,float *right)
{
    fm1_fx_sampler_t s;fm1_fx_sampler_config_t c;float output[2];unsigned i;
#define READ_STATE(n) s.n=n;
    FX_SAMPLER_STATE(READ_STATE)
#undef READ_STATE
#define READ_CONFIG(n) c.n=n;
    FX_SAMPLER_CONFIG(READ_CONFIG)
#undef READ_CONFIG
    for(i=0;i<4;i++){s.gn_pos[i]=gn_pos[i];s.gn_st[i]=gn_st[i];}
    s.vr_lp[0]=vr_lp[0];s.vr_lp[1]=vr_lp[1];
    c.sc_chaos=(float)fxs_v[EN_FXS_SC_CHAOS]*0.01f;c.sc_reverse=(float)fxs_v[EN_FXS_SC_REV]*0.01f;
    c.scr_style=fxs_v[EN_FXS_SR_STYLE];c.step_length=step_len;
    fm1_rust_fx_sampler(&s,&c,rb,&rng,*left,*right,output);
#define WRITE_STATE(n) n=s.n;
    FX_SAMPLER_STATE(WRITE_STATE)
#undef WRITE_STATE
    for(i=0;i<4;i++){gn_pos[i]=s.gn_pos[i];gn_st[i]=s.gn_st[i];}
    vr_lp[0]=s.vr_lp[0];vr_lp[1]=s.vr_lp[1];*left=output[0];*right=output[1];
}
#undef FX_SAMPLER_STATE
#undef FX_SAMPLER_CONFIG
