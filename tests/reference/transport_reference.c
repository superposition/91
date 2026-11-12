/* SPDX-License-Identifier: GPL-3.0-only */
/* Include the selected, unedited instrument source. This adapter records the
 * real en_render/command/MIDI path, not a second implementation of its timing.
 * Compile with the frozen R01 include directory for independent evidence, or
 * the current C0/Rust paths for an immediately reproducible differential run. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "amb.c"

static uint32_t midi_hash=2166136261u, midi_count, clock_count, render_frame;
static FILE *trace;
static void trace_words(const void *data,size_t count)
{
    if(trace && fwrite(data,4,count,trace)!=count){perror("transport trace write");exit(2);}
}
static uint32_t word(uint32_t h,uint32_t v)
{
    unsigned i;for(i=0;i<4;i++)h=(h^((v>>(8*i))&255u))*16777619u;return h;
}
static uint32_t bits(float x) {uint32_t v;memcpy(&v,&x,4);return v;}
void en_midi_out(uint32_t s,uint32_t a,uint32_t b)
{
    const uint32_t event[]={0x4du,render_frame,s,a,b};trace_words(event,5);
    midi_hash=word(word(word(word(midi_hash,render_frame),s),a),b);
    midi_count++;if(s==0xf8)clock_count++;
}
int main(int argc,char **argv)
{
    const unsigned frames[]={1,127,128,129,255,256,257,511,1024};
    unsigned scenario=argc>1?(unsigned)atoi(argv[1]):0, i,n,tempo;
    uint32_t state_hash=2166136261u,pcm_hash=2166136261u;
    uint32_t state[11];
    int32_t out[2048];
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace){perror("transport trace open");return 2;}}
    en_init();
    if(scenario==8) {
        printf("[");for(tempo=70;tempo<=190;tempo++) {
            apply(P_TEMPO,(int)tempo);
            printf("%s%u",tempo==70?"":",",bits(step_len));
        }puts("]");return 0;
    }
    if(scenario&2u)en_jam_set(JAM_ON,1);
    else {
        unsigned l,s;en_genre(7);en_set(P_SWING,100);en_set(P_HUMAN,100);
        for(l=0;l<EN_NLANE;l++) {
            en_set(P_LLEN+(int)l,7+(int)l);
            for(s=0;s<16;s++)en_mod[l][s]=(uint8_t)(1+s%7 | (s%8)<<3 | (s%4)<<6);
        }
    }
    en_set(P_TEMPO,70);en_play(1);
    for(i=0;i<5000;i++) {
        if(i==100)en_set(P_TEMPO,190);
        if(i==700)en_set(P_TEMPO,113);
        if(i==950)en_play(0);
        if(i==975)en_play(1);
        if(i==1400)en_clock(EN_CLK_START);
        if(i>=1401 && i<2000 && !(i%3))en_clock(EN_CLK_TICK);
        if(i==1500)en_clock(EN_CLK_STOP);
        if(i==1537)en_clock(EN_CLK_CONTINUE);
        if(i==1999)en_clock(EN_CLK_STOP);
        if(i==2020)en_clock(EN_CLK_CONTINUE);
        if(i==2500)en_play(0);
        if(i==2601)en_clock(EN_CLK_CONTINUE);
        if(i==3000)en_jam_set(JAM_HOLD,1);
        if(i==3200)en_jam_set(JAM_KEY,5);
        if(i==3300)en_jam_set(JAM_FORM,2);
        if(i==3500)en_jam_set(JAM_HOLD,0);
        if(i==3600)en_jam_set(JAM_IDEA,1);
        if(i==3900)en_set(P_TEMPO,70);
        if(i==4200)en_set(P_TEMPO,190);
        if(i==4400)en_clock(EN_CLK_START);
        if(i>=4401 && i<4700)en_clock(EN_CLK_TICK);
        if((scenario&4u) && i>=2100 && i<2400 && !(i%4))en_clock(EN_CLK_TICK);
        n=(scenario&1u)?frames[i%9]:256;
        /* A zero-length call drains commands and ages the external clock, but
         * must not advance samples, chunks or control cadence. */
        if(i%97==0)n=0;
        en_render(out,n,2048);
        {const uint32_t audio[]={0x41u,n};trace_words(audio,2);trace_words(out,2*n);}
        for(tempo=0;tempo<2*n;tempo++)pcm_hash=word(pcm_hash,(uint32_t)out[tempo]);
        render_frame+=n;
#if EN_RUST_TRANSPORT
        state[0]=bits(transport.pulse_left);state[1]=transport.pulse;
        state[2]=transport.external_age;state[3]=transport.control_acc;
#else
        state[0]=bits(pulse_left);state[1]=(uint32_t)pulse;
        state[2]=ext_age;state[3]=ctl_acc;
#endif
        state[4]=bits(step_len);state[5]=en_ext;state[6]=en_playing;
        state[7]=sixteenth;state[8]=(uint32_t)step_t;state[9]=en_prof[9];state[10]=en_jam_revision;
        trace_words(state,11);
        for(tempo=0;tempo<11;tempo++)state_hash=word(state_hash,state[tempo]);
    }
    printf("{\"scenario\":%u,\"frames\":%u,\"chunks\":%u,\"state_hash\":%u,\"pcm_hash\":%u,\"midi_hash\":%u,\"midi_messages\":%u,\"midi_clocks\":%u}\n",
        scenario,render_frame,en_prof[9],state_hash,pcm_hash,midi_hash,midi_count,clock_count);
    if(trace && fclose(trace)){perror("transport trace close");return 2;}
    return 0;
}
