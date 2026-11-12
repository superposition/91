/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile only against the frozen R01 C source. */
#include <stdio.h>
#include "amb.c"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static uint32_t hash(uint32_t h,uint32_t v) {
    unsigned i;for(i=0;i<4;i++)h=(h^((v>>(i*8))&255u))*16777619u;return h;
}
int main(void) {
    const unsigned kinds[]={C_SET,C_POOL,C_PLAY,C_NEXT,C_FREEZE,C_SEED,C_PANIC,C_CLOCK,C_REGROW,C_SECTION,C_FX,C_MUTE,C_NOTE,C_HUSH,C_FXK,C_TRK,C_GATEPAT,C_FXS,C_FILLQ,C_DJSYNC,C_FLXH,C_SHED,C_USERVOICE,C_USERCLEAR,C_ACSTEP,C_ACCAP,C_CMCAP,C_INTRO,C_CMSTEP,C_PROG,C_PROG2,C_PROGK,C_MAP,C_DTAP,C_DEG,C_BEND,C_PEDAL,C_BSSTEP,C_JAM};
    const uint32_t origins[]={0,1,0xffffffffu-1023,0xffffffffu-7};
    unsigned c,i,round,accepted,dropped,popped;uint32_t h,seed,old;
    printf("{\"kinds\":[");for(i=0;i<sizeof(kinds)/sizeof(*kinds);i++)printf("%s%u",i?",":"",kinds[i]);
    printf("],\"cases\":[");
    for(c=0;c<4;c++) {
        q_r=q_w=origins[c];h=2166136261u;seed=91;accepted=dropped=popped=0;
        for(round=0;round<256;round++) {
            for(i=0;i<(round%7==0?1536u:176u);i++) {
                seed=seed*1664525u+1013904223u;old=q_w;
                post(seed>>24,(int)(seed>>8),(int)seed);
                if(q_w!=old)accepted++;else dropped++;
                h=hash(h,q_w!=old);
            }
            for(i=0;i<(round%5==0?1400u:128u);i++) {
                unsigned present=q_r!=q_w;h=hash(h,present);
                if(present){h=hash(h,q[q_r%QN]);q_r++;popped++;}
            }
        }
        while(q_r!=q_w){h=hash(h,q[q_r%QN]);q_r++;popped++;}
        printf("%s{\"origin\":%u,\"hash\":%u,\"accepted\":%u,\"dropped\":%u,\"popped\":%u}",c?",":"",origins[c],h,accepted,dropped,popped);
    }
    puts("]}");return 0;
}
