/* SPDX-License-Identifier: GPL-3.0-only */
/* Exact emu_rust_test factory inputs over the native32-bit app/engine ABI. */
#include <time.h>
static int diagnostic_time(clockid_t c,struct timespec *t){(void)c;t->tv_sec=0;t->tv_nsec=0;return 0;}
#define clock_gettime diagnostic_time
#include "../../web/emu/amb_web.c"
#undef clock_gettime
extern void compiler_effects_state(FILE *,FILE *);
int main(int argc,char **argv){
    unsigned block,k,limit=argc>2?(unsigned)atoi(argv[2]):55000;uint32_t hash=2166136261u;FILE *f=argc>1?fopen(argv[1],"wb"):0;
    if(argc>1&&!f)return 2;
    FILE *state=argc>3?fopen(argv[3],"wb"):0,*names=argc>4?fopen(argv[4],"w"):0;
    web_seed(91);web_master(2800);web_boot();if(state)compiler_effects_state(state,names);if(names)fclose(names);
    for(block=0;block<limit;block++){
        switch(block){
        case 1000:web_keys(1);break;case 1050:web_keys(0);break;
        case 3000:web_enc(0,1);break;case 3060:web_enc(4,1);break;
        case 8000:case 16000:web_enc(2,1);break;
        case 24000:web_enc(0,1);break;case 24500:web_enc(4,1);break;
        case 32000:web_enc(0,-1);break;
        case 35000:case 35100:web_buttons(1u<<10);break;case 35005:case 35105:web_buttons(0);break;
        case 40000:web_enc(0,1);break;case 40100:web_enc(4,-1);break;
        }
        web_render(128);
        if(state)compiler_effects_state(state,0);
        for(k=0;k<256;k++){uint32_t bits;float value=k<128?out_l[k]:out_r[k-128];memcpy(&bits,&value,4);hash=(hash^bits)*16777619u;if(f&&fwrite(&bits,4,1,f)!=1)return 3;}
    }
    if(f)fclose(f);if(state)fclose(state);
    printf("{\"blocks\":%u,\"pcm_hash\":\"%08x\",\"blits\":%u,\"midi_messages\":%u,\"clock_messages\":%u}\n",limit,hash,web_blits(),midi_out_count,clock_out_count);
    return 0;
}
