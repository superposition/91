/* SPDX-License-Identifier: GPL-3.0-only */
/* Scoped scalar copies only; all 2x512-sample delay payloads remain in place. */
#define FX_TONE_STATE(X) X(wh_env) X(wh_cnt) X(mod_w) X(tp_ph) X(tp_ph2) X(tp_d) X(tp_dd) X(fl_ph) X(fl_d) X(fl_dd) X(rm_ph) X(crush_n) X(crush_l) X(crush_r)
#define FX_TONE_CONFIG(X) X(lp_amt_e) X(hp_amt_e) X(sw_m) X(tk_m) X(wh_m) X(tp_m) X(fl_m) X(ph_m) X(tp_wd) X(tp_fd) X(tp_drive) X(tp_idrive) X(tp_k) X(fl_inc) X(fl_rng) X(fl_fbk) X(fl_wet) X(fl_inv) X(ph_fbk) X(ph_k0) X(ph_k1) X(rm_m) X(rm_inc) X(rm_wet) X(cr_wet) X(cr_lv) X(cr_hold)
static void fx_tone_read(fm1_fx_tone_t *s,fm1_fx_tone_config_t *c)
{
    unsigned i,j,k;
#define READ_STATE(n) s->n=n;
    FX_TONE_STATE(READ_STATE)
#undef READ_STATE
#define READ_CONFIG(n) c->n=n;
    FX_TONE_CONFIG(READ_CONFIG)
#undef READ_CONFIG
    for(i=0;i<4;i++){
        s->lp_s[i]=lp_s[i];s->hp_s[i]=hp_s[i];s->wh_c[i]=wh_c[i];
        c->lp_c[i]=lp_c[i];c->hp_c[i]=hp_c[i];c->sw_c[i]=sw_c[i];c->sw_c2[i]=sw_c2[i];
    }
    for(i=0;i<2;i++){
        s->tp_lp[i]=tp_lp[i];s->ph_fb[i]=ph_fb[i];c->ph_a[i]=ph_a[i];
        for(j=0;j<4;j++){s->ph_z[i][j]=ph_z[i][j];c->tk_c[i][j]=tk_c[i][j];}
        for(j=0;j<2;j++){
            s->sw_s[i][j]=sw_s[i][j];s->sw_s2[i][j]=sw_s2[i][j];s->wh_s[i][j]=wh_s[i][j];
            for(k=0;k<2;k++)s->tk_s[i][j][k]=tk_s[i][j][k];
        }
    }
    c->sw_type=fxs_v[EN_FXS_SW_TYPE];c->sw_reso=fxs_v[EN_FXS_SW_RESO];c->tk_mix=fxs_v[EN_FXS_TK_MIX];
    c->wh_mix=fxs_v[EN_FXS_WH_MIX];c->wh_sens=fxs_v[EN_FXS_WH_SENS];c->wh_range=fxs_v[EN_FXS_WH_RANGE];c->wh_reso=fxs_v[EN_FXS_WH_RESO];
}
static void fx_tone_write(const fm1_fx_tone_t *s)
{
    unsigned i,j,k;
#define WRITE_STATE(n) n=s->n;
    FX_TONE_STATE(WRITE_STATE)
#undef WRITE_STATE
    for(i=0;i<4;i++){lp_s[i]=s->lp_s[i];hp_s[i]=s->hp_s[i];wh_c[i]=s->wh_c[i];}
    for(i=0;i<2;i++){
        tp_lp[i]=s->tp_lp[i];ph_fb[i]=s->ph_fb[i];
        for(j=0;j<4;j++)ph_z[i][j]=s->ph_z[i][j];
        for(j=0;j<2;j++){
            sw_s[i][j]=s->sw_s[i][j];sw_s2[i][j]=s->sw_s2[i][j];wh_s[i][j]=s->wh_s[i][j];
            for(k=0;k<2;k++)tk_s[i][j][k]=s->tk_s[i][j][k];
        }
    }
}
static void fx_tone(float *left,float *right)
{
    fm1_fx_tone_t s;fm1_fx_tone_config_t c;float output[2];
    fx_tone_read(&s,&c);
    fm1_rust_fx_tone(&s,&c,&tp_buf[0][0],&fl_buf[0][0],*left,*right,output);
    fx_tone_write(&s);*left=output[0];*right=output[1];
}
#undef FX_TONE_STATE
#undef FX_TONE_CONFIG
