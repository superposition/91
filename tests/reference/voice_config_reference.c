/* SPDX-License-Identifier: GPL-3.0-only */
/* Input captures and frozen scalar expressions, never a rewritten baseline. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "amb.c"
#include "../../firmware/src/dsp/rust_voices.h"
#if !EN_RUST_VOICES
#include "../../firmware/src/dsp/rust_voice_config.inc"
#include "../../firmware/src/dsp/rust_tonal.inc"
#include "../../firmware/src/dsp/rust_pad.inc"
#endif
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static uint32_t random_state=0x91271903u;
static uint32_t next(void){random_state^=random_state<<13;random_state^=random_state>>17;random_state^=random_state<<5;return random_state;}
static float unit(void){return (float)(next()>>8)*(1.0f/16777216.0f);}
static void words(const void *p,size_t n){size_t i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++){uint32_t w;memcpy(&w,b+4*i,4);printf("%s%u",i?",":"",w);}putchar(']');}
static void parameters(void){unsigned i;putchar('[');for(i=0;i<P_NPARAMS;i++)printf("%s%d",i?",":"",(int)par[i]);putchar(']');}
static void setup(unsigned i){
    unsigned j;static const int controls[]={P_PE_ATK,P_PE_HOLD,P_PE_REL,P_PE_REST,P_DL_DEPTH,P_DR_CUT,P_DE_AMT,P_AC_RES,P_AC_ACC,P_AC_ENV,P_AC_CUT,P_LK_KICK,P_LK_SNARE,P_DETUNE};
    for(j=0;j<sizeof controls/sizeof controls[0];j++)par[controls[j]]=(int16_t)((i/6+j*17)%101);
    par[P_DR_INST]=(int16_t)(i/6%26==24?-1:i/6%26==25?99:i/6%26);par[P_AC_WAVE]=(int16_t)(i/6%33==31?-1:i/6%33==32?99:i/6%33);par[P_PADTONE]=(int16_t)(i/6%26==24?-1:i/6%26==25?99:i/6%26);par[P_CM_SND]=(int16_t)(i/6%15==13?-1:i/6%15==14?99:i/6%15);
    par[P_DL_DEST]=(int16_t)(i/6%8);par[P_BL_MODE]=(int16_t)(i/6%3);par[P_BL_GLIDE]=(int16_t)(i/6%3==0?0:i/6%3==1?1:100);par[P_DR_MAN]=(int16_t)(i/6%2);par[P_PAD_MAN]=(int16_t)(i/12%2);par[P_PAD_COMP]=(int16_t)(i/6%11);
    fxs_v[EN_FXS_SQ_TGT]=(int16_t)(i/6%3);fxs_v[EN_FXS_SQ_DRIVE]=(int16_t)(i/6%101);fxs_v[EN_FXS_SQ_AMT]=(int16_t)((i/6*7)%101);fxs_v[EN_FXS_SQ_WOW]=(int16_t)((i/6*11)%101);
    fxs_v[EN_FXS_FT_PAD]=fxs_v[EN_FXS_FT_BASS]=fxs_v[EN_FXS_FT_ACID]=(int16_t)(i/6%5);
    en_playing=(uint8_t)(i/6%2);en_jam_on=(uint8_t)(i/12%2);jp_data=JP_SONG_0;chord_genre=(int)(i/6%EN_NGENRE);
    bass.play=(uint8_t)(i/24%2);bl_slide=(uint8_t)(i/48%2);acid.sliding=(uint8_t)(i/6%2);hb_voice=(uint8_t)(i/6%5);hb_semi=unit()*4.0f-2.0f;hb_pf=fm_exp2f(hb_semi/12.0f);
    bass_vol=unit();reso_k=unit()*1.5f;bass.env=unit();bass.vd=unit()*0.5f;drone_lfo.v=unit()*2.0f-1.0f;arcm=unit()*2.0f-1.0f;motion=unit();brk=unit()*2.0f-1.0f;bass_duck=unit();pulse_vol=unit();lk_kick=unit();tunefac=fm_exp2f((unit()*2.0f-1.0f)/12.0f);
    acid_vol=unit();lfo_acut=unit()*5.0f-2.5f;lfo_ares=unit();acid.acc=unit()*1.2f;
    pad_vol=unit();pad_lvl=unit();pad_gate=unit();detune=unit()*0.015f;pad_pitch=unit()-0.5f;bright=unit()*100.0f;pad_filt=unit()*2.0f-1.0f;pad_pan=unit()*1.8f-0.9f;stab_k=unit();jp_swell_env=unit();pad_lk=unit();rv_job.pad_w=next();
    cm_vol=unit();en_cm_level=unit();drive_k=1.0f+unit()*3.0f;
    for(j=0;j<EN_NTRACK;j++){trk_g[j]=unit();trk_g1[j]=unit();}tg_inv=1.0f/(float)(1+i/6%256);
    for(j=0;j<6;j++){mod[j].v=unit()*2.0f-1.0f;lane_g[j]=unit();}mod[9].v=unit()*2.0f-1.0f;
    for(j=0;j<4;j++)hat_inc[j]=HAT_INC[j]*(0.5f+unit()*1.5f);
    KK=&KITS[i/6%EN_NKIT];KS=&KITS[(i/6+3)%EN_NKIT];KH=&KITS[(i/6+7)%EN_NKIT];KP=&KITS[(i/6+11)%EN_NKIT];
}
int main(void){unsigned i;en_init();printf("{\"cases\":[");for(i=0;i<1536;i++){setup(i);printf("%s{\"test\":%u,\"kind\":%u,\"params\":",i?",":"",i,i%6);parameters();printf(",\"kits\":[%u,%u,%u,%u],\"raw\":",i/6%EN_NKIT,(i/6+3)%EN_NKIT,(i/6+7)%EN_NKIT,(i/6+11)%EN_NKIT);
    switch(i%6){
#define C(K,T) {const fm1_voice_##T##_t raw=voice_##K##_input();const fm1_voice_##K##_config_t result=voice_##K##_config();words(&raw,sizeof raw/4);printf(",\"config\":");words(&result,sizeof result/4);break;}
    case 0:C(drone,droneinput)
    case 1:C(acid,acidinput)
    case 2:C(pad,padinput)
    case 3:{const fm1_voice_padbusinput_t raw=voice_pad_bus_input();const fm1_voice_pad_bus_t result=voice_pad_bus_config();words(&raw,sizeof raw/4);printf(",\"config\":");words(&result,sizeof result/4);break;}
    case 4:C(drums,drumsinput)
    default:C(pluck,pluckinput)
    }putchar('}');}puts("]}");return 0;}
