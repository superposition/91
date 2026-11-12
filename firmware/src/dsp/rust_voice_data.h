/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
typedef struct {float k_start;float k_drop;float k_dec0;float k_dec1;float k_click;float k_drive;float k_rumble;float sn_f1;float sn_f2;float sn_noise;float sn_body;float sn_dec;float cl_gap;float cl_dec;float h_metal;float h_noise;float h_dec;float oh_dec;float p_freq;float p_ratio;float p_idx;float duck;float chop;float crush;} fm1_voice_data_kit_t;
extern const fm1_voice_data_kit_t FM1_VOICE_KITS[14];
_Static_assert(sizeof(fm1_voice_data_kit_t)==96,"Voice data ABI");
typedef struct {uint8_t osc;uint8_t stabs;uint8_t padding[2];float fc;float g;} fm1_voice_data_padsound_t;
extern const fm1_voice_data_padsound_t FM1_VOICE_PAD_SOUND[24];
_Static_assert(sizeof(fm1_voice_data_padsound_t)==12,"Voice data ABI");
typedef struct {uint8_t osc;uint8_t padding[3];float saw;float sub;float drive;float cut;float g;} fm1_voice_data_dronesound_t;
extern const fm1_voice_data_dronesound_t FM1_VOICE_DRONE_SOUND[24];
_Static_assert(sizeof(fm1_voice_data_dronesound_t)==24,"Voice data ABI");
typedef struct {uint8_t osc;uint8_t padding[3];float drive;float g;} fm1_voice_data_acidsound_t;
extern const fm1_voice_data_acidsound_t FM1_VOICE_ACID_SOUND[31];
_Static_assert(sizeof(fm1_voice_data_acidsound_t)==12,"Voice data ABI");
typedef struct {float fr;float i0;float i1;uint8_t rise;uint8_t padding[3];} fm1_voice_data_acidfm_t;
extern const fm1_voice_data_acidfm_t FM1_VOICE_ACID_FM[31];
_Static_assert(sizeof(fm1_voice_data_acidfm_t)==16,"Voice data ABI");
typedef struct {float fr;float i0;float i1;uint8_t rise;uint8_t padding[3];float g;} fm1_voice_data_plucksound_t;
extern const fm1_voice_data_plucksound_t FM1_VOICE_CM_SOUND[12];
_Static_assert(sizeof(fm1_voice_data_plucksound_t)==20,"Voice data ABI");
extern const float FM1_VOICE_DRONE_FR[24];
extern const float FM1_VOICE_PAD_PAN[4];
extern const float FM1_VOICE_BREATH[4];
extern const float FM1_VOICE_START_T[4];
extern const float FM1_VOICE_VOWEL[4][2];
extern const float FM1_VOICE_HAT_INC[4];
extern const float FM1_VOICE_LFO_RATE_BEATS[12];
extern const uint8_t FM1_VOICE_DRUM_GM[6];
extern const uint8_t FM1_VOICE_EN_TRIM[19][10];
extern const float FM1_VOICE_SINT[1025];
