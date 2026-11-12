/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile this driver with the untouched R01 include path for the independent
 * oracle; it invokes the actual old kernels and full instrument, not a DSP model. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <assert.h>
#include "amb.c"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static FILE *trace;
static uint32_t hash=2166136261u;
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
static void state(void){
#include "effects_state.inc"
    record(&rng,sizeof rng);record(&tape_w,sizeof tape_w);record(&plex_w,sizeof plex_w);record(&head_left,sizeof head_left);
    {const float x[]={echo_len,echo_len_t,head2,head2_t,head2_v,fb_lean,tape_acc,tape_lp,tape_hp,tape_p1,tape_p2,tape_c1,tape_c2,plex_lp,plex_lp2,pl_decay,pl_dd2,pl_bw,pl_damp_l,pl_damp_r,rg_env,rg_g,rg_hold,rv_shim_fb,sh_ph,sh_dc,lfo_s,lfo_c,lk_cur,cp_env,cp_g,gr_hold,pk_hold};record(x,sizeof x);}
    {int i;for(i=0;i<L_N;i++)record(&dl[i].i,sizeof dl[i].i);}
}
static void output(float *l,float *r,unsigned n,int32_t *out,int16_t *usb,float gain,float ug,int shed)
{
#if EN_RUST_EFFECTS
    fm1_rust_fx_output(l,r,n,out,usb,gain,ug,(uint32_t)shed);
#else
    unsigned i;for(i=0;i<n;i++){
        out[2*i]=(int32_t)((shed?soft_sat(l[i]*gain):fm_tanhf(l[i]*gain))*8300000.0f);
        out[2*i+1]=(int32_t)((shed?soft_sat(r[i]*gain):fm_tanhf(r[i]*gain))*8300000.0f);
    }
    if(usb)for(i=0;i<n;i++){usb[2*i]=(int16_t)(fm_tanhf(l[i]*ug)*32421.0f);usb[2*i+1]=(int16_t)(fm_tanhf(r[i]*ug)*32421.0f);}
#endif
}
int main(int argc,char **argv)
{
    const unsigned sizes[]={1,3,4,5,63,64,127,128,129,255,256};
    unsigned mode=argc>1?(unsigned)atoi(argv[1]):0,block,j,n,total=0;float peak=0.0f;uint32_t noise=91;
    int32_t pcm[2048];int16_t usb[2048];float left[256],right[256];
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}
    en_init();
    if(mode==0){
        for(block=0;block<1800;block++){
            n=sizes[block%11];
            if(block%181==0)master_dyn_reset();
            par[P_MC_RATIO]=(int16_t)((block/73)%7);par[P_MC_THR]=(int16_t)((block*7)%101);
            par[P_MC_MAKE]=(int16_t)((block*11)%101);par[P_ML_CEIL]=(int16_t)((block*13)%101);master_dyn_set();
            for(j=0;j<n;j++){
                noise=noise*1664525u+1013904223u;
                left[j]=(float)(int32_t)noise*(1.0f/2147483648.0f)*1.4f;
                right[j]=-left[j]*0.75f;
                if(block%181<5)left[j]=right[j]=0.0f;
                if(block%181==0 && j==0)left[j]=4.0f;
                if(block%97==0 && j==n-1){uint32_t nan=0x7fc12345u;memcpy(&right[j],&nan,4);}
            }
            master_dyn_block(left,right,n);
            record(left,n*4);record(right,n*4);record(lk_l,sizeof lk_l);record(lk_r,sizeof lk_r);record(lk_gm,sizeof lk_gm);record(lk_sub,sizeof lk_sub);
            {const float scalar[]={lk_cur,lk_sum,lk_rs,cp_env,cp_g,lk_subm,cp_thr,cp_slope,cp_make,cp_ka,cp_kr,lk_ceil,gr_hold,pk_hold,en_comp_gr};
             const uint32_t indices[]={lk_chk,lk_i,lk_n,lk_si};record(scalar,sizeof scalar);record(indices,sizeof indices);}
            output(left,right,n,pcm,usb,block%5==0?0.0f:block%3==0?2.0f:0.35f,1.0f,(int)(block%2));
            record(pcm,n*8);record(usb,n*4);total+=n;
            for(j=0;j<n;j++){assert(left[j]==left[j]&&right[j]==right[j]);peak=fm_maxf(peak,fm_fabsf(left[j]));assert(pcm[j*2]>=-8300000&&pcm[j*2]<=8300000);}
        }
    }else if(mode==30||mode==31){
        unsigned kind,i;en_playing=1;
        par[P_RV_DAMP]=100;par[P_RV_X]=100;par[P_RV_PRE]=100;
        par[P_MC_RATIO]=6;par[P_MC_THR]=20;par[P_MC_MAKE]=75;par[P_ML_CEIL]=35;
        fx_req=FXB(FX_TALK)|FXB(FX_WAH)|FXB(FX_SWEEP)|FXB(FX_RING)|FXB(FX_PHASER)|FXB(FX_FLANGER)|FXB(FX_TAPE);
        for(kind=0;kind<3;kind++){
            plate_init();memset(tape,0,sizeof tape);memset(plex,0,sizeof plex);memset(sh_buf,0,sizeof sh_buf);memset(rb,0,sizeof rb);
            tape_w=tape_ph=plex_w=0;tape_acc=tape_lp=tape_hp=tape_p1=tape_p2=tape_c1=tape_c2=plex_lp=plex_lp2=0.0f;
            echo_len=echo_len_t=1000.0f;echo_fbx=0.86f;head2=0.75f;head2_v=0.0f;pl_decay=0.9995f;pl_dd2=0.5f;par[P_RV_TYPE]=(int16_t)kind;
            reverb_set();master_dyn_reset();master_dyn_set();
            for(i=0;i<132352;i+=128){
                fx_control(128);
                for(j=0;j<128;j++){
                    float el,er,xl,xr,rl,rr,in=mode==30&&i+j==0?0.75f:0.0f;
                    echoes(in,&el,&er,&xl,&xr);reverb_tank(in,&rl,&rr);
                    left[j]=el*0.75f+xl*0.15f+rl;right[j]=er*0.75f+xr*0.15f+rr;fx_master(&left[j],&right[j]);
                }
                master_dyn_block(left,right,128);output(left,right,128,pcm,usb,0.5f,1.0f,0);
                record(pcm,sizeof(int32_t)*256);record(usb,sizeof(int16_t)*256);state();total+=128;
                for(j=0;j<128;j++){assert(left[j]==left[j]&&right[j]==right[j]);peak=fm_maxf(peak,fm_fabsf(left[j]));}
            }
        }
    }else{
        en_seed(91);en_genre((int)((mode-1)%EN_NGENRE));en_play(1);
        en_set(P_REVERB,85);en_set(P_ECHO,80);en_set(P_PL_CHO,75);en_set(P_MC_RATIO,6);en_set(P_MC_THR,20);en_set(P_MC_MAKE,75);en_set(P_ML_CEIL,35);
        if(mode>=2){en_set(P_PAN+4,15);en_set(P_SEND+4,75);en_set(P_PAN+3,80);en_set(P_SEND+3,90);en_fxk(EN_FXK_FILTER,-40);en_fxk(EN_FXK_CRUSH,70);}
        for(block=0;block<6000;block++){
            n=sizes[block%11];
            if(mode>=20&&block%24==0)en_clock(EN_CLK_TICK); /* external clock: direct, unquantized engagement paths */
            if(block==1200)en_fx(mode==2?FXB(FX_TALK)|FXB(FX_WAH)|FXB(FX_SWEEP)|FXB(FX_RING):FXB(FX_TAPE)|FXB(FX_PHASER)|FXB(FX_FLANGER));
            if(mode>=3 && block>=600 && block<4200 && block%300==0){
                static const uint8_t ringfx[]={FX_RPT2,FX_RPT16,FX_RPT8T,FX_REV,FX_STOP,FX_SCATTER,FX_FREEZE,FX_SCRATCH,FX_BRAKE,FX_SPIN,FX_HALF,FX_RVSWELL};
                const unsigned phase=block/300-2;
                uint64_t mask=mode==3?FXB(FX_TFILT)|FXB(FX_TDELAY)|FXB(FX_TPHASE):FXB(ringfx[(phase+mode-4)%12]);
                if(mode>=5)mask|=FXB(FX_PUMP)|FXB(FX_GATEX)|FXB(FX_RISER)|FXB(FX_VINYL)|FXB(FX_ROLL);
                if(mode>=6)mask|=FXB(FX_TALK)|FXB(FX_WAH)|FXB(FX_SWEEP)|FXB(FX_RING)|FXB(FX_PHASER)|FXB(FX_FLANGER)|FXB(FX_TAPE);
                if(mode==7){mask|=FXB(FX_FLEX);en_flex_select((int)(phase*2));}
                if(mode==8)mask=phase&1?~0ull:0ull; /* simultaneous priorities, mute/solo and fade-out */
                en_fx(mask);
                en_fxs(EN_FXS_TF_TYPE,(int)(phase%5));en_fxs(EN_FXS_TF_TGT,(int)(phase%EN_NPT));en_fxs(EN_FXS_TD_TGT,(int)((phase+3)%EN_NPT));en_fxs(EN_FXS_TP_TGT,(int)((phase+5)%EN_NPT));
                en_fxs(EN_FXS_PH_FB,100);en_fxs(EN_FXS_FL_FB,100);en_fxs(EN_FXS_TD_FB,100);en_fxs(EN_FXS_WH_RESO,100);
                en_fxs(EN_FXS_RP_PITCH,(int)(phase%2?24:0));en_fxs(EN_FXS_RP_DECAY,100);en_fxs(EN_FXS_RP_GATE,(int)(phase%4));en_fxs(EN_FXS_RP_REFRESH,(int)(phase%3));
                en_fxs(EN_FXS_SW_TYPE,(int)(phase%3));en_fxs(EN_FXS_PM_RATE,(int)(phase%5));en_gate_pat(phase&1?0x5249:0xffff);
                en_trk(phase&1?0x12:0,phase%3==0?0x84:0);en_fxk(EN_FXK_WASH,phase&1?100:0);en_fxk(EN_FXK_THROW,phase&1?100:0);
            }
            if(block==1800 && mode==2){en_fxs(EN_FXS_WH_RESO,100);en_fxs(EN_FXS_WH_RANGE,100);en_fxk(EN_FXK_FILTER,40);}
            if(block==2200){en_fx(0);en_set(P_RV_TYPE,1);en_set(P_RV_X,100);}
            if(block==3200){en_set(P_RV_TYPE,2);en_set(P_EVOLVE,100);}
            if(block==4200)en_play(0);
            if(block==4800)en_shed_level=3;
            if(block==5400){en_play(1);en_shed_level=0;}
            if(block==4200){en_fx(0);en_trk(0,0);en_fxk(EN_FXK_THROW,0);en_fxk(EN_FXK_WASH,0);}
            if(mode==8&&block==2600)en_freeze(1);if(mode==8&&block==3600)en_freeze(0);
            en_usb_out=usb;en_render(pcm,n,block%17==0?0:2048);record(pcm,n*8);record(usb,n*4);state();total+=n;
            if(block%100==0){record(rb,sizeof rb);record(tape,sizeof tape);record(plex,sizeof plex);record(pl_buf,sizeof pl_buf);record(sh_buf,sizeof sh_buf);record(ch_buf,sizeof ch_buf);}
            for(j=0;j<n*2;j++){assert(pcm[j]>=-8300000&&pcm[j]<=8300000);peak=fm_maxf(peak,(float)(pcm[j]<0?-pcm[j]:pcm[j])/8388608.0f);}
        }
    }
    if(mode==31)assert(peak==0.0f);else assert(peak>0.0001f);printf("{\"mode\":%u,\"frames\":%u,\"trace_hash\":\"%08x\",\"peak\":%.9g}\n",mode,total,hash,(double)peak);
    if(trace)fclose(trace);return 0;
}
