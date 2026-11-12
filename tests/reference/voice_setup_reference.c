/* SPDX-License-Identifier: GPL-3.0-only */
/* Exercise the selected real engine's slow controls and manual voice lifecycle. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "amb.c"
static uint32_t hash;static FILE *trace;static int active;
static const unsigned sizes[]={0,1,31,32,33,127,128,129,256};
static void record(const void *p,size_t n){size_t i;const uint8_t *b=p;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){const uint32_t m[]={s,a,b};if(active)record(m,sizeof m);}
static void setup(unsigned test){
    unsigned i;active=0;en_init();en_playing=1;en_jam_on=0;muted=0;midi_map=0;par[P_MIDI]=1;
    par[P_PAD_OCT]=-1;par[P_BASSOCT]=1;par[P_AC_OCT]=0;par[P_OCT]=0;par[P_AC_DEC]=100;par[P_BL_MODE]=0;par[P_BL_GLIDE]=0;par[P_BL_LEN]=7;par[P_PE_REL]=100;par[P_LK_HAT]=100;par[P_PADTONE]=9;
    tunefac=1.0f;step_len=200.0f;chord_pcs[0]=0;chord_pcs[1]=4;chord_pcs[2]=7;chord_nt=3;
    memset(pad,0,sizeof pad);memset(&bass,0,sizeof bass);memset(&acid,0,sizeof acid);memset(drum,0,sizeof drum);memset(held,0,sizeof held);memset(nheld,0,sizeof nheld);play_seq=0;en_held=0;
    for(i=0;i<6;i++){drum[i].on=1;drum[i].rn=3;drum[i].wait=17;drum[i].amp=0.4f;}
    for(i=0;i<4;i++){pad[i].target=48.0f+4.0f*(float)i;pad[i].amp=0.1f+0.1f*(float)i;pad[i].hf=1.0f;}
    bass.gtarget=36.0f;bass.amp=0.01f;bl_env=0.5f;bl_gate=0.0f;bl_up=0;bl_slide=0;
    memset(mod,0,sizeof mod);for(i=0;i<10;i++){mod[i].v=(float)i*0.1f;mod[i].left=i*16;mod[i].rate=(float)i*0.0001f;}
    pad_lfo=(lfo_t){0.999f,0.0f,-0.5f,0.75f};rng=0x173894afu;
    if(test>=85 && test<91){bass.env=0.2f;bass.env_up=(uint8_t)((test-85)%2);par[P_DE_ATK]=par[P_DE_DEC]=(int)((test-85)/2)*50;}
    hash=2166136261u;active=1;
}
static void state(void){const float line[]={bl_env,bl_gate};const uint8_t flags[]={bl_up,bl_slide};record(pad,sizeof pad);record(&bass,sizeof bass);record(&acid,sizeof acid);record(drum,sizeof drum);record(held,sizeof held);record(nheld,sizeof nheld);record(&play_seq,4);record((const void *)&en_held,1);record(line,8);record(flags,2);}
static void run(unsigned test){
    unsigned j,i;static const int keys[]={48,55,60,64,67,72,76,79,84,88,91,96};
    for(j=0;j<1024;j++){
        const unsigned n=sizes[j%9];
        if(test<84){lfo_step(&pad_lfo,(int)(test/7),(int)(test%7),(float)n,step_len);record(&pad_lfo,sizeof pad_lfo);record(&rng,4);}
        else if(test==84){mods_step(n);record(mod,sizeof mod);record(&rng,4);}
        else if(test<91){
            /* en_render never calls the full chunk-control chain for zero. */
            if(n)control(n);
            record(&bass.env,4);record(&bass.env_up,1);
        } else {
            const int voice=(int)test-100;
            if(j%128==127)quiet();
            else if(j%32==31){for(i=0;i<12;i++)note_cmd(voice,keys[i],0);}
            else if(j%32==15 && voice==0)pad_handoff();
            else if(j%32==20 && voice==0)pad_leave();
            else if(j%32==25 && voice==0)stab_fire(0.75f);
            else if(j%32==15 && voice==1)bl_note(31,(int)(j%4));
            else if(j%32==20 && voice==1)bass_to(7);
            else if(j%32==15 && voice==2){ac_pend.n=60;ac_pend.slid_ok=(uint8_t)(j%2);ac_pend.acc=(uint8_t)(j%3==0);ac_pend.hat=1;ac_pend.slide=(uint8_t)(j%4==0);ac_pend.beat=0;ac_pend.dec=0.123f;acid_fire();}
            else note_cmd(voice,keys[j%12],j%5==4?0:(int)(j%3==0?127:j%3==1?100:30));
            state();
        }
    }
}
int main(int argc,char **argv){unsigned test=argc>1?(unsigned)atoi(argv[1]):0,t;if(test==1000){printf("{\"times\":[");for(t=0;t<606;t++){const unsigned kind=t/101,value=t%101;float f=kind==0?pe_atk_s(value):kind==1?pe_hold_s(value):kind==2?pe_rel_s(value):kind==3?pe_rest_s(value):kind==4?de_atk_s(value):de_dec_s(value);uint32_t bits;memcpy(&bits,&f,4);printf("%s%u",t?",":"",bits);}printf("],\"cases\":[");for(t=0;t<=102;t++){if(t>=91&&t<100)continue;setup(t);run(t);printf("%s{\"test\":%u,\"hash\":%u}",t?",":"",t,hash);}puts("]}");return 0;}
    setup(test);if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}run(test);if(trace&&fclose(trace))return 2;printf("{\"test\":%u,\"frames\":1024,\"hash\":%u}\n",test,hash);return 0;}
