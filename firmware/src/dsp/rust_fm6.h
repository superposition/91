/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
#include <stddef.h>
typedef struct{int32_t level,target,inc,hold;uint8_t ix,rising;} fm1_fm6_eg_t;
typedef struct{int32_t pl,pt,pi;uint8_t pix,prise,pr[4],pv[4];} fm1_fm6_peg_t;
typedef struct{
    fm1_fm6_eg_t eg[6];uint32_t ph[6];int32_t fq[6],freq_log[6];uint8_t freq_valid;
    float gout[6],g[6],dg[6];int32_t base[6],porta[6];int16_t ol[6];int8_t rs[6];uint8_t plan[6];float fb[2];fm1_fm6_peg_t pe;
    uint8_t down,sub,note,vel,eng,loop_,fbs,quiet,played,frozen,still;int32_t spb;
} fm1_fm6_voice_t;
typedef struct{fm1_fm6_voice_t s;uint8_t used,gate,part;uint32_t age;int32_t gate_left,kill_left;float fade;} fm1_fm6_slot_t;
typedef struct{uint32_t ph,dly;int32_t val,depth;uint8_t rnd;} fm1_fm6_lfo_t;
typedef struct{uint32_t jam,part;int32_t current;uint32_t shed;} fm1_fm6_note_t;
typedef struct{uint32_t jam;int32_t current;uint32_t engine,float_path;int32_t tune,bend;} fm1_fm6_config_t;
typedef struct{uint32_t amd,points[4];uint8_t valid;} fm1_fm6_ams_t;
typedef struct{float *sin;uint16_t *mki_log,*mki_exp,*opl_log,*opl_exp;} fm1_fm6_tables_t;
typedef struct{float *bus,*sum;int32_t *ibus,*isum;} fm1_fm6_scratch_t;
typedef struct{float gain,start,end,inverse;uint32_t offset;} fm1_fm6_output_t;
typedef struct{int32_t requested,current;uint32_t dy_current,dy_requested,user_count;} fm1_fm6_selection_t;
typedef struct{uint32_t part,touch;int32_t release,brightness;uint32_t attack;} fm1_fm6_profile_t;
typedef struct{uint32_t pending,count;int32_t current;} fm1_fm6_user_t;
typedef struct{fm1_fm6_slot_t *slots;fm1_fm6_lfo_t *keys,*bass;const uint8_t *key_patch,*bass_patch;const fm1_fm6_config_t *context;fm1_fm6_ams_t *cache;const fm1_fm6_tables_t *tables;const fm1_fm6_scratch_t *scratch;float *block;uint32_t *position;float *bass_sample;} fm1_fm6_render_t;
_Static_assert(sizeof(fm1_fm6_eg_t)==20 && sizeof(fm1_fm6_peg_t)==24 && sizeof(fm1_fm6_voice_t)==388,"Rust FM operator state ABI");
_Static_assert(sizeof(fm1_fm6_slot_t)==408 && sizeof(fm1_fm6_lfo_t)==20,"Rust FM pool ABI");
_Static_assert(sizeof(fm1_fm6_note_t)==16 && sizeof(fm1_fm6_config_t)==24 && sizeof(fm1_fm6_ams_t)==24,"Rust FM scalar context ABI");
_Static_assert(sizeof(fm1_fm6_selection_t)==20 && sizeof(fm1_fm6_profile_t)==20 && sizeof(fm1_fm6_user_t)==12 && sizeof(fm1_fm6_output_t)==20,"Rust FM program/output ABI");
_Static_assert(sizeof(fm1_fm6_render_t)==12*sizeof(void *) && sizeof(fm1_fm6_tables_t)==5*sizeof(void *) && sizeof(fm1_fm6_scratch_t)==4*sizeof(void *),"Rust FM pointer descriptors ABI");
extern void fm1_rust_fm6_init(const fm1_fm6_tables_t *);
extern void fm1_rust_fm6_reset(fm1_fm6_slot_t *,fm1_fm6_lfo_t *);
extern uint32_t fm1_rust_fm6_note(fm1_fm6_slot_t *,fm1_fm6_lfo_t *,fm1_fm6_lfo_t *,const uint8_t *,const fm1_fm6_note_t *,uint32_t *,int32_t,float,int32_t);
extern void fm1_rust_fm6_gates(fm1_fm6_slot_t *,uint32_t,int32_t);
extern void fm1_rust_fm6_block(fm1_fm6_slot_t *,fm1_fm6_lfo_t *,fm1_fm6_lfo_t *,const uint8_t *,const uint8_t *,const fm1_fm6_config_t *,fm1_fm6_ams_t *,const fm1_fm6_tables_t *,const fm1_fm6_scratch_t *,float *);
extern void fm1_rust_fm6_render(const fm1_fm6_render_t *,float *,float *,uint32_t);
extern uint32_t fm1_rust_fm6_pool(fm1_fm6_slot_t *,uint32_t,uint32_t);
extern float fm1_rust_fm6_output(const float *,float *const *,const fm1_fm6_output_t *,uint32_t);
extern float fm1_rust_fm6_bass_output(const float *,float *,float,uint32_t);
extern uint32_t fm1_rust_fm6_program(fm1_fm6_selection_t *,const uint8_t *,uint8_t *);
extern void fm1_rust_fm6_profile(const uint8_t *,uint8_t *,const fm1_fm6_profile_t *);
extern float fm1_rust_fm6_gain(uint32_t,uint32_t);
extern const uint8_t *fm1_rust_fm6_name(uint32_t,uint32_t);
extern uint32_t fm1_rust_fm6_user(uint32_t,uint32_t,fm1_fm6_user_t *,const uint8_t *,uint8_t *,uint8_t *);
