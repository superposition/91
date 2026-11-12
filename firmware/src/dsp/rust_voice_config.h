/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
typedef struct {uint32_t playing;uint32_t played;uint32_t line_slide;uint32_t bent;int32_t filter;int32_t squash_target;int32_t squash_drive;int32_t squash_amount;float volume;float resonance;float envelope;float velocity;float lfo;float modulation;float arc;float motion;float section;float duck;float pulse;float link;float bend;float tuning;float gain0;float gain1;float inverse;} fm1_voice_droneinput_t;
typedef struct {int32_t filter;int32_t squash_target;int32_t squash_drive;int32_t squash_amount;int32_t squash_wow;uint32_t sliding;uint32_t bent;float volume;float cutoff;float resonance;float accent;float bend;float gain0;float gain1;float inverse;} fm1_voice_acidinput_t;
typedef struct {uint32_t playing;uint32_t jam;uint32_t dx;uint32_t genre;int32_t filter;float volume;float level;float gate;float detune;float motion;float modulation[6];float pitch;float bright;float arc;float filter_mod;float panning;float tuning;float stab_decay;float swell;float section;float link;} fm1_voice_padinput_t;
typedef struct {float bright;float arc;float filter_mod;float link;float gain0;float gain1;float inverse;uint32_t start;} fm1_voice_padbusinput_t;
typedef struct {float volume;float drive;float gains[6];float hat_inc[4];float gain0[4];float gain1[4];float inverse;} fm1_voice_drumsinput_t;
typedef struct {float volume;float bend;float gain0;float gain1;float inverse;float old_level;uint32_t bent;} fm1_voice_pluckinput_t;
extern void fm1_rust_voice_drone_config(const int16_t *,const fm1_voice_droneinput_t *,fm1_voice_drone_config_t *);
extern void fm1_rust_voice_acid_config(const int16_t *,const fm1_voice_acidinput_t *,fm1_voice_acid_config_t *);
extern void fm1_rust_voice_pad_config(const int16_t *,const fm1_voice_padinput_t *,fm1_voice_pad_config_t *);
extern void fm1_rust_voice_pad_bus_config(const int16_t *,const fm1_voice_padbusinput_t *,fm1_voice_pad_bus_t *);
extern void fm1_rust_voice_pluck_config(const int16_t *,const fm1_voice_pluckinput_t *,fm1_voice_pluck_config_t *);
extern void fm1_rust_voice_breath_config(const int16_t *,uint32_t,fm1_voice_pad_breath_t *);
extern void fm1_rust_voice_drums_config(const fm1_voice_drumsinput_t *,const fm1_voice_data_kit_t *const *,fm1_voice_drums_config_t *);
_Static_assert(sizeof(fm1_voice_droneinput_t)==100 && sizeof(fm1_voice_acidinput_t)==60 && sizeof(fm1_voice_padinput_t)==104 && sizeof(fm1_voice_padbusinput_t)==32 && sizeof(fm1_voice_drumsinput_t)==84 && sizeof(fm1_voice_pluckinput_t)==28,"Voice scalar capture ABI");
