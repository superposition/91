/* SPDX-License-Identifier: GPL-3.0-only */
/* Include the actual selected/frozen app; no copied audio arithmetic oracle. */
#ifndef __wasm__
#include <stdio.h>
#endif
#ifndef WEB_AUDIO_SOURCE
#define WEB_AUDIO_SOURCE "../../web/emu/amb_web.c"
#endif
#include WEB_AUDIO_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("write"))) extern void capture(const void *,uint32_t);
#define fwrite(pointer,size,count,stream) capture(pointer,(uint32_t)((size)*(count)))
#endif

static uint32_t available(void) {
#if EN_RUST_WEB
    return fm1_rust_web_available();
#else
    return ring_w-ring_r;
#endif
}
int main(void) {
    static int32_t input[8192];
    uint32_t rng=0x974b12u,k,step,n;
    for(k=0;k<8192;k++) {
        rng=rng*1664525u+1013904223u;
        memcpy(&input[k],&rng,4);
    }
    input[0]=INT32_MIN;input[1]=INT32_MAX;input[2]=0;input[3]=-1;
    for(step=0;step<4096;step++) {
        uint32_t count=(step*73u)%257u;
        if(step%101u==0)count=4096;
        web_audio(input,count);
        n=(step*127u)%1027u;
        while(available()<(n>1024?1024:n))web_audio(input,256);
        web_render(n);
        fwrite(web_out_l(),4,1024,stdout);
        fwrite(web_out_r(),4,1024,stdout);
        k=available();fwrite(&k,4,1,stdout);
    }
    for(k=0;k<65536;k++) {
        theme_t theme=THEMES[0];
        theme.pad=(uint16_t)k;theme.acc=(uint16_t)(k^0xaaaa);
        theme.bg=(uint16_t)~k;theme.acid=(uint16_t)(k^0x5555);
        th=&theme;
        fwrite(web_bg(),4,12,stdout);
    }
#ifdef __wasm__
    return 0;
#else
    return ferror(stdout)?1:0;
#endif
}
