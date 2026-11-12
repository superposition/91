/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include "rust_harmony_abi.h"
#include <string.h>
extern int32_t fm1_rust_harm_scale(uint32_t,int32_t *);
extern int32_t fm1_rust_harm_chord(uint32_t,int32_t,int32_t,int32_t,int32_t *);
extern int32_t fm1_rust_harm_voice(const int32_t *,const int32_t *,int32_t,int32_t *,int32_t,int32_t);
extern float fm1_rust_harm_time(int32_t);
extern int32_t fm1_rust_harm_bars(uint32_t,uint32_t,int32_t);
extern int32_t fm1_rust_harm_meter_bars(uint32_t,uint32_t,int32_t,uint32_t,uint32_t);
extern uint32_t fm1_rust_harm_step(fm1_harm_state_t *,const fm1_harm_config_t *,uint32_t *,uint32_t,uint32_t,uint32_t,uint32_t);
extern int32_t fm1_rust_harm_targets(fm1_harm_state_t *,const fm1_harm_config_t *,const float *,fm1_harm_targets_t *);
extern int32_t fm1_rust_harm_preview(const fm1_harm_state_t *,const fm1_harm_config_t *,int32_t,int32_t,int32_t *);
extern int32_t fm1_rust_harm_capture(const fm1_harm_state_t *,const fm1_harm_config_t *,uint8_t *);
extern void fm1_rust_harm_mood(fm1_harm_mood_t *,uint32_t,uint32_t,float,int32_t,uint32_t *);
_Static_assert(EN_NPAD==4&&EN_NGENRE==19&&EN_PRG_N==8,"Rust harmonic capacities");
extern int32_t fm1_rust_harm_bing(int32_t,uint32_t,fm1_harm_bing_t *);
extern int32_t fm1_rust_harm_bass(uint32_t,int32_t,int32_t);
