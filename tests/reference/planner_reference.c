/* SPDX-License-Identifier: GPL-3.0-only */
/* Historical oracle adapter. Compile against the immutable R01 source snapshot. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static uint32_t hash_byte(uint32_t h,unsigned b){return (h^(b&255u))*16777619u;}
static uint32_t hash_u32(uint32_t h,uint32_t v){unsigned i;for(i=0;i<4;i++)h=hash_byte(h,v>>(i*8));return h;}
int main(int argc,char **argv){
    unsigned arg,key,minor,bars,form,move,section,i,j,bar,step,n,velocity,gate,valid,first=1;
    uint8_t bytes[JP_MAX+1],bad[JP_MAX+1];
    printf("{\"provenance\":\"R01 source.zip SHA-256 3d20587f825b8a26cd1f9ab6a715e65e78838fce0c19b5c27ce0427c15fa6263\",\"cases\":[");
    for(arg=1;arg<(unsigned)argc;arg++){
        FILE *f=fopen(argv[arg],"rb");const char *name=strrchr(argv[arg],'/');uint32_t validation=2166136261u;
        if(!f)return 2;memset(bytes,0,sizeof bytes);n=(unsigned)fread(bytes,1,JP_MAX,f);fclose(f);
        if(!en_jam_valid(bytes,n))return 3;name=name?name+1:argv[arg];
        for(key=0;key<12;key++)for(minor=0;minor<2;minor++)for(bars=8;bars<=16;bars+=8){
            uint32_t h=2166136261u;
            for(form=0;form<4;form++)for(move=0;move<4;move++){
                jp_settings_t settings={1,key,minor,bars,form,move,60,0,3,0};
                jp_phrase_t a={0},b={0};uint32_t seed=4u+form*257u+move*4099u;
                jp_plan(bytes,&settings,0,0,&a,&seed);
                for(section=0;section<8;section++){
                    jp_plan(bytes,&settings,section+1,&a,&b,&seed);
                    h=hash_byte(h,a.bars);h=hash_byte(h,a.role);h=hash_byte(h,a.motif);h=hash_byte(h,a.section);
                    for(i=0;i<bars*4;i++){
                        for(j=0;j<4;j++)h=hash_byte(h,a.beat[i].note[j]);
                        h=hash_byte(h,a.beat[i].node);h=hash_byte(h,a.beat[i].passing);
                    }
                    for(bar=0;bar<bars;bar++)for(step=0;step<16;step++){
                        int note=jp_bass_note(bytes,&settings,&a,&b,bar,step,&velocity,&gate);
                        h=hash_byte(h,(unsigned)note);h=hash_byte(h,(unsigned)note>>8);
                        h=hash_byte(h,velocity);h=hash_byte(h,gate);
                    }
                    h=hash_u32(h,seed);a=b;
                }
            }
            if(!first)putchar(',');first=0;
            printf("{\"pack\":\"%s\",\"key\":%u,\"minor\":%u,\"bars\":%u,\"hash\":%u}",name,key,minor,bars,h);
        }
        (void)validation;
    }
    printf("],\"validation\":[");
    for(arg=1;arg<(unsigned)argc;arg++){
        FILE *f=fopen(argv[arg],"rb");const char *name=strrchr(argv[arg],'/');uint32_t h=2166136261u;unsigned total=0;
        memset(bytes,0,sizeof bytes);n=(unsigned)fread(bytes,1,JP_MAX,f);fclose(f);name=name?name+1:argv[arg];
        for(i=0;i<=JP_MAX+1;i++){valid=en_jam_valid(bytes,i);h=hash_byte(h,valid);total+=valid;}
        for(i=0;i<n;i++)for(j=0;j<3;j++){
            static const uint8_t change[]={1,128,255};memcpy(bad,bytes,sizeof bytes);bad[i]^=change[j];
            valid=en_jam_valid(bad,n);h=hash_byte(h,valid);total+=valid;
        }
        if(arg!=1)putchar(',');printf("{\"pack\":\"%s\",\"hash\":%u,\"accepted\":%u}",name,h,total);
    }
    printf("]}\n");return 0;
}
