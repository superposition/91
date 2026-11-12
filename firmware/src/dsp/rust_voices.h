/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
typedef struct {float ph,inc,inc_end,amp,adec,idx,x1,x2,att,mph[4],pend,t,hp,ph2,inc2;int32_t wait,wait2;float pend2;int32_t rgap;float rmul,xl,fo;uint8_t rn,on,note;} fm1_voice_drum_t;
typedef struct {float gains[6],hat_inc[4],drive,drop,click,rumble,sn_body,sn_noise,cl_gap,h_noise,gains0[4],gains1[4],inverse;} fm1_voice_drums_config_t;
typedef struct {float ph,ps,inc,env,dm,vel;uint32_t age;} fm1_voice_pluck_t;
typedef struct {float gain,pitch,ratio,index0,index1,gain0,gain1,inverse,old_level;uint32_t rise,fm;} fm1_voice_pluck_config_t;
_Static_assert(sizeof(fm1_voice_drum_t)==104 && sizeof(fm1_voice_drums_config_t)==108,"Rust percussion ABI");
_Static_assert(sizeof(fm1_voice_pluck_t)==28 && sizeof(fm1_voice_pluck_config_t)==44,"Rust pluck ABI");
extern void fm1_rust_voice_drums(fm1_voice_drum_t *,const fm1_voice_drums_config_t *,const float *,uint32_t *,float *const *,uint32_t);
extern float fm1_rust_voice_pluck(fm1_voice_pluck_t *,const fm1_voice_pluck_config_t *,const float *,float *const *,uint32_t);
typedef struct {float ph[3],sph,mph,note,target,amp,s[4],c1[4],c2[4],env,gtarget;uint8_t midi_note,rest,env_up,play;float vd;} fm1_voice_drone_t;
typedef struct {float env,gate;uint8_t up;} fm1_voice_line_t;
typedef struct {uint32_t oscillator;int32_t filter;uint32_t line,playing;float drive,k1,k2,input,gain,saw,sub,target,rate,fm_index,pulse_width,glide,pitch,bend,tuning,fm_ratio,cutoff,envelope,motion,arc,break_pos,break_neg,duck,links,gain0,gain1,inverse;} fm1_voice_drone_config_t;
typedef struct {float ph,inc,inc_t,amp,amp_t,env,edec,acc,s[4],c1[4],c2[4];int32_t gate_left;uint8_t note,sliding,play;float ps,p3,p4,p5,lph;} fm1_voice_acid_t;
typedef struct {uint32_t oscillator;int32_t filter;float gain,drive,resonance,envelope,accent,base,k1,k2,input,wow,velocity,glide,bend,gain0,gain1,inverse,fm_ratio,fm_index0,fm_index1;uint32_t fm_rise;} fm1_voice_acid_config_t;
_Static_assert(sizeof(fm1_voice_drone_t)==96 && sizeof(fm1_voice_line_t)==12 && sizeof(fm1_voice_drone_config_t)==124,"Rust drone ABI");
_Static_assert(sizeof(fm1_voice_acid_t)==108 && sizeof(fm1_voice_acid_config_t)==88,"Rust acid ABI");
extern float fm1_rust_voice_drone(fm1_voice_drone_t *,const fm1_voice_drone_config_t *,fm1_voice_line_t *,uint32_t *,const float *,float *,uint32_t);
extern float fm1_rust_voice_acid(fm1_voice_acid_t *,const fm1_voice_acid_config_t *,uint32_t *,const float *,float *,uint32_t);
