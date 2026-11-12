/* SPDX-License-Identifier: GPL-3.0-only */
/* Real selected DJ constructor and render loop; frozen R01 remains unedited. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <assert.h>
#include <time.h>
#include "amb.c"
#include "../../firmware/src/dsp/rust_voices.h"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static uint32_t hash;static FILE *trace;static float peak;
static const unsigned sizes[]={0,1,31,32,33,127,128,129,256};
static void record(const void *p,size_t n){size_t i;const uint8_t *b=p;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
static void words(const void *p,size_t n){size_t i;const uint8_t *b=p;putchar('[');for(i=0;i<n;i++){uint32_t w;memcpy(&w,b+4*i,4);printf("%s%u",i?",":"",w);}putchar(']');}
static void initialize(unsigned test){unsigned i;en_init();apply(P_MIDI,0);en_playing=1;en_jam_on=0;memset(djv,0x5a,sizeof djv);
    for(i=0;i<4;i++){dj_t *s=&djv[i];s->on=s->held=s->hold_mode=0;s->key=(uint8_t)(i+1);s->type=DJ_BEEP;s->hz=600.0f;s->aux=0.12f;s->t=0.0f;s->ph=0.1f;s->ph2=0.2f;s->ph3=0.3f;s->env=0.25f;s->z=0.01f;s->z2=-0.01f;s->age=5+i;}
    dj_age=test==210?0xfffffffdu:0;en_smp_active=0;rng=0x9e3779b9u^((test+1u)*2654435761u);
    fxs_v[EN_FXS_SM_MODE]=(int16_t)(test<162?test%3:0);fxs_v[EN_FXS_SM_LOOP]=(int16_t)(test<162?test/3%2:test==203||test==207||test==210);fxs_v[EN_FXS_SM_LEN]=(int16_t)(test%3);fxs_v[EN_FXS_SM_MIX]=100;
    trk_g[TR_ACID]=0.3f;trk_g1[TR_ACID]=0.9f;tg_inv=1.0f/256.0f;
    if(test<162)smp_note((int)(test/6),1);
    else if(test==200)smp_note(25,1);else if(test==201)smp_note(20,1);else if(test==202||test==203||test==207)smp_note(0,1);
    else if(test==204){smp_note(24,1);djv[0].type=DJ_STAB;}
    else if(test==205){smp_note(26,1);djv[0].type=255;}
    else if(test==208)smp_note(14,1);else if(test==209)smp_note(3,1);
    else if(test==210){smp_note(0,1);smp_note(4,1);smp_note(17,1);smp_note(21,1);smp_note(26,1);}
    if(test==207)fxs_v[EN_FXS_SM_MIX]=0;
    if(test==208){fxs_v[EN_FXS_SM_MIX]=-1;fxs_v[EN_FXS_SM_LEN]=-1;}
    if(test==209){fxs_v[EN_FXS_SM_MIX]=101;fxs_v[EN_FXS_SM_LEN]=99;}
}
static unsigned blocks(unsigned test){return test<162?512:4096;}
static void events(unsigned test,unsigned block){
    if(test<162 && block==128)smp_note((int)(test/6),0);
    if(test==203 && block==2048)smp_note(0,0);
    if(test==207 && block==128)smp_note(0,0);
    if(test==210){unsigned k;if(block==32)smp_note(0,0);if(block==64)smp_note(3,1);if(block==128)for(k=0;k<27;k++)smp_note((int)k,0);if(block==256)smp_note(14,1);}
}
static uint32_t run(unsigned test,unsigned count,int timed){unsigned block,j;uint32_t frames=0;hash=2166136261u;peak=0.0f;
    for(block=0;block<count;block++){const unsigned n=timed?128:sizes[block%9];events(test,block);
        if(timed && !(block%64)){memset(djv,0,sizeof djv);smp_note(test<162?(int)(test/6):test==200?25:test==201?20:0,1);}
        memset(mixl,0,sizeof mixl);memset(mixr,0,sizeof mixr);memset(rsend,0,sizeof rsend);memset(esend,0,sizeof esend);
        if(!timed && test!=206 && test!=207 && test!=208)for(j=0;j<n;j++){mixl[j]=(float)(j%7)*0.0001f;mixr[j]=-(float)(j%9)*0.0002f;rsend[j]=(float)(j%5)*0.0003f;esend[j]=-(float)(j%3)*0.0004f;}
        render_dj(n);frames+=n;
        if(!timed){record(mixl,n*4);record(mixr,n*4);record(rsend,n*4);record(esend,n*4);record(djv,sizeof djv);record(&rng,4);record(&dj_age,4);record((const void*)&en_smp_active,1);
            for(j=0;j<n;j++){assert(isfinite(mixl[j])&&isfinite(mixr[j])&&isfinite(rsend[j])&&isfinite(esend[j]));peak=fm_maxf(peak,fm_maxf(fm_fabsf(mixl[j]),fm_fabsf(mixr[j])));}}
    }return frames;
}
static void allocation(void){unsigned step;initialize(210);hash=2166136261u;printf("\"allocation\":[");for(step=0;step<40;step++){const int key=step%10==0?-1:step%10==1?27:step%10==2?99:(int)(step*7%27),on=step%4!=3;smp_note(key,on);record(djv,sizeof djv);record(&dj_age,4);printf("%s%u",step?",":"",hash);}putchar(']');}
static void lifecycle(void){dj_t saved[4];uint32_t age;initialize(210);memcpy(saved,djv,sizeof saved);age=dj_age;play_stop();assert(!memcmp(saved,djv,sizeof saved)&&dj_age==age);quiet();assert(!memcmp(saved,djv,sizeof saved)&&dj_age==age);en_init();assert(!memcmp(saved,djv,sizeof saved)&&dj_age==age);printf("\"stop_panic_init_preserve_dj\":true");}
int main(int argc,char **argv){unsigned test=argc>1?(unsigned)atoi(argv[1]):0,frames;
    _Static_assert(sizeof(dj_t)==48,"frozen DJ ABI");
    if(test==1000){unsigned i;printf("{\"names\":[");for(i=0;i<27;i++)printf("%s\"%s\"",i?",":"",en_dj_name((int)i));printf("],\"sine\":");words(SINT,1025);printf(",");allocation();printf(",");lifecycle();printf(",\"cases\":[");
        for(i=0;i<=210;i++){fm1_voice_dj_config_t c;if(i>=162&&i<200)continue;initialize(i);c=(fm1_voice_dj_config_t){fxs_v[EN_FXS_SM_MIX],fxs_v[EN_FXS_SM_LEN],trk_g[TR_ACID],trk_g1[TR_ACID],tg_inv};printf("%s{\"test\":%u,\"state\":",i?",":"",i);words(djv,sizeof djv/4);printf(",\"config\":");words(&c,sizeof c/4);printf(",\"seed\":%u,\"age\":%u",rng,dj_age);frames=run(i,blocks(i),0);printf(",\"blocks\":%u,\"frames\":%u,\"hash\":%u,\"peak\":%.9g}",blocks(i),frames,hash,(double)peak);}puts("]}");return 0;}
    initialize(test);if(argc>2&&!strcmp(argv[2],"bench")){struct timespec a,b;unsigned count=argc>3?(unsigned)atoi(argv[3]):10000;clock_gettime(CLOCK_MONOTONIC,&a);run(test,count,1);clock_gettime(CLOCK_MONOTONIC,&b);printf("{\"test\":%u,\"blocks\":%u,\"frames\":%u,\"nanoseconds\":%.0f}\n",test,count,count*128,(double)(b.tv_sec-a.tv_sec)*1e9+(double)(b.tv_nsec-a.tv_nsec));return 0;}
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}frames=run(test,blocks(test),0);if(trace&&fclose(trace))return 2;assert(peak<64.0f);printf("{\"test\":%u,\"frames\":%u,\"hash\":%u,\"peak\":%.9g}\n",test,frames,hash,(double)peak);return 0;
}
