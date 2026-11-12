/* SPDX-License-Identifier: GPL-3.0-only */
/* Host cost only: real job/span adapters, gates and periodic retrigger. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include "amb.c"
void en_midi_out(uint32_t a,uint32_t b,uint32_t c){(void)a;(void)b;(void)c;}
static uint64_t ns(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return(uint64_t)t.tv_sec*1000000000u+(uint64_t)t.tv_nsec;}
int main(int argc,char **argv){unsigned mode=argc>1?(unsigned)atoi(argv[1]):0,steps=argc>2?(unsigned)atoi(argv[2]):5000,i,j,n;uint32_t h=2166136261u,cursor=0;uint64_t start=ns(),init;int32_t audio[512];
    fm_init();init=ns()-start;en_init();
    if(mode==4){const uint8_t *data=en_jam_builtin(0,&n);if(!en_jam_upload(data,n))return 2;en_jam_set(JAM_ON,1);en_render(audio,256,2048);fm_jam_load(jp_data);}else{par[P_CM_SND]=CM_NPLAIN;fm_set_patch(2);}
    fm_engine=mode<3?mode:FM6_MARK1;fm6_float_path=mode==3;fm_ed[FP_ALG]=31;fm_ed[FP_FB]=7;cm_vol=.7f;pad_vol=.6f;bass_vol=.7f;trk_g[TR_CM]=trk_g1[TR_CM]=trk_g[TR_PAD]=trk_g1[TR_PAD]=1;tg_inv=1.0f/256;
    start=ns();for(i=0;i<steps;i++){rv_job_t job;memset(&job,0,sizeof job);job.n=256;job.fm_on=1;job.fm_w=cursor;rv_job.fm_w=cursor;
        if(i%13==0){if(mode==4){for(j=0;j<5;j++)fm_jam_note(0,48+(int)j*4,.8f,3000);fm_jam_note(1,32+(int)(i%12),.7f,3000);}else for(j=0;j<6;j++)fm_note_on(48+(int)j*4,.8f,3000);}
        fm_gates(256,2205);fm_job(&job);fm_ctr=cursor+FM_D;render_fm(256);if(mode==4)render_bass(256);
        for(j=0;j<256;j++){uint32_t bits;memcpy(&bits,&fm_ol[j],4);h=(h^bits)*16777619u;}cursor+=256;
    }
    printf("{\"mode\":%u,\"chunks\":%u,\"frames\":%u,\"hash\":\"%08x\",\"init_nanoseconds\":%llu,\"nanoseconds\":%llu}\n",mode,steps,steps*256,h,(unsigned long long)init,(unsigned long long)(ns()-start));return 0;
}
