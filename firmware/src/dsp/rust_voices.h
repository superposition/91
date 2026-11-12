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
