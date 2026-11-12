/* SPDX-License-Identifier: GPL-3.0-only */
/* Real private FM calls from the selected instrument source, with field-wise
 * state records (never struct padding) and complete output/scratch bytes. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t a,uint32_t b,uint32_t c){(void)a;(void)b;(void)c;}
static FILE *trace;static uint32_t hash=2166136261u,blocks,frames,events;static float peak;
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;if(trace&&fwrite(p,1,n,trace)!=n)exit(2);}
#define FIELD(s,n) record(&(s)->n,sizeof (s)->n)
static void state(void){unsigned k,j;for(k=0;k<6;k++){fm_slot_t *x=&fmx[k];fm6_voice_t *s=&x->s;
    for(j=0;j<6;j++){fm6_eg_t *e=&s->eg[j];FIELD(e,level);FIELD(e,target);FIELD(e,inc);FIELD(e,hold);FIELD(e,ix);FIELD(e,rising);}
    FIELD(s,ph);FIELD(s,fq);FIELD(s,freq_log);FIELD(s,freq_valid);FIELD(s,gout);FIELD(s,g);FIELD(s,dg);FIELD(s,base);FIELD(s,porta);FIELD(s,ol);FIELD(s,rs);FIELD(s,plan);FIELD(s,fb);
    {fm6_peg_t *e=&s->pe;FIELD(e,pl);FIELD(e,pt);FIELD(e,pi);FIELD(e,pix);FIELD(e,prise);FIELD(e,pr);FIELD(e,pv);}
    FIELD(s,down);FIELD(s,sub);FIELD(s,note);FIELD(s,vel);FIELD(s,eng);FIELD(s,loop);FIELD(s,fbs);FIELD(s,quiet);FIELD(s,played);FIELD(s,frozen);FIELD(s,still);FIELD(s,spb);
    FIELD(x,used);FIELD(x,gate);FIELD(x,part);FIELD(x,age);FIELD(x,gate_left);FIELD(x,kill_left);FIELD(x,fade);
    }
    {fm6_lfo_t *l=&fm_lfo;FIELD(l,ph);FIELD(l,dly);FIELD(l,val);FIELD(l,depth);FIELD(l,rnd);l=&fm_bass_lfo;FIELD(l,ph);FIELD(l,dly);FIELD(l,val);FIELD(l,depth);FIELD(l,rnd);}
    record(&fm_age,sizeof fm_age);record(&fm_cur,sizeof fm_cur);record(&fm_pos,sizeof fm_pos);record(&fm_ams_amd,sizeof fm_ams_amd);record(fm_ams_pt,sizeof fm_ams_pt);record(&fm_ams_ok,sizeof fm_ams_ok);
}
static void block(unsigned tail){unsigned i;fm_gates(64,(int32_t)tail);fm_block();blocks++;frames+=64;
    record(fm_blk,sizeof fm_blk);record(fm6_bus,sizeof fm6_bus);record(fm6_sum,sizeof fm6_sum);record(fm6_ibus,sizeof fm6_ibus);record(fm6_isum,sizeof fm6_isum);state();
    for(i=0;i<64;i++){float v=fm_blk[i];assert(v==v&&v<128.0f&&v>-128.0f);peak=fm_maxf(peak,fm_fabsf(v));}
}
static void note(int pitch,float vel,int32_t gate){fm_note_on(pitch,vel,gate);events++;state();}
static void jam_note(unsigned part,int pitch,float vel,int32_t gate){fm_jam_note(part,pitch,vel,gate);events++;state();}
int main(int argc,char **argv){unsigned scenario=argc>1?(unsigned)atoi(argv[1]):0,i,j,alg,eng,feedback,dy,patch,n;int32_t audio[512];
    if(argc>2){trace=fopen(argv[2],"wb");if(!trace)return 2;}en_init();
    if(scenario==0||scenario==1){
        for(eng=0;eng<3;eng++)for(feedback=0;feedback<2;feedback++)for(alg=0;alg<32;alg++){
            fm_reset();fm_set_patch(0);fm_engine=eng;fm6_float_path=scenario==1;fm_ed[FP_ALG]=(uint8_t)alg;fm_ed[FP_FB]=(uint8_t)(feedback?7:0);
            for(i=0;i<6;i++)note(48+(int)i*3,0.2f+0.13f*(float)i,500+(int32_t)i*97);
            for(i=0;i<64;i++){if(i==8)note(60,.95f,800);if(i==12)note(60,.25f,300);block(2205);}
        }
    }else if(scenario==2){
        for(dy=0;dy<2;dy++)for(patch=0;patch<EN_NFMFACT;patch++){
            fm_reset();en_fm_dy=(uint8_t)dy;fm_set_patch((int)patch);record(fm_ed,sizeof fm_ed);fm_engine=patch%3;
            note(patch%2?0:127,patch%2?0.0f:1.2f,128);note(60,.73f,700);
            for(i=0;i<128;i++){if(i==24)note(64,.9f,200);block(1102);}
        }
    }else if(scenario==3){
        const uint8_t *data=en_jam_builtin(0,&n);assert(en_jam_upload(data,n));en_jam_set(JAM_ON,1);en_render(audio,256,2048);assert(jp_dx());
        fm_jam_load(jp_data);for(i=0;i<12;i++)jam_note(0,48+(int)i,.7f,2000);jam_note(1,36,.7f,2000);
        for(i=0;i<400;i++){if(i%11==0)jam_note(1,28+(int)(i%25),.8f,280);if(i%17==0)for(j=0;j<4;j++)jam_note(0,48+(int)((i+j*4)%25),.5f,600);if(i==80)jp_patch_dirty|=1;if(i==160)jp_patch_dirty|=2;block(1800);for(j=0;j<6;j++)if(fmx[j].used)assert((j==4)==(fmx[j].part==1));}
        /* Frozen-C numerical-age endpoint: preserve the known slot0 exception. */
        for(i=0;i<6;i++)fmx[i].used=0;jam_note(1,36,.7f,4000);fmx[4].age=UINT32_MAX;fm_age=UINT32_MAX;jam_note(1,38,.7f,4000);
        assert(fmx[0].used&&fmx[0].part==1&&fmx[4].used&&fmx[4].part==1);state();
    }else if(scenario==4){
        for(i=0;i<16;i++){uint8_t bytes[128];memcpy(bytes,FM6_DY[i],128);bytes[118]=(uint8_t)(128+'A'+i);assert(en_user_voice((int)i,bytes));drain();}
        assert(en_user_count()==16);record(fm_user,sizeof fm_user);record(fm_uname,sizeof fm_uname);
        for(i=0;i<16;i++){fm_set_patch(EN_NFMFACT+(int)i);record(fm_ed,sizeof fm_ed);note(36+(int)i,.8f,150);for(j=0;j<40;j++)block(600);}
        en_user_clear();drain();assert(en_user_count()==0);fm_set_patch(EN_NFMFACT+15);assert(fm_cur==0);state();
    }else if(scenario==5){
        fm_set_patch(0);for(i=0;i<6;i++)note((int)(i*25),1.0f,128);
        for(i=0;i<700;i++){fm_engine=(i/8)%3;fm6_float_path=(i/5)&1;fm_tune=i%2?139810:-139810;fm_bend=i%3?2796202:-2796202;
            if(i%37==0){fm_ed[FP_LFW]=(uint8_t)((i/37)%8);fm_ed[FP_LFS]=(uint8_t)(i%100);fm_ed[FP_LFD]=(uint8_t)((i*3)%100);note((int)(i%128),1.0f,100);}
            block(1);
        }for(i=0;i<200;i++)block(1);for(i=0;i<6;i++)assert(!fmx[i].used);fm_reset();state();
    }else if(scenario==6||scenario==7){
        uint32_t cursor=UINT32_MAX-34u;const unsigned sizes[]={1,3,31,64,127,255,256};
        if(scenario==7){const uint8_t *data=en_jam_builtin(0,&n);assert(en_jam_upload(data,n));en_jam_set(JAM_ON,1);en_render(audio,256,2048);fm_jam_load(jp_data);assert(jp_dx());}
        else{par[P_CM_SND]=CM_NPLAIN;fm_set_patch(0);}
        cm_vol=.8f;pad_vol=.6f;bass_vol=.7f;
        for(i=0;i<180;i++){
            unsigned count=sizes[i%7];rv_job_t job;memset(&job,0,sizeof job);job.n=(int)count;job.fm_w=cursor;job.fm_on=i%19!=18;rv_job.fm_w=cursor;
            if(i%13==0){if(scenario==7){jam_note(0,48+(int)(i%24),.7f,700);jam_note(1,32+(int)(i%12),.6f,700);}else note(48+(int)(i%24),.7f,700);}
            fm_gates(count,1000);tg_inv=1.0f/(float)count;trk_g[TR_CM]=trk_g[TR_PAD]=.23f;trk_g1[TR_CM]=trk_g1[TR_PAD]=.91f;
            fm_job(&job);fm_ctr=cursor+FM_D;render_fm(count);if(scenario==7)render_bass(count);
            record(fm_fifo,sizeof fm_fifo);if(scenario==7){record(fm_bass_fifo,sizeof fm_staging.bass);record(bass_o,count*sizeof(float));}
            record(fm_ol,count*sizeof(float));record(fm_or,count*sizeof(float));record(fm_rs,count*sizeof(float));record(fm_es,count*sizeof(float));record(&fm_bass_sample,sizeof fm_bass_sample);record(&fm_ctr,sizeof fm_ctr);state();
            for(j=0;j<count;j++)peak=fm_maxf(peak,fm_fabsf(fm_ol[j]));cursor+=count;blocks++;frames+=count;
        }
    }else return 2;
    assert(peak>0.00001f);printf("{\"scenario\":%u,\"blocks\":%u,\"frames\":%u,\"events\":%u,\"hash\":\"%08x\",\"peak\":%.9g}\n",scenario,blocks,frames,events,hash,(double)peak);if(trace)fclose(trace);return 0;
}
