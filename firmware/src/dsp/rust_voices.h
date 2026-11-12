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
typedef struct {float ph[4],inc[4],sub,note,target,t,rate,peak,amp,fa,fh,fr,ic[4],c1[4],c2[4],vib,pan,hf;uint8_t midi_note,hand,voices,play,pnote;uint32_t pord;float pamp;} fm1_voice_pad_t;
typedef struct {float gain,stab;uint32_t controls[4];} fm1_voice_pad_runtime_t;
typedef struct {float attack,hold,release,rest;uint32_t playing,manual,oscillator;} fm1_voice_pad_breath_t;
typedef struct {fm1_voice_pad_breath_t breath;float gain,gate,spread,motion,modulation[6],pitch,bright,arc,filter_mod,panning,sound_cut,tuning,stab_decay,swell,breath_scale[4],vowels[4][2];uint32_t stabs,dx;int32_t filter;} fm1_voice_pad_config_t;
typedef struct {uint32_t mask;uint8_t notes[4];float amplitudes[4],level;} fm1_voice_pad_publication_t;
typedef struct {float sound_cut,bright,arc,filter_mod,link_amount,link_env,gain0,gain1,inverse;uint32_t start;} fm1_voice_pad_bus_t;
_Static_assert(sizeof(fm1_voice_pad_t)==148 && sizeof(fm1_voice_pad_runtime_t)==24,"Rust pad state ABI");
_Static_assert(sizeof(fm1_voice_pad_breath_t)==28 && sizeof(fm1_voice_pad_config_t)==164 && sizeof(fm1_voice_pad_publication_t)==28 && sizeof(fm1_voice_pad_bus_t)==40,"Rust pad call ABI");
typedef void (*fm1_voice_pad_midi_fn)(uint32_t,uint32_t,uint32_t);
extern void fm1_rust_voice_pad(fm1_voice_pad_t *,fm1_voice_pad_runtime_t *,const fm1_voice_pad_config_t *,uint32_t *,const float *,float *,float *,fm1_voice_pad_publication_t *,uint32_t,fm1_voice_pad_midi_fn);
extern void fm1_rust_voice_pad_breathe(fm1_voice_pad_t *,const fm1_voice_pad_breath_t *,float,uint32_t *,fm1_voice_pad_midi_fn);
extern void fm1_rust_voice_pad_bus(float *,const fm1_voice_pad_bus_t *,float *const *,float *,uint32_t);
