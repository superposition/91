/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
typedef struct {
    uint32_t mask,jam_mask,lane,sixteenth,bars,length,condition,time,slice;
    int32_t section,section_bars,section_length,section_request;
    uint32_t groove_bars,section_flags,keep,genre,fill_hold,fill_bar,drift_fill,fill_kind;
    uint32_t drop_impact,jam,modification,ghost,probability,accent,human,fx,energy;
    float step_length;int32_t swing;float hat_level,chop;
} fm1_lane_t;
typedef struct {uint32_t kind;int32_t wait;float velocity,multiplier;} fm1_hit_action_t;
typedef struct {
    uint32_t step,hit,kick,crash,hat,perc,count,reserved;
    fm1_hit_action_t actions[3];
} fm1_hit_t;
_Static_assert(sizeof(fm1_lane_t)==136 && sizeof(fm1_hit_t)==80,"Rust lane/hit ABI");
extern uint32_t fm1_rust_seq_density(uint32_t,int32_t);
extern int32_t fm1_rust_seq_swing(uint32_t,float,float);
extern int32_t fm1_rust_seq_loose(int32_t,uint32_t,float,uint32_t *);
extern void fm1_rust_seq_lane(const fm1_lane_t *,uint32_t *,fm1_hit_t *);
typedef struct {int32_t countdown;uint32_t bars,fill;int32_t left,next;uint32_t changed,mutation;} fm1_drift_t;
_Static_assert(sizeof(fm1_drift_t)==28 && _Alignof(fm1_drift_t)==4,"Rust drift ABI");
extern int32_t fm1_rust_seq_earliest(const int32_t *);
extern uint32_t fm1_rust_seq_wait(int32_t *,int32_t);
extern uint32_t fm1_rust_seq_chain(uint32_t,int32_t,float,float,int32_t *,float *);
extern uint32_t fm1_rust_seq_song_pool(const uint16_t *);
extern uint32_t fm1_rust_seq_drift_length(uint32_t,uint32_t,uint32_t);
extern void fm1_rust_seq_mutate(uint16_t *,uint32_t,uint32_t *);
extern void fm1_rust_seq_drift(fm1_drift_t *,uint32_t,uint32_t,uint32_t,uint32_t,int32_t,uint32_t *);
extern uint32_t fm1_rust_seq_boundary(uint32_t,uint32_t,uint32_t,uint32_t,uint32_t);
extern uint32_t fm1_rust_seq_time(uint32_t,uint32_t,int32_t,uint32_t,int32_t);
extern uint32_t fm1_rust_seq_fill(uint32_t,uint32_t,uint32_t);
extern uint32_t fm1_rust_seq_pattern_chain(uint32_t,uint32_t *);
extern uint32_t fm1_rust_seq_slice(uint32_t,uint32_t,uint32_t *);
extern uint32_t fm1_rust_seq_section(uint32_t,int32_t,int32_t,int32_t,float,uint32_t *);
