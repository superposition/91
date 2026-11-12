/* SPDX-License-Identifier: GPL-3.0-only */
/* Real selected drone/acid kernels, with immutable R01 as the independent oracle. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>
#include <assert.h>
#include "amb.c"
#include "../../firmware/src/dsp/rust_voices.h"
#if !EN_RUST_VOICES
#include "../../firmware/src/dsp/rust_tonal.inc"
#endif
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static FILE *trace;static uint32_t hash;static float peak;
static const unsigned sizes[]={0,1,31,32,33,127,128,129,256};
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
static void words(const void *p,size_t n){unsigned i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++){uint32_t v;memcpy(&v,b+4*i,4);printf("%s%u",i?",":"",v);}putchar(']');}
static void setup(unsigned test){
    unsigned variant=test<100?test%4:(test-100)%4,amount=variant==3?100:variant*33;
    en_init();en_jam_on=0;en_playing=variant!=3;apply(P_MIDI,0);
    memset(&bass,0,sizeof bass);memset(&acid,0,sizeof acid);bl_env=0.4f;bl_gate=100.0f;bl_up=1;bl_slide=variant&1;
    apply(P_BASS,100);apply(P_ACID,100);apply(P_DR_MAN,variant==3);apply(P_BL_MODE,variant==2?1:0);apply(P_BL_GLIDE,(int)amount);
    apply(P_DR_INST,test<96?(int)(test/4):0);apply(P_DR_CUT,(int)amount);apply(P_DR_RES,(int)amount);apply(P_DE_AMT,(int)amount);apply(P_DL_DEST,(int)(test%5));apply(P_DL_DEPTH,75);apply(P_LK_KICK,90);
    apply(P_AC_WAVE,test>=100&&test<224?(int)((test-100)/4):0);apply(P_AC_RES,(int)((test*17+variant*33)%101));apply(P_AC_ENV,(int)amount);apply(P_AC_ACC,100);apply(P_AC_CUT,(int)amount);
    fxs_v[EN_FXS_FT_BASS]=(int16_t)(test%5);fxs_v[EN_FXS_FT_ACID]=(int16_t)(test%5);fxs_v[EN_FXS_SQ_TGT]=2;fxs_v[EN_FXS_SQ_DRIVE]=(int16_t)amount;fxs_v[EN_FXS_SQ_AMT]=(int16_t)amount;fxs_v[EN_FXS_SQ_WOW]=(int16_t)amount;
    mod[9].v=0.3f;drone_lfo.v=-0.6f;arcm=0.35f;motion=0.6f;brk=variant&1?0.8f:-0.6f;bass_duck=0.3f;lk_kick=0.7f;lfo_acut=0.4f;lfo_ares=0.2f;
    trk_g[TR_DRONE]=0.2f;trk_g1[TR_DRONE]=0.9f;trk_g[TR_ACID]=0.4f;trk_g1[TR_ACID]=0.8f;tg_inv=1.0f/256.0f;
    bass.ph[0]=0.98f;bass.ph[1]=0.37f;bass.ph[2]=0.75f;bass.sph=0.6f;bass.mph=0.96f;
    bass.note=36.0f+12.0f*(float)variant;bass.target=bass.note+7.0f;bass.amp=0.65f;bass.env=0.8f;bass.play=variant==1;bass.vd=0.1f;
    svf_coefs(bass.c1,1000.0f,0.5f);svf_coefs(bass.c2,1300.0f,1.0f);
    acid.ph=0.98f;acid.ps=0.7f;acid.p3=0.4f;acid.p4=0.2f;acid.p5=0.9f;acid.lph=0.99f;
    acid.inc=mtof(36.0f+12.0f*(float)variant)*(1.0f/EN_SR);acid.inc_t=mtof(43.0f+12.0f*(float)variant)*(1.0f/EN_SR);
    acid.amp=0.6f;acid.amp_t=variant==3?0.0f:1.0f;acid.env=0.95f;acid.edec=0.985f;acid.acc=1.0f;acid.sliding=(uint8_t)(variant&1);acid.gate_left=variant==0?500:variant==1?-1:variant==2?1:0;
    svf_coefs(acid.c1,1500.0f,0.5f);svf_coefs(acid.c2,1725.0f,1.0f);
    hb_voice=(uint8_t)(test<100?EN_V_DRONE:EN_V_ACID);hb_semi=variant==1?1.5f:0.0f;hb_pf=variant==1?fm_exp2f(1.5f/12.0f):1.0f;
    dr_ctl=ac_ctl=variant==0?0:variant==1?7:variant==2?31:0xfffffff0u;
    if(test==96){bass.amp=0.0f;bass.play=0;en_playing=0;}
    if(test==224){acid.amp=acid.amp_t=0.0f;}
}
static uint32_t run(unsigned test,unsigned blocks,int timed){
    unsigned b,j;uint32_t frames=0;hash=2166136261u;peak=0.0f;
    for(b=0;b<blocks;b++){
        unsigned n=timed?128:sizes[b%9];
        if(timed&&!(b%64)){bass.amp=0.65f;bl_env=0.4f;bl_gate=4000.0f;bl_up=1;acid.amp=0.6f;acid.amp_t=1.0f;acid.env=0.95f;acid.gate_left=4000;}
        if(test<100)render_bass(n);else render_acid(n);frames+=n;
        if(!timed){
            const float *out=test<100?bass_o:acid_o;record(out,n*4);
            if(test<100){const float line[]={bl_env,bl_gate};const uint32_t up=bl_up;record(&bass,sizeof bass);record(line,8);record(&up,4);record(&dr_ctl,4);record((const void *)&en_bass_level,4);}
            else{record(&acid,sizeof acid);record(&ac_ctl,4);record((const void *)&en_acid_level,4);}
            for(j=0;j<n;j++){assert(isfinite(out[j]));peak=fm_maxf(peak,fm_fabsf(out[j]));}
        }
    }return frames;
}
int main(int argc,char **argv){
    unsigned test=argc>1?(unsigned)atoi(argv[1]):0,frames;
    _Static_assert(sizeof bass==96 && sizeof acid==108,"frozen tonal voice layouts");
    if(test==1000){unsigned t;printf("{\"cases\":[");
        for(t=0;t<=224;t++){if(t>=97&&t<100)continue;setup(t);printf("%s{\"test\":%u,\"state\":",t?",":"",t);words(t<100?(const void *)&bass:(const void *)&acid,t<100?sizeof bass/4:sizeof acid/4);
            if(t<100){const fm1_voice_drone_config_t c=voice_drone_config();const float line[]={bl_env,bl_gate};printf(",\"config\":");words(&c,sizeof c/4);printf(",\"line\":");words(line,2);printf(",\"up\":%u,\"control\":%u",(unsigned)bl_up,dr_ctl);}
            else{const fm1_voice_acid_config_t c=voice_acid_config();printf(",\"config\":");words(&c,sizeof c/4);printf(",\"control\":%u",ac_ctl);}
            run(t,256,0);printf(",\"hash\":%u,\"peak\":%.9g}",hash,(double)peak);
        }puts("]}");return 0;
    }
    setup(test);
    if(argc>2&&!strcmp(argv[2],"bench")){struct timespec a,b;unsigned blocks=argc>3?(unsigned)atoi(argv[3]):10000;clock_gettime(CLOCK_MONOTONIC,&a);run(test,blocks,1);clock_gettime(CLOCK_MONOTONIC,&b);printf("{\"test\":%u,\"blocks\":%u,\"frames\":%u,\"nanoseconds\":%.0f}\n",test,blocks,blocks*128,(double)(b.tv_sec-a.tv_sec)*1e9+(double)(b.tv_nsec-a.tv_nsec));return 0;}
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}frames=run(test,256,0);if(trace&&fclose(trace))return 2;
    assert(peak<64.0f);printf("{\"test\":%u,\"frames\":%u,\"hash\":%u,\"peak\":%.9g}\n",test,frames,hash,(double)peak);return 0;
}
