/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include "fm1_rust_hal.h"
#include "readable.inc"
static void emit(uint32_t address,uint32_t length){
    const uint32_t result=(uint32_t)fm1_mem_readable(address,length);
    fwrite(&result,sizeof result,1,stdout);
}
int main(void){
    const uint32_t edges[]={0,0x01c00000u,0x01c80000u,0x02000000u,0x02100000u,0xffffffffu};
    const uint32_t lengths[]={0,1,2,7,255,256,512,65535,0x80000u,0x100000u,0xffffffffu};
    for(unsigned i=0;i<6;i++)for(int step=-8;step<=8;step++)for(unsigned j=0;j<11;j++)
        emit(edges[i]+(uint32_t)step,lengths[j]);
    uint32_t rng=0x91f0a137u;
    for(unsigned i=0;i<32768;i++){
        rng^=rng<<13;rng^=rng>>17;rng^=rng<<5;
        emit(rng,i&1?lengths[i%11]:rng*16777619u);
    }
    return 0;
}
