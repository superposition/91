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
    if(scenario==11){
        unsigned test,step,round,value;printf("[");
        for(test=0;test<64;test++){
            static const int major[12]={0,2,4,5,7,9,11,0,0,0,0,0};uint32_t h=2166136261u;
            nsteps=7;chord_nt=3;deg=(int)(test%7);for(step=0;step<12;step++)steps[step]=major[step];chord_pcs[0]=0;chord_pcs[1]=4;chord_pcs[2]=7;chord_pcs[3]=0;
            par[P_ROOT]=(int16_t)(test%12);par[P_OCT]=(int16_t)((int)(test%7)-3);par[P_AC_OCT]=(int16_t)((int)(test%5)-2);
            par[P_AC_LEN]=(int16_t)(1+test%16);par[P_AC_DENS]=(int16_t)(test*17%101);par[P_AC_RANGE]=(int16_t)(1+test%3);par[P_AC_SLIDE]=(int16_t)(test*31%101);
            par[P_AC_MUT]=(int16_t)(test*29%101);par[P_AC_CHG]=(int16_t)(test%4);par[P_AC_LOCK]=(int16_t)(test%7==0);evolve=(float)(test%10)*0.1f;
            ac_gate=ac_accent=ac_slide=0;for(step=0;step<16;step++)ac_off[step]=ac_oct[step]=0;rng=(test+1u)*2654435761u;ac_regrow();
            for(round=0;round<9;round++){
                if(round)ac_mutate();
                h=word(word(word(h,ac_gate),ac_accent),ac_slide);for(step=0;step<16;step++)h=word(word(h,(uint8_t)ac_off[step]),(uint8_t)ac_oct[step]);h=word(h,rng);
                for(value=0;value<45;value++)for(step=0;step<16;step++)h=word(h,(uint32_t)en_acid_preview((int)step,(int)value));
            }printf("%s%u",test?",":"",h);
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
    if(scenario>=35){
        unsigned test=scenario-35;en_set(P_AC_LEN,1+(int)test%16);en_set(P_AC_RESP,5+(int)(test*7)%40);en_set(P_AC_EVERY,(int)test%8);en_set(P_AC_CHANCE,(int)test%2);
        en_set(P_AC_MUT,100);en_set(P_AC_CHG,(int)test%4);en_set(P_AC_DENS,65);en_set(P_AC_RANGE,1+(int)test%3);en_set(P_AC_SLIDE,80);en_set(P_AC_LOCK,0);
        en_set(P_CM_LVL,80);en_set(P_CM_DENS,90);en_set(P_CM_MOTIF,100);en_set(P_CM_VU,60);en_set(P_CM_LOOP,(int)test%4);en_set(P_CM_RATE,(int)test%4);en_set(P_CM_STYLE,(int)test%6);en_set(P_CM_LEAP,80);en_set(P_CM_LOCK,0);
        en_set(P_BL_MODE,1+(int)test%16);en_set(P_BL_MOVE,100);en_set(P_BL_LEN,100);en_pedal((int)test&1);
    }
    en_set(P_TEMPO,70);en_play(1);
    for(i=0;i<(scenario>=35?10000u:5000u);i++) {
        if(scenario>=35){
            unsigned test=scenario-35;
            if(i==200){en_acid_capture(3,30+(int)test*3,1,1);en_pluck_capture(2,48+(int)test);}
            if(i==350){en_acid_step(1,(int)test%12,(int)test%4,1,1,1);en_pluck_step(1,(int)test%12,3);}
            if(i==600){en_set(P_AC_MAN,1);en_set(P_CM_MAN,1);en_set(P_DR_MAN,1);}
            if(i==850){en_set(P_AC_MAN,0);en_set(P_CM_MAN,0);en_set(P_DR_MAN,0);}
            if(i==1100){en_set(P_AC_LOCK,1);en_set(P_CM_LOCK,1);en_set(P_CM_LOOP,2);}
            if(i==1300){en_set(P_AC_LOCK,0);en_set(P_CM_LOCK,0);en_set(P_CM_LOOP,3);}
            if(i==1600){en_set(P_AC_RESP,5+(int)(test*11)%40);en_set(P_AC_EVERY,(int)(test+3)%8);en_set(P_AC_CHANCE,(int)(test+1)%2);en_set(P_AC_LEN,1+(int)(test+7)%16);}
            if(i==2600){static const uint8_t empty[16]={0};en_bass_steps_set(empty);}
            if(i==2700){en_set(P_BL_MODE,EN_BL_STEPS);en_bass_capture();}
            if(i==3000){en_bass_step(5,1|3<<3|1<<6);en_bass_step(9,4|5<<3|2<<6);}
            if(i==3400){en_set(P_CM_MOTIF,35);en_set(P_CM_STYLE,(int)(test+3)%6);en_set(P_CM_VU,100);en_set(P_AC_MUT,35);}
            if(i==4000){en_pedal((int)(test+1)&1);en_set(P_BL_MODE,5);}
            if(i==4100)en_fx(1ull<<FX_SCATTER);
            if(i==4200)en_fx(1ull<<FX_RVSWELL);
            if(i==4300)en_fx(1ull<<FX_RPT8);
            if(i==4400)en_fx(1ull<<FX_BRAKE);
            if(i==4500)en_fx(0);
            /* Let a complete phrase record and develop after the stop/start,
             * external-clock and DejaVu exercises above. */
            if(i==5000){en_set(P_CM_VU,0);en_set(P_CM_MOTIF,100);en_set(P_CM_LOCK,0);en_set(P_CM_DENS,100);en_set(P_CM_RATE,3);}
        }
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
            {const uint32_t melodic[]={0x4c,rng,ac_gate,ac_accent,ac_slide,(uint32_t)ac_pos,ac_loop,ac_loop0,ac_bar0,ac_ans_bar,ac_ans_loop,ac_resync_bar,ac_resync_loop,
                (uint32_t)cm_i,cm_k,cm_sync,mo_play,(uint32_t)(int32_t)mo_last,mo_valid,mo_rec,mo_on,mo_inv,mo_alt,mo_restate,mo_first,mo_idx0,mo_prev,bs_set,bl_beat,(uint32_t)(int32_t)en_bs_pos,
                (uint32_t)ac_pend.n,ac_pend.slid_ok,ac_pend.acc,ac_pend.hat,ac_pend.slide,ac_pend.beat,bits(ac_pend.dec),(uint32_t)bl_pn,bl_pfl,(uint32_t)cm_pnote,bits(cm_pvel),
                (uint32_t)hr_left,bits(drift_left),bits(drift_total),change_pending};
            trace_words(melodic,sizeof melodic/4);trace_words(ac_off,4);trace_words(ac_oct,4);trace_words(cm_f,8);trace_words(cm_g,8);trace_words(cm_ix,8);trace_words(cm_vl,8);trace_words(mo_k,4);trace_words(bs,4);trace_words(bs_hist,4);trace_words(cm_mn,2);trace_words(cm_moff,8);}
        }
        for(tempo=0;tempo<11;tempo++)state_hash=word(state_hash,state[tempo]);
    }
    printf("{\"scenario\":%u,\"render_calls\":%u,\"frames\":%u,\"chunks\":%u,\"state_hash\":%u,\"pcm_hash\":%u,\"midi_hash\":%u,\"midi_messages\":%u,\"midi_clocks\":%u}\n",
        scenario,i,render_frame,en_prof[9],state_hash,pcm_hash,midi_hash,midi_count,clock_count);
#if EN_RUST_SCHEDULER
    assert(fm1_rust_jam_stat(18)>0 && fm1_rust_jam_stat(20)>0);
    if(scenario<16 && (scenario&2u))assert(fm1_rust_jam_stat(21)>0);
    if(scenario>=16)assert(fm1_rust_jam_stat(19)>0);
#endif
#if EN_RUST_MELODIC
    assert(fm1_rust_jam_stat(26)>0 && fm1_rust_jam_stat(27)>0);
    if(scenario>=16)assert(fm1_rust_jam_stat(22)>0 && fm1_rust_jam_stat(23)>0 && fm1_rust_jam_stat(24)>0 && fm1_rust_jam_stat(25)>0);
#endif
    if(trace && fclose(trace)){perror("transport trace close");return 2;}
    return 0;
}
