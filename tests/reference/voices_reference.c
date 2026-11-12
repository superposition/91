/* SPDX-License-Identifier: GPL-3.0-only */
/* The selected instrument's actual voice kernels, including immutable R01. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>
#include <assert.h>
#include "amb.c"
#include "../../firmware/src/dsp/rust_voices.h"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static FILE *trace;
static uint32_t hash;
static float peak;
static const unsigned sizes[]={0,1,31,32,33,127,128,129,256};
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
static void words(const void *p,size_t n){unsigned i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++){uint32_t v;memcpy(&v,b+4*i,4);printf("%s%u",i?",":"",v);}putchar(']');}
static void setup(unsigned test){
    unsigned d;en_init();en_jam_on=0;en_playing=1;apply(P_MIDI,0);noise=0x9e3779b9u*(test+1u);
    memset(drum,0,sizeof drum);memset(cm,0,sizeof cm);cm_alt=0;
    apply(P_PULSE,100);apply(P_CM_LVL,100);apply(P_DRIVE,(int)(test*17u%101));
    for(d=0;d<EN_NDRUM;d++){lane_g[d]=0.4f+0.1f*(float)d;lane_tune[d]=(float)((int)(test%3)-1)*12.0f;lane_dec[d]=1.0f;}
    for(d=0;d<EN_NTRACK;d++){trk_g[d]=0.15f+0.07f*(float)d;trk_g1[d]=1.0f-0.05f*(float)d;}tg_inv=1.0f/256.0f;
    if(test<98){
        const unsigned kit=test/7,lane=test%7;
        apply(P_KIT_KICK,(int)kit);apply(P_KIT_SNARE,(int)kit);apply(P_KIT_HAT,(int)kit);apply(P_KIT_PERC,(int)kit);
        for(d=0;d<EN_NDRUM;d++)if(lane==6||lane==d)drum_fire((int)d,0.85f);
    }else{
        const unsigned sound=test==136?0:test==137?CM_NPLAIN:(test-100)/3,variant=(test-100)%3;
        apply(P_CM_SND,(int)sound);apply(P_CM_DEC,variant==0?0:variant==1?45:100);
        cm_trigger(36+(int)variant*24,0.95f);cm_trigger(43+(int)variant*24,0.65f);
        if(variant==2)cm[0].age=0xfffffff0u;
        if(test==136)cm_vol=0.0f;
        if(test==137){cm[0].env=1.0f;cm[1].env=0.7f;}
        en_cm_level=0.75f;
    }
}
static fm1_voice_drums_config_t drum_config(void){
    const float g=pulse_vol*1.1f;
    const fm1_voice_drums_config_t c={{g*lane_g[L_KICK]*1.15f,g*0.38f*lane_g[L_SNARE],g*0.42f*lane_g[L_CLAP],g*0.22f*lane_g[L_HAT],g*0.18f*lane_g[L_OHAT],g*0.3f*lane_g[L_PERC]},
        {hat_inc[0],hat_inc[1],hat_inc[2],hat_inc[3]},drive_k*KK->k_drive,KK->k_drop,KK->k_click,KK->k_rumble,KS->sn_body,KS->sn_noise,KS->cl_gap,KH->h_noise,
        {trk_g[TR_KICK],trk_g[TR_SNARE],trk_g[TR_HATS],trk_g[TR_PERC]},{trk_g1[TR_KICK],trk_g1[TR_SNARE],trk_g1[TR_HATS],trk_g1[TR_PERC]},tg_inv};return c;
}
static fm1_voice_pluck_config_t pluck_config(void){
    const int sn=par[P_CM_SND]<0?0:par[P_CM_SND]>=CM_NPLAIN?CM_NPLAIN-1:par[P_CM_SND];const cm_snd_t *s=&CM_SOUND[sn];
    const fm1_voice_pluck_config_t c={cm_vol*0.5f*s->g,HB(EN_V_PLUCK)?hb_pf:1.0f,s->fr,s->i0,s->i1,trk_g[TR_CM],trk_g1[TR_CM],tg_inv,en_cm_level,s->rise,(uint32_t)(par[P_CM_SND]>=CM_NPLAIN)};return c;
}
static uint32_t run(unsigned test,unsigned blocks,int timed){
    unsigned block,j;uint32_t frames=0;hash=2166136261u;peak=0.0f;
    for(block=0;block<blocks;block++){
        const unsigned n=timed?128:sizes[block%9];
        memset(mixl,0,sizeof mixl);memset(mixr,0,sizeof mixr);memset(rsend,0,sizeof rsend);memset(esend,0,sizeof esend);
        if(timed && !(block%64)){
            if(test<98){unsigned d;for(d=0;d<6;d++)if(test%7==6||test%7==d)drum_fire((int)d,0.85f);}
            else cm_trigger(60,0.8f);
        }
        if(test<98)render_drums(n);else render_cm(n);
        frames+=n;
        if(!timed){
            record(mixl,n*4);record(mixr,n*4);record(rsend,n*4);record(esend,n*4);
            if(test<98){record(drum,sizeof drum);record(&noise,sizeof noise);}else{record(cm,sizeof cm);record((const void *)&en_cm_level,4);}
            for(j=0;j<n;j++){assert(isfinite(mixl[j])&&isfinite(mixr[j])&&isfinite(rsend[j])&&isfinite(esend[j]));peak=fm_maxf(peak,fm_maxf(fm_fabsf(mixl[j]),fm_fabsf(mixr[j])));}
        }
    }return frames;
}
int main(int argc,char **argv){
    unsigned test=argc>1?(unsigned)atoi(argv[1]):0,frames;
    _Static_assert(sizeof(drum[0])==104 && sizeof(cm[0])==28,"frozen voice layouts");
    if(test==1000){unsigned t;printf("{\"sine\":");words(SINT,1025);printf(",\"cases\":[");
        for(t=0;t<=137;t++){if(t==98||t==99)continue;setup(t);printf("%s{\"test\":%u,\"state\":",t?",":"",t);words(t<98?(const void *)drum:(const void *)cm,t<98?sizeof drum/4:sizeof cm/4);
            if(t<98){const fm1_voice_drums_config_t c=drum_config();printf(",\"config\":");words(&c,sizeof c/4);printf(",\"seed\":%u",noise);}else{const fm1_voice_pluck_config_t c=pluck_config();printf(",\"config\":");words(&c,sizeof c/4);}
            run(t,256,0);printf(",\"hash\":%u,\"peak\":%.9g}",hash,(double)peak);
        }puts("]}");return 0;
    }
    setup(test);
    if(argc>2 && !strcmp(argv[2],"bench")){
        struct timespec start,end;unsigned blocks=argc>3?(unsigned)atoi(argv[3]):20000;
        clock_gettime(CLOCK_MONOTONIC,&start);run(test,blocks,1);clock_gettime(CLOCK_MONOTONIC,&end);
        printf("{\"test\":%u,\"blocks\":%u,\"frames\":%u,\"nanoseconds\":%.0f}\n",test,blocks,blocks*128,(double)(end.tv_sec-start.tv_sec)*1e9+(double)(end.tv_nsec-start.tv_nsec));return 0;
    }
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}
    frames=run(test,256,0);if(trace&&fclose(trace))return 2;
    assert(peak<8.0f);
    printf("{\"test\":%u,\"frames\":%u,\"hash\":%u,\"peak\":%.9g}\n",test,frames,hash,(double)peak);return 0;
}
