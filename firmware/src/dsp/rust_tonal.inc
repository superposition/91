/* SPDX-License-Identifier: GPL-3.0-only */
/* Only existing scalar values cross this capture boundary. */
static fm1_voice_droneinput_t voice_drone_input(void)
{
    const fm1_voice_droneinput_t r={en_playing,bass.play,bl_slide,HB(EN_V_DRONE),fxs_v[EN_FXS_FT_BASS],fxs_v[EN_FXS_SQ_TGT],fxs_v[EN_FXS_SQ_DRIVE],fxs_v[EN_FXS_SQ_AMT],bass_vol,reso_k,bass.env,bass.vd,drone_lfo.v,mod[9].v,arcm,motion,brk,bass_duck,pulse_vol,lk_kick,hb_semi,tunefac,trk_g[TR_DRONE],trk_g1[TR_DRONE],tg_inv};return r;
}
static fm1_voice_acidinput_t voice_acid_input(void)
{
    const fm1_voice_acidinput_t r={fxs_v[EN_FXS_FT_ACID],fxs_v[EN_FXS_SQ_TGT],fxs_v[EN_FXS_SQ_DRIVE],fxs_v[EN_FXS_SQ_AMT],fxs_v[EN_FXS_SQ_WOW],acid.sliding,HB(EN_V_ACID),acid_vol,lfo_acut,lfo_ares,acid.acc,hb_pf,trk_g[TR_ACID],trk_g1[TR_ACID],tg_inv};return r;
}
#if EN_RUST_VOICES
static fm1_voice_drone_config_t voice_drone_config(void)
{
    const fm1_voice_droneinput_t raw=voice_drone_input();fm1_voice_drone_config_t c;fm1_rust_voice_drone_config(par,&raw,&c);return c;
}
static fm1_voice_acid_config_t voice_acid_config(void)
{
    const fm1_voice_acidinput_t raw=voice_acid_input();fm1_voice_acid_config_t c;fm1_rust_voice_acid_config(par,&raw,&c);return c;
}
#else
/* SPDX-License-Identifier: GPL-3.0-only */
/* Scalar block inputs. Keep original f32 association across the ABI seam. */
static fm1_voice_drone_config_t voice_drone_config(void)
{
    const drone_snd_t *s=&DRONE_SOUND[(unsigned)par[P_DR_INST]<EN_NSOUND?par[P_DR_INST]:0];
    const int dest=par[P_DL_DEST],sq=fxs_v[EN_FXS_SQ_TGT]>=1,bl_on=par[P_BL_MODE]&&!bass.play;
    const float sqd=sq?(float)fxs_v[EN_FXS_SQ_DRIVE]*0.01f:0.0f,sqa=sq?(float)fxs_v[EN_FXS_SQ_AMT]*0.01f:0.0f;
    const float depth=(float)par[P_DL_DEPTH]*0.01f,lv=dest==2?1.0f-0.8f*depth*(0.5f-0.5f*drone_lfo.v):1.0f;
    const float target=(en_playing&&!par[P_DR_MAN])||bass.play?1.0f:0.0f;
    float glide=bass.play?GLIDE_K*12.0f:GLIDE_K;
    if(bl_on)glide=par[P_BL_GLIDE]?1.0f/((0.01f+0.004f*(float)par[P_BL_GLIDE])*EN_SR):bl_slide?1.0f/(0.06f*EN_SR):1.0f;
    const fm1_voice_drone_config_t c={s->osc,fxs_v[EN_FXS_FT_BASS],(uint32_t)bl_on,en_playing,
        1.4f*s->drive*(dest==3?1.0f+2.5f*depth*(0.5f+0.5f*drone_lfo.v):1.0f)*(1.0f+3.0f*sqd),
        reso_k*(1.0f-0.93f*sqa)*(dest==4?1.0f-0.85f*depth*(0.5f+0.5f*drone_lfo.v):1.0f),1.4f*(1.0f-0.75f*sqa),1.0f+2.0f*sqd,
        bass_vol*0.36f*lv*s->g/fm_minf(1.0f+0.15f*(s->drive-1.0f),1.3f)*(1.0f+bass.vd)/(1.0f+0.5f*sqd+0.4f*sqa),s->saw,s->sub,target,
        target>0.0f?1.0f/((bass.play?0.03f:2.0f)*EN_SR):1.0f/((par[P_DR_MAN]?0.3f:4.0f)*EN_SR),
        0.5f+2.2f*bass.env+0.3f*arcm,0.5f+0.3f*mod[9].v,glide,dest==1?depth*drone_lfo.v:0.0f,HB(EN_V_DRONE)?hb_semi:0.0f,tunefac,
        DRONE_FR[(unsigned)par[P_DR_INST]<EN_NSOUND?par[P_DR_INST]:0],
        (float)par[P_DR_CUT]*0.075f-1.5f+s->cut+(dest==0?2.5f*depth*drone_lfo.v:0.0f),4.0f*(float)par[P_DE_AMT]*0.01f,
        0.8f*motion*mod[9].v,0.8f*arcm,2.0f*fm_maxf(brk,0.0f),1.6f*fm_minf(brk,0.0f),0.35f*bass_duck*pulse_vol,
        4.0f*(float)par[P_LK_KICK]*0.01f*lk_kick,trk_g[TR_DRONE],trk_g1[TR_DRONE],tg_inv};
    return c;
}
static fm1_voice_acid_config_t voice_acid_config(void)
{
    const int sn=(unsigned)par[P_AC_WAVE]<EN_NACID?par[P_AC_WAVE]:0;const acid_snd_t *s=&ACID_SOUND[sn];
    const int sq=fxs_v[EN_FXS_SQ_TGT]!=1;
    const float sqd=sq?(float)fxs_v[EN_FXS_SQ_DRIVE]*0.01f:0.0f,sqa=sq?(float)fxs_v[EN_FXS_SQ_AMT]*0.01f:0.0f,sqw=sq?(float)fxs_v[EN_FXS_SQ_WOW]*0.01f:0.0f;
    const float res=(float)par[P_AC_RES]*0.01f,accamt=(float)par[P_AC_ACC]*0.01f;
    const fm1_voice_acid_config_t c={s->osc,fxs_v[EN_FXS_FT_ACID],acid_vol*0.75f*s->g/(1.0f+0.5f*sqd+0.4f*sqa),s->drive,res,
        (float)par[P_AC_ENV]*0.06f,accamt,(float)par[P_AC_CUT]*0.066f+lfo_acut,
        (1.45f-1.38f*res)*(1.0f-0.93f*sqa)*(1.0f-0.85f*lfo_ares),1.2f*(1.0f-0.75f*sqa),1.0f+3.0f*sqd,sqw,
        0.75f+0.5f*accamt*acid.acc,acid.sliding?0.0012f:1.0f,HB(EN_V_ACID)?hb_pf:1.0f,trk_g[TR_ACID],trk_g1[TR_ACID],tg_inv,
        ACID_FM[sn].fr,ACID_FM[sn].i0,ACID_FM[sn].i1,ACID_FM[sn].rise};return c;
}

#endif
