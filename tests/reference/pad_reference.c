/* SPDX-License-Identifier: GPL-3.0-only */
/* The real selected pad worker and bus; frozen R01 kernels are never edited. */
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
#include "../../firmware/src/dsp/rust_pad.inc"
#endif
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static FILE *trace;static uint32_t hash;static float peak;
static const unsigned sizes[]={0,1,31,32,33,127,128,129,256};
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
static void words(const void *p,size_t n){unsigned i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++){uint32_t v;memcpy(&v,b+4*i,4);printf("%s%u",i?",":"",v);}putchar(']');}
static void bytes(const void *p,size_t n){size_t i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",(unsigned)b[i]);putchar(']');}
static void setup(unsigned test){
    unsigned i,k,variant=test<300?test%10:(test-300)%3,sound=test<300?test/10:(test-300)/3;
    en_init();en_jam_on=variant==8;en_playing=variant!=9;apply(P_MIDI,0);apply(P_PADTONE,(int)sound);apply(P_PAD,100);apply(P_BRIGHT,variant%3==2?100:variant%3==1?0:50);apply(P_DETUNE,variant%3==2?100:35);
    apply(P_PAD_COMP,variant==1?2:1);apply(P_PAD_MAN,variant==2);apply(P_PE_ATK,variant%3==2?100:0);apply(P_PE_HOLD,0);apply(P_PE_REL,variant%3==2?100:0);apply(P_PE_REST,0);apply(P_PL_CHO,0);
    memset(pad,0,sizeof pad);memset(pq_st,0,sizeof pq_st);memset(pq_d1,0,sizeof pq_d1);memset(pq_d2,0,sizeof pq_d2);pq_n=0;
    pad_gain=variant==9?0.0f:0.7f;pad_gate=variant==9?0.0f:1.0f;stab_env=0.8f;stab_k=0.92f;prng=0x2545f491u^((test+1u)*2654435761u);
    motion=0.6f;arcm=0.3f;pad_filt=0.15f;pad_pan=-0.1f;pad_pitch=0.4f;pad_lvl=0.85f;pad_lk=0.7f;brk=0.3f;jp_swell_env=0.75f;
    for(i=0;i<6;i++)mod[i].v=(float)((int)i-3)*0.2f;
    for(i=0;i<4;i++){
        pad_t *v=&pad[i];v->note=48.0f+3.0f*(float)i;v->target=v->note+7.0f;v->t=variant==3?0.9999f:0.05f+0.08f*(float)i;v->rate=0.003f;v->peak=0.75f;v->amp=variant==9?0.0f:0.5f;v->fa=0.1f;v->fh=0.3f;v->fr=0.2f;
        v->vib=0.99f;v->pan=PAD_PAN[i];v->hf=variant==4?HAND_STEP*0.5f:variant==5?0.99999f:1.0f;v->hand=variant==4?1:variant==5?2:0;v->voices=(uint8_t)(variant==4?(pad_osc()+3)%10:pad_osc());v->midi_note=(uint8_t)v->note;
        v->play=(uint8_t)(variant==6?1:variant==7?2:0);v->pamp=variant==7?0.001f:0.3f;v->pnote=(uint8_t)v->note;v->pord=i+1;
        for(k=0;k<4;k++){v->ph[k]=0.08f+0.17f*(float)i+0.09f*(float)k;v->inc[k]=mtof(v->note)*(1.0f/EN_SR);v->ic[k]=(float)((int)k-2)*0.001f;}
        v->sub=0.2f;svf_coefs(v->c1,600.0f,0.45f);svf_coefs(v->c2,1200.0f,0.3f);
        pad_ctl[i]=(uint32_t[]){0,7,31,0xfffffff0u}[(variant+i)%4];en_pad_note[i]=(uint8_t)(80+i);en_pad_amp[i]=0.123f;
    }
    en_pad_level=0.5f;
    if(variant==4){pq_n=15;for(i=0;i<15;i++){pq_st[i]=0x90;pq_d1[i]=(uint8_t)(60+i);pq_d2[i]=80;}}
    fxs_v[EN_FXS_FT_PAD]=(int16_t)(test%5);trk_g[TR_PAD]=0.3f;trk_g1[TR_PAD]=0.9f;tg_inv=1.0f/256.0f;
    rv_job.pad_w=500;plp[0]=0.01f;plp[1]=-0.02f;plp[2]=0.03f;plp[3]=-0.04f;
    for(i=0;i<PAD_R;i++)pad_ring[2][i]=(float)((int)(i%17)-8)*0.0001f;
}
static void state(void){
    const float scalars[]={pad_gain,stab_env};record(pad,sizeof pad);record(scalars,8);record(pad_ctl,sizeof pad_ctl);record(pq_st,16);record(pq_d1,16);record(pq_d2,16);record(&pq_n,1);record(&prng,4);record((const void *)en_pad_note,4);record((const void *)en_pad_amp,16);record((const void *)&en_pad_level,4);
}
static uint32_t run(unsigned test,unsigned blocks,int timed){
    unsigned block,j;uint32_t frames=0;hash=2166136261u;peak=0.0f;
    for(block=0;block<blocks;block++){
        unsigned n=timed?128:sizes[block%9];
        if(test<300){render_pad(n);if(!timed){record(padl,n*4);record(padr,n*4);state();for(j=0;j<n;j++){assert(isfinite(padl[j])&&isfinite(padr[j]));peak=fm_maxf(peak,fm_maxf(fm_fabsf(padl[j]),fm_fabsf(padr[j])));}}if(n)pad_midi_flush();}
        else{
            for(j=0;j<n;j++){padl[j]=(float)((int)j-128)*0.0005f;padr[j]=(float)(128-(int)j)*0.0003f;}
            pad_bus(n);if(!timed){record(pad_ol,n*4);record(pad_or,n*4);record(plp,sizeof plp);record(pad_ring[2],sizeof pad_ring[2]);record(&rv_job.pad_w,4);for(j=0;j<n;j++){assert(isfinite(pad_ol[j])&&isfinite(pad_or[j]));peak=fm_maxf(peak,fm_maxf(fm_fabsf(pad_ol[j]),fm_fabsf(pad_or[j])));}}
        }
        frames+=n;rv_job.pad_w+=n;
    }return frames;
}
int main(int argc,char **argv){
    unsigned test=argc>1?(unsigned)atoi(argv[1]):0,frames;
    _Static_assert(sizeof(pad[0])==148 && PAD_R==512,"frozen pad ABI");
    if(test==1000){unsigned t;float exps[961];for(t=0;t<961;t++)exps[t]=fm_expf(-90.0f+(float)t*0.1875f);setup(300);printf("{\"exp_samples\":");words(exps,961);printf(",\"dry_initial\":");words(pad_ring[2],PAD_R);printf(",\"cases\":[");
        for(t=0;t<372;t++){if(t>=240&&t<300)continue;setup(t);printf("%s{\"test\":%u",t?",":"",t);
            if(t<300){const fm1_voice_pad_config_t c=voice_pad_config();const float metrics[]={pad_gain,stab_env};printf(",\"state\":");words(pad,sizeof pad/4);printf(",\"config\":");words(&c,sizeof c/4);printf(",\"metrics\":");words(metrics,2);printf(",\"controls\":");words(pad_ctl,4);printf(",\"midi\":[");bytes(pq_st,16);putchar(',');bytes(pq_d1,16);putchar(',');bytes(pq_d2,16);printf("],\"midi_count\":%u,\"seed\":%u,\"ui_notes\":",(unsigned)pq_n,prng);bytes((const void*)en_pad_note,4);printf(",\"ui_amps\":");words((const void*)en_pad_amp,4);}
            else{const fm1_voice_pad_bus_t c=voice_pad_bus_config();printf(",\"config\":");words(&c,sizeof c/4);printf(",\"filter\":");words(plp,4);}
            run(t,128,0);printf(",\"hash\":%u,\"peak\":%.9g}",hash,(double)peak);
        }puts("]}");return 0;
    }
    setup(test);
    if(argc>2&&!strcmp(argv[2],"bench")){struct timespec a,b;unsigned blocks=argc>3?(unsigned)atoi(argv[3]):10000;clock_gettime(CLOCK_MONOTONIC,&a);run(test,blocks,1);clock_gettime(CLOCK_MONOTONIC,&b);printf("{\"test\":%u,\"blocks\":%u,\"frames\":%u,\"nanoseconds\":%.0f}\n",test,blocks,blocks*128,(double)(b.tv_sec-a.tv_sec)*1e9+(double)(b.tv_nsec-a.tv_nsec));return 0;}
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}frames=run(test,128,0);if(trace&&fclose(trace))return 2;assert(peak<64.0f);printf("{\"test\":%u,\"frames\":%u,\"hash\":%u,\"peak\":%.9g}\n",test,frames,hash,(double)peak);return 0;
}
