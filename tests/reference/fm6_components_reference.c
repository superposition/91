/* SPDX-License-Identifier: GPL-3.0-only */
/* Frozen-source component oracle. Run with the immutable R01 include path. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t a,uint32_t b,uint32_t c){(void)a;(void)b;(void)c;}
static uint32_t h;
static void bytes(const void *p,size_t n){size_t i;const uint8_t *b=p;for(i=0;i<n;i++)h=(h^b[i])*16777619u;}
#define F(s,n) bytes(&(s)->n,sizeof (s)->n)
static void eg(fm6_eg_t *e){F(e,level);F(e,target);F(e,inc);F(e,hold);F(e,ix);F(e,rising);}
static void peg(fm6_peg_t *e){F(e,pl);F(e,pt);F(e,pi);F(e,pix);F(e,prise);F(e,pr);F(e,pv);}
static void scalar(int32_t v){bytes(&v,4);}
static void init(fm6_voice_t *s,unsigned eng){unsigned i,k;memset(s,0,sizeof *s);for(k=0;k<6;k++){s->ph[k]=UINT32_MAX-31u+k*997u;s->fq[k]=170003+(int32_t)k*1193;s->g[k]=eng==0?0.25f:eng==1?12000.0f:64.0f;s->dg[k]=eng==0?-0.001953125f:eng==1?-64.0f:0.25f;}
    s->fb[0]=-.125f;s->fb[1]=.0625f;s->fbs=7;
    for(i=0;i<64;i++){fm6_sum[i]=(float)((int)i-31)*.03125f;fm6_isum[i]=((int32_t)i-31)*524288;for(k=0;k<2;k++){fm6_bus[k][i]=(float)((int)(i*7+k*3)%31-15)*.125f;fm6_ibus[k][i]=((int32_t)((i*7+k*3)%31)-15)*2097152;}}
}
static void opstate(fm6_voice_t *s){bytes(s->ph,sizeof s->ph);bytes(s->g,sizeof s->g);bytes(s->fb,sizeof s->fb);bytes(fm6_bus,sizeof fm6_bus);bytes(fm6_sum,sizeof fm6_sum);bytes(fm6_ibus,sizeof fm6_ibus);bytes(fm6_isum,sizeof fm6_isum);}
static void emit(unsigned *count){printf("%s%u",(*count)++?",":"",h);}
int main(void){unsigned eng,k,in,out,add,kind,nix,count=0,rate,level,rs,j,wave,delay;const unsigned ns[]={1,3,63,64};fm6_voice_t s;
    fm_init();printf("{\"operators\":[");
    /* Every operator, bus pair (including in==out), accumulation and path. */
    for(eng=0;eng<3;eng++)for(k=0;k<6;k++)for(in=0;in<3;in++)for(out=0;out<3;out++)for(add=0;add<2;add++)for(kind=0;kind<(eng==1?4u:2u);kind++)for(nix=0;nix<4;nix++){
        float *fo=out?fm6_bus[out-1]:fm6_sum;const float *fi=in?fm6_bus[in-1]:0;int32_t *io=out?fm6_ibus[out-1]:fm6_isum;const int32_t *ii=in?fm6_ibus[in-1]:0;
        init(&s,eng);if(kind==0)fm6_op(&s,k,fo,fi,(int)add,eng,ns[nix]);else if(kind==1)fm6_op_fb(&s,k,fo,(int)add,eng,ns[nix]);else if(kind==2)fm6_op_i(&s,k,io,ii,(int)add,ns[nix]);else fm6_op_fb_i(&s,k,io,(int)add,ns[nix]);
        h=2166136261u;opstate(&s);emit(&count);
    }
    printf("],\"loops\":[");count=0;for(kind=0;kind<2;kind++)for(k=2;k<=3;k++)for(out=0;out<3;out++)for(nix=0;nix<4;nix++){
        init(&s,1);s.loop=(uint8_t)k;if(kind)fm6_op_loop_i(&s,out?fm6_ibus[out-1]:fm6_isum,ns[nix]);else fm6_op_loop(&s,out?fm6_bus[out-1]:fm6_sum,ns[nix]);h=2166136261u;opstate(&s);emit(&count);
    }
    printf("],\"envelopes\":[");count=0;for(rate=0;rate<100;rate++)for(level=0;level<3;level++)for(rs=0;rs<3;rs++){
        fm6_eg_t e={0};uint8_t op[21]={0};unsigned q;for(q=0;q<4;q++){op[q]=(uint8_t)((rate+q*17)%100);op[4+q]=(uint8_t)(level==0?0:level==1?50:99);}fm6_eg_go(&e,op,4000,(int32_t)rs*7-7,0);h=2166136261u;eg(&e);for(j=0;j<256;j++){if(j==96)fm6_eg_go(&e,op,4000,(int32_t)rs*7-7,3);scalar(fm6_eg_step(&e,op,4000,(int32_t)rs*7-7,j<96));eg(&e);}emit(&count);
    }
    printf("],\"pitch\":[");count=0;for(rate=0;rate<100;rate++)for(level=0;level<3;level++){
        uint8_t p[155]={0};fm6_peg_t e={0};for(j=0;j<4;j++){p[FP_PR1+j]=(uint8_t)((rate+j*17)%100);p[FP_PL1+j]=(uint8_t)(level==0?0:level==1?50:99);}fm6_peg_set(&e,p);h=2166136261u;peg(&e);for(j=0;j<256;j++){if(j==96)fm6_peg_go(&e,3);scalar(fm6_peg_step(&e,j<96));peg(&e);}emit(&count);
    }
    printf("],\"lfo\":[");count=0;for(wave=0;wave<8;wave++)for(rate=0;rate<100;rate++)for(delay=0;delay<2;delay++){
        uint8_t p[155]={0};fm6_lfo_t l={0};p[FP_LFW]=(uint8_t)wave;p[FP_LFS]=(uint8_t)rate;p[FP_LFD]=(uint8_t)(delay?99:0);l.ph=UINT32_MAX-100;l.dly=UINT32_MAX-100;l.rnd=255;h=2166136261u;for(j=0;j<256;j++){fm6_lfo_step(&l,p);F(&l,ph);F(&l,dly);F(&l,val);F(&l,depth);F(&l,rnd);}emit(&count);
    }
    printf("]}\n");return 0;
}
