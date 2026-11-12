/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef PAD_JOB_GENRE
#define PAD_JOB_GENRE (chord_genre) /* frozen standalone oracle adapter */
#endif
static fm1_voice_pad_breath_t voice_pad_breath_config(void)
{
    const fm1_voice_pad_breath_t c={pe_atk_s(par[P_PE_ATK]),pe_hold_s(par[P_PE_HOLD]),pe_rel_s(par[P_PE_REL]),pe_rest_s(par[P_PE_REST]),en_playing,(uint32_t)par[P_PAD_MAN],(uint32_t)pad_osc()};return c;
}
static fm1_voice_pad_config_t voice_pad_config(void)
{
    const fm1_voice_pad_config_t c={
        {pe_atk_s(par[P_PE_ATK]),pe_hold_s(par[P_PE_HOLD]),pe_rel_s(par[P_PE_REL]),pe_rest_s(par[P_PE_REST]),en_playing,(uint32_t)par[P_PAD_MAN],(uint32_t)pad_osc()},
        pad_vol*0.5f*pad_lvl*(1.0f+0.01f*(float)SECT[PAD_JOB_GENRE].pad*fm_maxf(brk,0.0f))*PAD_SOUND[pad_snd()].g*(1.0f+0.35f*(float)par[P_LK_SNARE]*0.01f*pad_lk),pad_gate,
        en_jam_on&&!par[P_DETUNE]?0.0f:detune,motion,{mod[0].v,mod[1].v,mod[2].v,mod[3].v,mod[4].v,mod[5].v},pad_pitch,bright,arcm,pad_filt,pad_pan,PAD_SOUND[pad_snd()].fc,tunefac,stab_k,jp_swell_env,
        {BREATH[0],BREATH[1],BREATH[2],BREATH[3]},{{VOWEL[0][0],VOWEL[0][1]},{VOWEL[1][0],VOWEL[1][1]},{VOWEL[2][0],VOWEL[2][1]},{VOWEL[3][0],VOWEL[3][1]}},(uint32_t)pad_stabs(),(uint32_t)jp_dx(),fxs_v[EN_FXS_FT_PAD]};return c;
}
static fm1_voice_pad_bus_t voice_pad_bus_config(void)
{
    const fm1_voice_pad_bus_t c={PAD_SOUND[pad_snd()].fc,bright,arcm,pad_filt,(float)par[P_LK_SNARE],pad_lk,trk_g[TR_PAD],trk_g1[TR_PAD],tg_inv,rv_job.pad_w};return c;
}
