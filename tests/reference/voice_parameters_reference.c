/* SPDX-License-Identifier: GPL-3.0-only */
/* Real selected apply/init/cm_trigger observations; immutable R01 is the oracle. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include "amb.c"
static uint32_t last_velocity;
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){if(s==0x91 && a==60)last_velocity=b;}
static uint32_t bits(float v){uint32_t w;memcpy(&w,&v,4);return w;}
static uint32_t hash=2166136261u;
static void record(const void *p,size_t n){const uint8_t *b=p;size_t i;for(i=0;i<n;i++)hash=(hash^b[i])*16777619u;}
static float observed(int id){
    if(id>=P_LANE && id<P_ECHO){int lane=(id-P_LANE)/3,kind=(id-P_LANE)%3;return kind==0?lane_g[lane]:kind==1?lane_tune[lane]:lane_dec[lane];}
    switch(id){case P_PAD:return pad_vol;case P_ACID:return acid_vol;case P_CM_LVL:return cm_vol;case P_BASS:return bass_vol;case P_PULSE:return pulse_vol;case P_NOISE:return noise_vol;case P_DETUNE:return detune;case P_DR_RES:return reso_k;case P_DRIVE:return drive_k;case P_BRIGHT:return bright;case P_MOTION:return motion;case P_NZ_PUMP:return nz_pump;default:return tunefac;}
}
int main(void){
    static const int ids[]={P_PAD,P_ACID,P_CM_LVL,P_BASS,P_PULSE,P_NOISE,P_DETUNE,P_DR_RES,P_DRIVE,P_BRIGHT,P_MOTION,P_NZ_PUMP,P_TUNE};
    static const float velocities[]={-1.0f,0.0f,0.01f,0.3f,1.0f,1.27f};
    unsigned i,j,k,first=1;int value;en_init();printf("{\"curves\":[");
    for(i=0;i<sizeof ids/sizeof ids[0]+18;i++){int id=i<sizeof ids/sizeof ids[0]?ids[i]:P_LANE+(int)(i-sizeof ids/sizeof ids[0]);const en_param_t *info=en_param_info(id);
        for(value=info->lo;value<=info->hi;value++){apply(id,value);printf("%s[%d,%d,%u]",first?"":",",id,value,bits(observed(id)));first=0;}}
    printf("],\"identity\":[");first=1;
    for(i=0;i<2;i++)for(j=0;j<11;j++)for(value=-1;value<25;value++){en_jam_on=(uint8_t)i;par[P_PAD_COMP]=(int16_t)j;par[P_PADTONE]=(int16_t)value;printf("%s[%u,%u,%d,%d,%d]",first?"":",",i,j,value,pad_stabs(),pad_osc());first=0;}
    printf("],\"hats\":[");for(i=0;i<EN_NKIT;i++){apply(P_KIT_HAT,(int)i);printf("%s[",i?",":"");for(j=0;j<4;j++)printf("%s%u",j?",":"",bits(hat_inc[j]));putchar(']');}
    printf("],\"acid_decay\":[");for(i=0;i<=100;i++){float decay=0.06f*fm_exp2f((float)i*0.05f);printf("%s%u",i?",":"",bits(decay));}
    printf("],\"notes\":[");first=1;en_jam_on=0;par[P_MIDI]=1;muted=0;midi_map=0;
    for(i=0;i<=100;i++)for(j=0;j<2;j++)for(k=0;k<sizeof velocities/sizeof velocities[0];k++){
        int gate=-1;unsigned n;fm_reset();fm_set_patch(0);memset(cm_mn,0,sizeof cm_mn);memset(cm_moff,0,sizeof cm_moff);memset(cm,0,sizeof cm);cm_alt=0;par[P_CM_DEC]=(int16_t)i;par[P_CM_SND]=(int16_t)(j?CM_NPLAIN:0);last_velocity=0;cm_trigger(60,velocities[k]);
        if(j)for(n=0;n<FM_NV;n++)if(fmx[n].used)gate=fmx[n].gate_left;
        assert(cm_mn[0]==60);assert(!j || gate>0);printf("%s[%u,%u,%u,%u,%d,%d,%u]",first?"":",",i,j,bits(velocities[k]),last_velocity,cm_moff[0],gate,j?0:bits(cm[1].dm));first=0;
    }
    printf("],\"manual\":[");first=1;
    {static const int keys[]={0,24,36,60,96,127};int octave,part;par[P_CM_SND]=0;
    for(octave=-2;octave<=2;octave+=2)for(part=-2;part<=2;part+=2)for(i=0;i<sizeof keys/sizeof keys[0];i++)for(k=0;k<sizeof velocities/sizeof velocities[0];k++){
        par[P_OCT]=(int16_t)octave;par[P_CM_OCT]=(int16_t)part;cm_alt=0;cm_play(keys[i],velocities[k]);printf("%s[%d,%d,%d,%u,%u,%u]",first?"":",",octave,part,keys[i],bits(velocities[k]),(unsigned)en_cm_note,bits(cm[1].vel));first=0;
    }}
    printf("],\"routing\":[");first=1;
    {static const float waves[]={-1.0f,-0.3f,0.0f,0.6f,1.0f};unsigned pd,dd,depth,w;fm_reset();step_len=2000.0f;par[P_PL_SHAPE]=par[P_DL_SHAPE]=4;par[P_PL_RATE]=par[P_DL_RATE]=0;wob_rate=-1;
    for(pd=0;pd<8;pd++)for(dd=0;dd<8;dd++)for(depth=0;depth<=100;depth+=50)for(w=0;w<5;w++){
        par[P_PL_DEST]=(int16_t)pd;par[P_DL_DEST]=(int16_t)dd;par[P_PL_DEPTH]=(int16_t)depth;par[P_DL_DEPTH]=(int16_t)(100-depth);pad_lfo.ph=drone_lfo.ph=0.1f;pad_lfo.b=waves[w];drone_lfo.b=waves[4-w];control(32);
        printf("%s[%u,%u,%u,%u,%u,%u,%u,%u,%u,%u,%u,%u,%u,%u]",first?"":",",pd,dd,depth,100-depth,bits(pad_lfo.v),bits(drone_lfo.v),bits(pad_filt),bits(pad_lvl),bits(pad_pan),bits(pad_pitch),bits(lfo_acut),bits(lfo_ares),bits(lfo_echo),bits(lfo_tcut));first=0;
    }}
    printf("],\"links\":[");first=1;
    {static const float velocities2[]={0.0f,0.5f,1.0f};unsigned on,amount,lane,v;for(on=0;on<2;on++)for(amount=0;amount<=100;amount+=100)for(lane=0;lane<6;lane++)for(v=0;v<3;v++){
        float hitk,hits;par[P_LINKS]=(int16_t)on;par[P_LK_KICK]=par[P_LK_SNARE]=(int16_t)amount;lk_kick=0.2f;lk_snare=0.3f;drum_fire((int)lane,velocities2[v]);hitk=lk_kick;hits=lk_snare;control(32);
        printf("%s[%u,%u,%u,%u,%u,%u,%u,%u]",first?"":",",on,amount,lane,bits(velocities2[v]),bits(hitk),bits(hits),bits(lk_kick),bits(lk_snare));first=0;
    }}
    printf("],\"reinit_hashes\":[");
    for(i=0;i<8;i++){
        memset(pad,0x11+i,sizeof pad);memset(mod,0x11+i,sizeof mod);for(j=0;j<4;j++)pad_ctl[j]=17+j;dr_ctl=ac_ctl=31;rng=0x9e3779b9u+i;prng=0x2545f491u+i;en_init();hash=2166136261u;
        record(pad,sizeof pad);record(mod,sizeof mod);record(&bass,sizeof bass);record(&acid,sizeof acid);record(drum,sizeof drum);record(&rng,4);record(&prng,4);record(pq_st,16);record(pq_d1,16);record(pq_d2,16);record(&pq_n,1);record(pad_ctl,sizeof pad_ctl);record(&dr_ctl,4);record(&ac_ctl,4);{const float metric[]={pad_gain,stab_env};record(metric,sizeof metric);}printf("%s%u",i?",":"",hash);
    }
    puts("]}");return 0;
}
