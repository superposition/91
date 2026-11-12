/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile against untouched R01, fresh C0 and Rust: actual setting/reset/grid
 * callbacks and complete output/fade paths, without a duplicate DSP model. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static FILE *trace;static uint32_t hash=2166136261u,rows;
static void record(const void *p,size_t n){size_t i;const uint8_t *b=p;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(fwrite(p,1,n,trace)!=n)exit(2);}
static void effects(void){
#include "effects_state.inc"
    record((const void *)&en_flx_done,sizeof en_flx_done);record(&rng,sizeof rng);rows++;
}
int main(int argc,char **argv){
    int mode=argc>1?atoi(argv[1]):0,i,j;uint32_t frames=0;
    trace=fopen(argc>2?argv[2]:"effects-completion.trace","wb");assert(trace);en_init();
    if(mode==0){
        for(i=-1;i<=101;i++){
            apply(P_ECHO,i);apply(P_REVERB,i);apply(P_SIZE,i);apply(P_NZ_PUMP,i);
            {float s[]={echo_send,rev_send,pl_decay_t,echo_fb,nz_pump};record(s,sizeof s);rows++;}
        }
        for(i=PARAMS[P_TEMPO].lo;i<=PARAMS[P_TEMPO].hi;i++){apply(P_TEMPO,i);record(&duck_dec,4);record(&step_len,4);rows++;}
        for(i=-1;i<=EN_NGENRE;i++)for(j=-1;j<=10;j++){en_trim(i,j);record(&trim_to,4);rows++;}
        /* Real zero-frame renders still smooth trim and publish mute/USB/hush. */
        for(i=0;i<800;i++){
            int32_t out[2]={17,-19};int16_t usb[2]={23,-29};
            en_usb_out=i&1?usb:0;muted=(uint8_t)((i/7)&1);hush=(uint8_t)((i/11)&1);
            trim_to=(float)(i%127)*0.01f;
            en_render(out,0,(uint32_t)i*17u);
            {float s[]={trim_k,usb_gain,gap_g,hush_g};record(s,sizeof s);record((const void *)&en_hushed,1);record(out,sizeof out);record(usb,sizeof usb);rows++;}
        }
    }else if(mode==1){
        int pat,q,m,h,p,st;en_flx_done=0xfffffffeu;
        for(pat=0;pat<EN_NFLEX;pat++)for(q=0;q<3;q++)for(m=0;m<2;m++)for(h=0;h<2;h++)for(p=0;p<2;p++){
            bmode=BM_LIVE;flx_active=0;flx_pos=0;flx_pat=0;flx_arm=(uint8_t)(pat+1);flx_held=(uint8_t)h;flx_phys=(uint8_t)p;flx_wait_rel=0;
            fxs_v[EN_FXS_FX_QUANT]=(int16_t)q;fxs_v[EN_FXS_FX_MODE]=(int16_t)m;fxs_v[EN_FXS_FX_MIX]=(int16_t)((pat*7)%101);
            rb_w=0xfffffff0u+(uint32_t)pat;step_len=pat&1?17000.5f:2500.25f;
            for(st=0;st<64;st++){
                if(st==19)flx_arm=(uint8_t)((pat+1)%EN_NFLEX+1);
                if(st==37)bmode=BM_STOP;
                flx_tick(st&15);effects();
            }
        }
        for(i=0;i<9;i++)for(j=0;j<3;j++){
            step_len=15000.25f;rb_w=0xfffffffd;rp_pit=24.0f;rp_t=2.75f;bmode=BM_LIVE;rpt_engage(i);
            rp_ref=(uint32_t)j;rp_ok=(uint32_t)(i&1);
            for(st=0;st<32;st++){rpt_refresh(st&15);effects();}
            var_leave();effects();
        }
    }else if(mode==2){
        const unsigned sizes[]={0,1,3,4,5,63,64,127,128,129,255,256};
        int32_t pcm[516];int16_t usb[516];unsigned block,k,n;en_seed(91);en_play(1);en_set(P_ECHO,90);en_set(P_REVERB,90);en_set(P_SIZE,100);
        for(block=0;block<1100;block++){
            n=sizes[block%12];gap_on=(uint8_t)((block/17)&1);en_hush((int)((block/31)&1));en_mute((int)((block/47)&1));en_trim((int)(block%EN_NGENRE),(int)(block%10));
            for(k=0;k<516;k++){pcm[k]=0x5a5a5a5a;usb[k]=0x5a5a;}
            en_usb_out=block%3?usb+2:0;en_render(pcm+2,n,block%17==0?0:2048);
            assert(pcm[0]==0x5a5a5a5a&&pcm[1]==0x5a5a5a5a&&pcm[2*n+2]==0x5a5a5a5a&&pcm[2*n+3]==0x5a5a5a5a);
            assert(usb[0]==0x5a5a&&usb[1]==0x5a5a&&usb[2*n+2]==0x5a5a&&usb[2*n+3]==0x5a5a);
            record(pcm,sizeof pcm);record(usb,sizeof usb);
            {float s[]={trim_to,trim_k,usb_gain,gap_g,hush_g};record(s,sizeof s);record((const void *)&en_hushed,1);}
            frames+=n;effects();
        }
    }else if(mode==4){
        unsigned n;const unsigned sizes[]={1,3,63,256};
        for(i=0;i<=100;i++)for(j=0;j<=4;j++){
            unsigned k;apply(P_PL_CHO,i);en_shed_level=(uint8_t)j;n=sizes[(unsigned)(i+j)%4];
            for(k=0;k<n;k++){pad_ol[k]=(float)((int)k-127)*.001f;pad_or[k]=-pad_ol[k]*.7f;}
            pad_chorus(n);record(pad_ol,n*4);record(pad_or,n*4);record(ch_buf,sizeof ch_buf);record(ch_ph,sizeof ch_ph);record(&ch_i,4);rows++;frames+=n;
        }
    }else if(mode==3){
        for(j=0;j<64;j++){
            for(i=0;i<DL_TOTAL;i++)pl_buf[i]=(int16_t)(i*17+j);
            for(i=0;i<SH_N;i++)sh_buf[i]=(float)(i+j)*0.0001f;
            for(i=0;i<L_N;i++)dl[i].i=i+j;
            rg_env=.37f;rg_g=.26f;rg_hold=19.0f;rv_shim_fb=.19f;sh_ph=.42f;sh_w=123;sh_dc=.3f;pl_bw=.14f;pl_damp_l=.23f;pl_damp_r=.72f;lfo_s=.3f;lfo_c=.8f;
            plate_init();record(pl_buf,sizeof pl_buf);record(sh_buf,sizeof sh_buf);
            {float s[]={rg_env,rg_g,rg_hold,rv_shim_fb,sh_ph,sh_dc,pl_bw,pl_damp_l,pl_damp_r,lfo_s,lfo_c};record(s,sizeof s);record(&sh_w,4);}
            for(i=0;i<L_N;i++)record(&dl[i].i,4);
            for(i=0;i<TAPE_N;i++)tape[i]=(int16_t)(i*11);
            for(i=0;i<PLEX_N;i++)plex[i]=(int16_t)(i*13);
            tape_w=17;plex_w=31;tape_ph=3;head2=.66f;head2_t=.5f;head2_v=.01f;echo_len=2000;
            tape_acc=.19f;tape_lp=.23f;tape_hp=.31f;tape_p1=.41f;tape_p2=.51f;tape_c1=.61f;tape_c2=.71f;plex_lp=.81f;plex_lp2=.91f;
            en_init();record(tape,sizeof tape);record(plex,sizeof plex);
            {float s[]={head2,head2_t,head2_v,echo_len,tape_acc,tape_lp,tape_hp,tape_p1,tape_p2,tape_c1,tape_c2,plex_lp,plex_lp2};record(s,sizeof s);}
            record(&tape_w,4);record(&tape_ph,4);record(&plex_w,4);rows++;
        }
    }else return 2;
    fclose(trace);printf("{\"mode\":%d,\"rows\":%u,\"frames\":%u,\"trace_hash\":\"%08x\"}\n",mode,rows,frames,hash);return 0;
}
