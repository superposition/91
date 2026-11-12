/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual exported helpers and legacy callbacks from an untouched R01 include,
 * fresh C0 or the real Rust-enabled engine. No duplicated harmonic model. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "amb.c"
#if EN_RUST_HARMONY
extern uint32_t fm1_rust_jam_stat(uint32_t);
#endif
static FILE *trace;static uint32_t hash=2166136261u,rows,messages;
static void record(const void *p,size_t n){size_t i;const uint8_t *b=p;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){uint8_t p[3]={(uint8_t)s,(uint8_t)a,(uint8_t)b};record(p,sizeof p);messages++;}
static void state(void){
#include "harmony_state.inc"
    unsigned i;record((const void *)en_chord,sizeof en_chord);record((const void *)&en_chord_root,1);record((const void *)&en_chord_n,1);
    for(i=0;i<4;i++)record(&pad[i].target,sizeof pad[i].target);
    {const float f[]={bass.gtarget,bass.target,bass.note,bass.env};record(f,sizeof f);}
    record(&rng,sizeof rng);record(&prng,sizeof prng);rows++;
}
int main(int argc,char **argv){
    const int mode=argc>1?atoi(argv[1]):0;int i,j,k,g;
    trace=argc>2?fopen(argv[2],"wb"):0;assert(argc<=2||trace);en_init();
    if(mode==0){
        const int roots[]={-24,-1,0,1,5,11,12,100000};unsigned mask;
        for(mask=0;mask<4096;mask++){
            int st[12],n,degree[3];for(i=0;i<12;i++)st[i]=0x6a6a6a6a;
            n=en_scale(mask|0xf0000000u,st);record(&n,4);record(st,sizeof st);rows++;
            degree[0]=-1;degree[1]=0;degree[2]=n+1;
            for(i=0;i<8;i++)for(j=0;j<3;j++)for(k=-1;k<=8;k++){
                int pcs[4]={0x5b5b5b5b,0x5b5b5b5b,0x5b5b5b5b,0x5b5b5b5b};
                int count=en_chord_kind(mask,roots[i],degree[j],k,pcs);
                record(&count,4);record(pcs,sizeof pcs);rows++;
            }
        }
    }else if(mode==1){
        const int regs[4][2]={{48,77},{24,53},{79,108},{48,53}};int f,t,nt,seed;
        for(seed=0;seed<24;seed++)for(f=0;f<5;f++)for(t=0;t<5;t++)for(nt=0;nt<=4;nt++)for(g=0;g<4;g++){
            int arena[16];for(i=0;i<16;i++)arena[i]=36+(i*7+seed*3)%65;
            for(i=0;i<4;i++)arena[8+i]=((int[]){0,4,7,11})[i]+seed%12;
            en_voice_lead(arena+f,arena+8,nt,arena+t,regs[g][0],regs[g][1]);
            record(arena,sizeof arena);rows++;
        }
    }else if(mode==2){
        int key,kind,voicing,ped,md;
        for(g=0;g<19;g++)for(md=0;md<8;md++)for(key=0;key<12;key++)for(kind=0;kind<8;kind++)for(voicing=0;voicing<2;voicing++)for(ped=0;ped<4;ped++){
            chord_genre=(uint8_t)g;pool=EN_MODE_POOL[md];scale_set();deg=(key+g+md)%nsteps;
            par[P_ROOT]=(int16_t)key;par[P_CHORD]=(int16_t)kind;par[P_VOICING]=(int16_t)voicing;pedal=(uint8_t)ped;
            par[P_PAD_OCT]=(int16_t)(g%5-2);par[P_OCT]=(int16_t)(key%5-2);
            pg_len=8;pg_i=(uint8_t)(key%8);pg_on=(uint8_t)(key&1);for(i=0;i<8;i++)pg_kind[i]=(uint8_t)((i+md)%8);
            if(md&1)dom_apply();
            if(nsteps==7&&(key&2)){for(i=0;i<7;i++)steps[i]=AEOLIAN[i];dom_on=0;en_chord_alt=2;}
            for(i=0;i<4;i++)pad[i].target=(float)(38+i*17+(key+kind)%7)+.5f;
            pad_retarget();
            {int preview[4]={101,102,103,104};int count=en_prog_chord((key+3)%12,kind,preview);record(&count,4);record(preview,sizeof preview);}
            state();
        }
    }else if(mode==3){
        int lp,md,step;en_playing=1;par[P_MIDI]=1;par[P_DE_TRIG]=0;
        for(g=0;g<19;g++)for(lp=0;lp<7;lp++)for(md=0;md<8;md++){
            chord_genre=(uint8_t)g;par[P_ROOT]=(int16_t)((g+md+lp)%12);par[P_CHORD]=(int16_t)((g+md)%8);par[P_CH_LOOP]=(int16_t)lp;par[P_DRIFT]=(int16_t)((md*17+g)%101);
            naiwan_pending=-1;naiwan_rotate=0;pg_on=0;pg_i=0;pg_len=0;drop_tonic=0;drift_unit=(uint8_t)(lp%4);
            dh_n=dh_i=dh_ln=hist_n=0;deg=0;deg_prev=0;cad_n=250;pool_set(EN_MODE_POOL[md]);
            rng=(g==0&&lp==0&&md==0)?0u:(uint32_t)(91+g*1709+lp*151+md*29);
            for(step=0;step<64;step++){
                if(step==19)drop_tonic=1;
                if(step==31){pool_set(0xfffu);par[P_CH_LOOP]=(int16_t)((lp+3)%7);}
                if(step==43)pool_set(EN_MODE_POOL[md]);
                chord_change();state();
            }
        }
    }else if(mode==4){
        /* Actual control accumulation is1..383 frames; the already-ported FX
         * control rejects unsupported zero/oversized calls independently. */
        const unsigned ns[]={1,31,127,128,129,255,256,383};unsigned n,block;
        for(block=0;block<4096;block++){
            n=ns[block%8];chord_genre=(uint8_t)(block%19);drift_unit=(uint8_t)(block%4);par[P_DRIFT]=(int16_t)(block%101);
            pg_i=(uint8_t)(block%8);pg_len=8;pg_on=(uint8_t)(block&1);pg_bars[pg_i]=(uint8_t)(block%4);
            rng=block*1664525u+1013904223u;hr_left=(int32_t)(block%36)-1;step_len=3675.0f+(float)(block%500);drift_total=(float)(block%7)*1741.25f;drift_left=drift_total*.731f;
            if(block&1)drift_reset();else apply(P_DRIFT,(int)(block%101));
            {int bars0=hr_bars(),bars1=hr_bars_v(par[P_DRIFT]);float secs=drift_secs(par[P_DRIFT]);record(&bars0,4);record(&bars1,4);record(&secs,4);}
            arc_left=block%3?n+1:n;arc=(float)((int)block%11-5)*.1f;arc_tgt=.71f;brk=.3f;motion=(float)(block%101)*.01f;sec=(int)(block%3);en_playing=(uint8_t)(block&1);
            control(n);
            {const float f[]={arc,arc_tgt,brk,arcm};record(f,sizeof f);record(&arc_left,4);record((const void *)&en_arc,4);}
            state();
        }
    }else if(mode==5){
        uint8_t b[6],kinds[3];unsigned mask,step;en_playing=0;
        for(mask=0;mask<512;mask++){
            chord_genre=(uint8_t)(mask%19);par[P_ROOT]=(int16_t)(mask%12);par[P_CHORD]=(int16_t)(mask%8);par[P_CH_LOOP]=(int16_t)(mask%7);par[P_DRIFT]=(int16_t)(mask%101);
            en_playing=(uint8_t)(mask&1);naiwan_pending=-1;naiwan_rotate=0;pool_set(mask&8?0xfffu:EN_MODE_POOL[mask%8]);
            memset(b,0xa5,sizeof b);en_prog_capture(b);record(b,sizeof b);
            memset(b,0,sizeof b);memset(kinds,0,sizeof kinds);
            for(i=0;i<8;i++){en_prg_put(b,3*i,3,(mask>>i&1u)?(unsigned)(i%7+1):0u);en_prg_put(b,24+2*i,2,(mask+i)%4);en_prg_put(kinds,3*i,3,(mask+i)%8);}
            en_prg_put(b,40,1,mask>>2&1u);en_prg_put(b,41,2,mask>>3&3u);en_prog(b);en_prog_kinds(kinds);drain();state();
            for(step=0;step<12;step++){
                if(step==3){drop_tonic=1;chord_change();}
                else if(step==8){int16_t p[P_NPARAMS];uint16_t pool;en_piece_g(0,(int)(mask%10),p,&pool);chord_change();}
                else chord_change();
                drain();memset(b,0xa5,sizeof b);en_prog_capture(b);record(b,sizeof b);state();
            }
        }
    }else return 2;
#if EN_RUST_HARMONY
    assert(fm1_rust_jam_stat(79)==0);
#endif
    if(mode==3)assert(en_host_alt[1]>0&&en_host_alt[2]>0);
    if(trace)fclose(trace);printf("{\"mode\":%d,\"rows\":%u,\"midi_messages\":%u,\"trace_hash\":\"%08x\",\"alterations\":[%d,%d,%d],\"diminished_moves\":%d}\n",mode,rows,messages,hash,en_host_alt[0],en_host_alt[1],en_host_alt[2],en_host_dimwalk);return 0;
}
