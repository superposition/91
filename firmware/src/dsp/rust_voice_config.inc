/* SPDX-License-Identifier: GPL-3.0-only */
static fm1_voice_drumsinput_t voice_drums_input(void)
{
    const fm1_voice_drumsinput_t r={pulse_vol,drive_k,{lane_g[0],lane_g[1],lane_g[2],lane_g[3],lane_g[4],lane_g[5]},{hat_inc[0],hat_inc[1],hat_inc[2],hat_inc[3]},{trk_g[TR_KICK],trk_g[TR_SNARE],trk_g[TR_HATS],trk_g[TR_PERC]},{trk_g1[TR_KICK],trk_g1[TR_SNARE],trk_g1[TR_HATS],trk_g1[TR_PERC]},tg_inv};return r;
}
static fm1_voice_pluckinput_t voice_pluck_input(void)
{
    const fm1_voice_pluckinput_t r={cm_vol,hb_pf,trk_g[TR_CM],trk_g1[TR_CM],tg_inv,en_cm_level,HB(EN_V_PLUCK)};return r;
}
#if EN_RUST_VOICES
static fm1_voice_drums_config_t voice_drums_config(void)
{
    const fm1_voice_drumsinput_t raw=voice_drums_input();fm1_voice_drums_config_t c;const kit_t *kits[4]={KK,KS,KH,KP};fm1_rust_voice_drums_config(&raw,kits,&c);return c;
}
#else
static fm1_voice_drums_config_t voice_drums_config(void)
{
    const float g=pulse_vol*1.1f;
    const fm1_voice_drums_config_t c={{g*lane_g[L_KICK]*1.15f,g*0.38f*lane_g[L_SNARE],g*0.42f*lane_g[L_CLAP],g*0.22f*lane_g[L_HAT],g*0.18f*lane_g[L_OHAT],g*0.3f*lane_g[L_PERC]},
        {hat_inc[0],hat_inc[1],hat_inc[2],hat_inc[3]},drive_k*KK->k_drive,KK->k_drop,KK->k_click,KK->k_rumble,KS->sn_body,KS->sn_noise,KS->cl_gap,KH->h_noise,
        {trk_g[TR_KICK],trk_g[TR_SNARE],trk_g[TR_HATS],trk_g[TR_PERC]},{trk_g1[TR_KICK],trk_g1[TR_SNARE],trk_g1[TR_HATS],trk_g1[TR_PERC]},tg_inv};
    return c;
}
#endif
#if EN_RUST_VOICES
static fm1_voice_pluck_config_t voice_pluck_config(void)
{
    const fm1_voice_pluckinput_t raw=voice_pluck_input();fm1_voice_pluck_config_t c;fm1_rust_voice_pluck_config(par,&raw,&c);return c;
}
#else
static fm1_voice_pluck_config_t voice_pluck_config(void)
{
    const int sn=par[P_CM_SND]<0?0:par[P_CM_SND]>=CM_NPLAIN?CM_NPLAIN-1:par[P_CM_SND];const cm_snd_t *s=&CM_SOUND[sn];
    const fm1_voice_pluck_config_t c={cm_vol*0.5f*s->g,HB(EN_V_PLUCK)?hb_pf:1.0f,s->fr,s->i0,s->i1,trk_g[TR_CM],trk_g1[TR_CM],tg_inv,en_cm_level,s->rise,(uint32_t)(par[P_CM_SND]>=CM_NPLAIN)};
    return c;
}
#endif
