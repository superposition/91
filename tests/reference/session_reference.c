/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile against the frozen R01 source, never the Rust codec. */
#define EN_WEB 1
#include <stdint.h>
static void web_audio(const int32_t *block, uint32_t frames) {(void)block;(void)frames;}
#include "host/amb_host.c"

static uint32_t byte(uint32_t h, unsigned v) {return (h ^ v) * 16777619u;}
int main(void)
{
    uint8_t b[22]={'J','S','M',1,1,0,0,16,0,3,60,0,0,0,100,0};
    unsigned i,j,accepted=0,tempo_accepted=0;
    uint32_t h=2166136261u,lengths=2166136261u,tempos=2166136261u;
    for(i=0;i<=sizeof b;i++) lengths=byte(lengths,(unsigned)jam_session_valid(b,i));
    for(i=0;i<20;i++) {
        uint8_t original=b[i];
        for(j=0;j<256;j++) {
            unsigned valid;b[i]=(uint8_t)j;valid=(unsigned)jam_session_valid(b,20);
            accepted+=valid;h=byte(h,valid);
        }
        b[i]=original;
    }
    for(i=0;i<=65535;i++) {
        unsigned valid;b[14]=(uint8_t)i;b[15]=(uint8_t)(i>>8);
        valid=(unsigned)jam_session_valid(b,20);tempo_accepted+=valid;tempos=byte(tempos,valid);
    }
    printf("{\"mutation_hash\":%u,\"mutation_accepted\":%u,\"length_hash\":%u,\"tempo_hash\":%u,\"tempo_accepted\":%u}\n",
        h,accepted,lengths,tempos,tempo_accepted);
    return 0;
}
