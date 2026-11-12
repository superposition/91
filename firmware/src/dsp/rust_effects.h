/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include "rust_fx_control.h"
#include "rust_fx_engage.h"
extern float fm1_rust_fx_setting(uint32_t,int32_t);
extern float fm1_rust_fx_duck_decay(float);
extern float fm1_rust_fx_reduction(float);
extern void fm1_rust_fx_size(int32_t,float *);
extern void fm1_rust_fx_gains(float,float,uint32_t,uint32_t,float *);
extern uint32_t fm1_rust_fx_fades(float *,uint32_t,uint32_t,int32_t *,int16_t *,uint32_t);
extern float fm1_rust_fx_hold(float *,const float *,uint32_t);
typedef struct {
    float cur,sum,release,envelope,gain,sub_min;
    uint32_t check,index,count,sub_index;
    float threshold,slope,makeup,attack,decay,ceiling,reduction_hold,peak_hold;
} fm1_fx_master_t;
_Static_assert(sizeof(fm1_fx_master_t)==72 && _Alignof(fm1_fx_master_t)==4,"Rust master scalar ABI");
extern void fm1_rust_fx_master_set(fm1_fx_master_t *,int32_t,int32_t,int32_t,int32_t);
extern void fm1_rust_fx_master_reset(fm1_fx_master_t *,float *,float *,float *,float *);
extern void fm1_rust_fx_master_block(fm1_fx_master_t *,float *,float *,float *,float *,float *,float *,uint32_t);
extern void fm1_rust_fx_mixer_gains(int32_t,int32_t,float *);
extern void fm1_rust_fx_mixer_end(float *,const float *,uint32_t,float);
extern void fm1_rust_fx_output(const float *,const float *,uint32_t,int32_t *,int16_t *,float,float,uint32_t);
typedef struct {
    int32_t tape_write,phase;
    float accumulator,low,high,previous1,previous2,current1,current2;
    int32_t plex_write;float plex_low,plex_low2,length,head2;
} fm1_fx_echo_t;
typedef struct {float target_length,head_speed,motion,modulation,lfo,lean,feedback,pan[4];uint32_t shed;} fm1_fx_echo_config_t;
_Static_assert(sizeof(fm1_fx_echo_t)==56 && sizeof(fm1_fx_echo_config_t)==48,"Rust echo ABI");
extern void fm1_rust_fx_echo(fm1_fx_echo_t *,const fm1_fx_echo_config_t *,int16_t *,int16_t *,float,float *);
typedef struct {int32_t positions[12];float gate_envelope,gate_gain,gate_hold,shimmer_feedback,shimmer_dc,shimmer_phase,bandwidth,damp_left,damp_right,sine,cosine;uint32_t shimmer_write;} fm1_fx_reverb_t;
typedef struct {int32_t kind;float damping,hold,shimmer,decay,diffusion;} fm1_fx_reverb_config_t;
_Static_assert(sizeof(fm1_fx_reverb_t)==96 && sizeof(fm1_fx_reverb_config_t)==24,"Rust reverb ABI");
extern void fm1_rust_fx_reverb(float,fm1_fx_reverb_t *,const fm1_fx_reverb_config_t *,int16_t *,float *,float *);
extern void fm1_rust_fx_reverb_block(fm1_fx_reverb_t *,const fm1_fx_reverb_config_t *,int16_t *,float *,const int16_t *,uint32_t,const int16_t *,float *,float *,uint32_t);
extern void fm1_rust_fx_reverb_reset(fm1_fx_reverb_t *,int16_t *);
typedef struct {uint32_t index;float phases[3];} fm1_fx_chorus_t;
_Static_assert(sizeof(fm1_fx_chorus_t)==16,"Rust chorus ABI");
extern void fm1_rust_fx_chorus(fm1_fx_chorus_t *,float *,float *,float *,uint32_t,int32_t,uint32_t);
typedef struct {
    float lp_s[4],hp_s[4],sw_s[2][2],sw_s2[2][2],tk_s[2][2][2],wh_s[2][2],wh_c[4],wh_env;
    uint32_t wh_cnt,mod_w;
    float tp_ph,tp_ph2,tp_d,tp_dd,fl_ph,fl_d,fl_dd,tp_lp[2],ph_z[2][4],ph_fb[2],rm_ph;
    int32_t crush_n;float crush_l,crush_r;
} fm1_fx_tone_t;
typedef struct {
    float lp_amt_e,hp_amt_e,lp_c[4],hp_c[4],sw_c[4],sw_c2[4],tk_c[2][4],sw_m,tk_m,wh_m;
    int32_t sw_type,sw_reso,tk_mix,wh_mix,wh_sens,wh_range,wh_reso;
    float tp_m,fl_m,ph_m,tp_wd,tp_fd,tp_drive,tp_idrive,tp_k,fl_inc,fl_rng,fl_fbk,fl_wet,fl_inv,ph_fbk,ph_k0,ph_k1,ph_a[2],rm_m,rm_inc,rm_wet,cr_wet,cr_lv;
    int32_t cr_hold;
} fm1_fx_tone_config_t;
_Static_assert(sizeof(fm1_fx_tone_t)==232 && sizeof(fm1_fx_tone_config_t)==240,"Rust tone ABI");
extern void fm1_rust_fx_tone(fm1_fx_tone_t *,const fm1_fx_tone_config_t *,float *,float *,float,float,float *);
typedef struct {
    uint32_t rb_w,rb_wi,bm_sm,rb_odd;float rb_hl,rb_hr;int32_t bmode,brate;
    float rp_rate,rp_gain,rp_pos;uint32_t bm_start;float bm_off,bm_speed,bm_mix,bm_tgt,sc_pos,sc_off;
    int32_t sc_rev;uint32_t sc_k;float scr_t,flx_p,gn_pos[4],gn_st[4],rv_pos,vr_rate,vr_lp[2];
} fm1_fx_sampler_t;
typedef struct {
    float rp_step,rp_gatef,rp_t,rp_pit;uint32_t rp_ref,rp_ok;float bm_len,sc_len;int32_t sc_n;
    int32_t sc_chaos,sc_reverse;float scr_w,scr_per;int32_t scr_style;uint32_t flx_live,flx_mute,flx_rev;
    float flx_off,flx_sub,gn_len,gn_region,gn_scale;int32_t gn_n;
    float rv_len,vr_tgt,vr_c,vr_dull,rl_gain,step_length;
} fm1_fx_sampler_config_t;
_Static_assert(sizeof(fm1_fx_sampler_t)==136 && sizeof(fm1_fx_sampler_config_t)==116,"Rust performance ring ABI");
extern void fm1_rust_fx_sampler(fm1_fx_sampler_t *,const fm1_fx_sampler_config_t *,int16_t *,uint32_t *,float,float,float *);
extern void fm1_rust_fx_render(fm1_fx_sampler_t *,const fm1_fx_sampler_config_t *,fm1_fx_tone_t *,const fm1_fx_tone_config_t *,int16_t *,float *,float *,uint32_t *,float *,float *,uint32_t);
typedef struct {float filter[4],feedback[2],stages[2][4];} fm1_fx_bus_t;
typedef struct {float filter[4],phase[2],filter_mix,phase_mix;int32_t mix;uint32_t kind;} fm1_fx_bus_config_t;
_Static_assert(sizeof(fm1_fx_bus_t)==56 && sizeof(fm1_fx_bus_config_t)==40,"Rust bus ABI");
extern void fm1_rust_fx_bus(fm1_fx_bus_t *,const fm1_fx_bus_config_t *,float *,float *,float *,float *,uint32_t);
typedef struct {float pan[4],lean,target,speed;uint32_t left;} fm1_fx_heads_t;
extern void fm1_rust_fx_echo_reset(fm1_fx_echo_t *,fm1_fx_heads_t *,int16_t *,int16_t *);
typedef struct {float motion,modulation[3],length,head;uint32_t playing,frozen;} fm1_fx_heads_config_t;
_Static_assert(sizeof(fm1_fx_heads_t)==32 && sizeof(fm1_fx_heads_config_t)==32,"Rust head control ABI");
extern void fm1_rust_fx_heads(fm1_fx_heads_t *,const fm1_fx_heads_config_t *,uint32_t,uint32_t *);
extern uint32_t fm1_rust_fx_echo_length(uint32_t,int32_t,float,const float *,float *);
typedef struct {int32_t kind,pre;float damping,hold,shimmer;} fm1_fx_rv_settings_t;
extern void fm1_rust_fx_reverb_set(fm1_fx_rv_settings_t *,int32_t,int32_t,int32_t,int32_t);
extern void fm1_rust_fx_decay(float *,float,uint32_t);
typedef struct {float envelope,gate;} fm1_fx_duck_t;
typedef struct {float depth,decay,noise_pump,kick_gain,step_length;int32_t step,gate;uint32_t odd,bus;} fm1_fx_duck_config_t;
typedef struct {float left,right;uint32_t count;} fm1_fx_crush_t;
extern void fm1_rust_fx_duck(fm1_fx_duck_t *,const fm1_fx_duck_config_t *,float *const *,uint32_t);
extern void fm1_rust_fx_crush(fm1_fx_crush_t *,float *,float *,const float *,const float *,uint32_t,float);
typedef struct {float b0,b1,b2,br,pw,pp,y1,y2,crack,lp,r1,r2;uint32_t rng,cnt;} fm1_fx_vinyl_gen_t;
typedef struct {fm1_fx_vinyl_gen_t generators[2];float held[2];uint32_t phase;} fm1_fx_vinyl_t;
typedef struct {float fade,level,lowpass,hiss,pop,amplitude,gain0,gain1,inverse;uint32_t playing,shed;} fm1_fx_vinyl_config_t;
_Static_assert(sizeof(fm1_fx_vinyl_t)==124 && sizeof(fm1_fx_vinyl_config_t)==44,"Rust vinyl ABI");
_Static_assert(sizeof(fm1_fx_rv_settings_t)==20 && sizeof(fm1_fx_duck_t)==8 && sizeof(fm1_fx_duck_config_t)==36 && sizeof(fm1_fx_crush_t)==12,"Rust FX scalar layout");
extern void fm1_rust_fx_vinyl(fm1_fx_vinyl_t *,const fm1_fx_vinyl_config_t *,float *,float *,uint32_t);
typedef struct {float first[3],second[3];uint32_t first_bus,second_bus;} fm1_mix_merge_t;
typedef struct {float reverb_gain,echo_gain,reverb_send,throw_,track_mix;int32_t track_send;float wash;} fm1_mix_echo_sends_t;
_Static_assert(sizeof(fm1_mix_merge_t)==32 && sizeof(fm1_mix_echo_sends_t)==28,"Rust mixer scalar ABI");
extern void fm1_rust_mix_route_plain(float *const *,uint32_t);
extern void fm1_rust_mix_route_drums(float *const *,const float *,const float *,uint32_t);
extern void fm1_rust_mix_pad_fm(const fm1_mix_merge_t *,float *const *,const float *const *,uint32_t);
extern void fm1_rust_mix_bass_acid(const fm1_mix_merge_t *,float *const *,const float *const *,uint32_t);
extern float fm1_rust_mix_reverb_gain(float,float,float,float);
extern float fm1_rust_mix_duck_depth(float,float,float);
extern void fm1_rust_mix_echo_block(fm1_fx_echo_t *,const fm1_fx_echo_config_t *,int16_t *,int16_t *,const fm1_mix_echo_sends_t *,float *const *,float *,uint32_t);
extern void fm1_rust_mix_tank(float *const *,const float *,const float *,const float *,uint32_t);
