/* SPDX-License-Identifier: GPL-3.0-only */
/* Include the selected, unedited instrument source. This adapter records the
 * real en_render/command/MIDI path, not a second implementation of its timing.
 * Compile with the frozen R01 include directory for independent evidence, or
 * the current C0/Rust paths for an immediately reproducible differential run. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
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
    if(scenario==9) {
        unsigned density,mask;printf("[");
        for(density=0;density<=100;density++){
            uint32_t h=2166136261u;for(mask=0;mask<65536;mask++){en_pats[0][0]=(uint16_t)mask;h=word(h,en_lane_mask(0,0,(int)density));}
            printf("%s%u",density?",":"",h);
        }puts("]");return 0;
    }
    if(scenario==10) {
        unsigned mode,index,l;printf("[");
        for(mode=1;mode<=2;mode++){
            uint32_t h=2166136261u;apply(P_DD_MUTATE,(int)mode);
            for(index=0;index<4096;index++){
                rng=(index+1u)*2654435761u;
                for(l=0;l<EN_NLANE;l++)en_pats[0][l]=(uint16_t)((index*1103515245u+l*1013904223u)>>((l+index)%16));
                dd_mutate(0);for(l=0;l<EN_NLANE;l++)h=word(h,en_pats[0][l]);h=word(h,rng);
            }printf("%s%u",mode==1?"":",",h);
        }puts("]");return 0;
    }
    if(scenario<16 && (scenario&2u))en_jam_set(JAM_ON,1);
    else {
        unsigned l,s;en_genre(scenario>=16?(int)(scenario-16)%19:7);en_set(P_SWING,100);en_set(P_HUMAN,100);
        for(l=0;l<EN_NLANE;l++) {
            en_set(P_LLEN+(int)l,7+(int)l);
            for(s=0;s<16;s++)en_mod[l][s]=(uint8_t)(1+s%7 | (s%8)<<3 | (s%4)<<6);
        }
    }
    if(scenario>=16) {
        unsigned p,l;uint16_t patterns[EN_NPAT][EN_NLANE];
        en_piece_pats_g((int)(scenario-16)%19,3,patterns);
        for(p=0;p<EN_NPAT;p++)for(l=0;l<EN_NLANE;l++)en_pats[p][l]=patterns[p][l];
        en_set(P_DD_CHANGE,2);en_set(P_DD_MUTATE,2);en_set(P_DD_EVERY,1);en_set(P_GHOST,100);en_set(P_ACCENT,75);en_set(P_KIT_KICK,4);
        for(l=0;l<EN_NLANE;l++){en_set(P_COND+(int)l,(int)((scenario+l)%8));en_set(P_PROB+(int)l,75);en_set(P_DENS+(int)l,20+(int)l*15);en_set(P_LSRC+(int)l,1+(int)l);en_set(P_LLEN+(int)l,l&1?16:5+(int)l);}
    }
    en_set(P_TEMPO,70);en_play(1);
    for(i=0;i<5000;i++) {
        if(scenario>=16) {
            if(i%400==0){en_set(P_CHAIN,1+(int)(i/400)%9);en_set(P_SLICE,9);en_set(P_TIME,(int)(i/400)%3);}
            if(i==300)en_section(EN_CMD_BREAK);
            if(i==500)en_section(EN_CMD_BUILD);
            if(i==1800)en_section(EN_CMD_DROP);
            if(i==2100)en_fill_next();
            if(i==2200)en_fx(1ull<<FX_HATROLL|1ull<<FX_FLAM|1ull<<FX_DBLKICK);
            if(i==2300)en_fx(1ull<<FX_HATTRIP|1ull<<FX_SNROLL);
            if(i==2400)en_fx(0);
            if(i==2700)en_fill_next();
            if(i==2700)en_fx(1ull<<FX_FILL);
            if(i==2800){en_fx(0);en_set(P_SWING,0);en_set(P_HUMAN,0);}
            if(i==3200){en_set(P_SWING,100);en_set(P_HUMAN,100);}
        }
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
        {
            const uint32_t scheduling[]={0x53u,rng,dd_bars,(uint32_t)dd_in,dd_fill,(uint32_t)sec,(uint32_t)sec_bars,(uint32_t)sec_req,
                (uint32_t)pat_now,(uint32_t)pat_next,fill_kind,slice_perm,chain_i,en_dd_changes,en_dd_mutations,
                (uint32_t)cm_wait,(uint32_t)ac_wait,(uint32_t)bl_wait,(uint32_t)stab_wait};
            trace_words(scheduling,sizeof scheduling/4);
            for(tempo=0;tempo<EN_NPAT;tempo++){unsigned l;for(l=0;l<EN_NLANE;l++){uint32_t m=en_pats[tempo][l];trace_words(&m,1);}}
            for(tempo=0;tempo<EN_NDRUM;tempo++){
                const uint32_t delayed[]={(uint32_t)drum[tempo].wait,(uint32_t)drum[tempo].wait2,bits(drum[tempo].pend),bits(drum[tempo].pend2),drum[tempo].rn,(uint32_t)drum[tempo].rgap,bits(drum[tempo].rmul)};
                trace_words(delayed,7);
            }
        }
        for(tempo=0;tempo<11;tempo++)state_hash=word(state_hash,state[tempo]);
    }
    printf("{\"scenario\":%u,\"frames\":%u,\"chunks\":%u,\"state_hash\":%u,\"pcm_hash\":%u,\"midi_hash\":%u,\"midi_messages\":%u,\"midi_clocks\":%u}\n",
        scenario,render_frame,en_prof[9],state_hash,pcm_hash,midi_hash,midi_count,clock_count);
#if EN_RUST_SCHEDULER
    assert(fm1_rust_jam_stat(18)>0 && fm1_rust_jam_stat(20)>0);
    if(scenario<16 && (scenario&2u))assert(fm1_rust_jam_stat(21)>0);
    if(scenario>=16)assert(fm1_rust_jam_stat(19)>0);
#endif
    if(trace && fclose(trace)){perror("transport trace close");return 2;}
    return 0;
}
