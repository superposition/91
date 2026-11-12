/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_PROJECT_H
#define FM1_RUST_PROJECT_H
#include <stdint.h>
typedef struct {uint16_t patterns[6][6];uint8_t modifiers[6][16];} fm1_project_patterns;
typedef struct {
    void *runtime;uint8_t *image;uint16_t *saved;
    uint8_t *pending,*pending_globals;void *globals;int32_t *from_old;
    void *extras,*live_extras;uint8_t *pending_extras;uint16_t *bass_dirty;
    int(*load)(uint32_t,void*,uint32_t);
    int(*save)(uint32_t,const void*,uint32_t);
} fm1_project_fields;
int fm1_rust_project(uint32_t,const fm1_project_fields*,uint32_t,fm1_project_patterns*,int32_t*);
int fm1_rust_project_list(const uint8_t*,uint32_t,int32_t,int32_t,uint32_t,uint32_t);
int fm1_rust_persist_ui(uint32_t,const uint8_t*,uint32_t,const int16_t*,uint32_t,uint8_t*,uint32_t);
int fm1_rust_session_save_allowed(uint32_t);
int fm1_rust_session_slot(int32_t);
int fm1_rust_session_resolve(uint8_t*,uint32_t);
typedef struct {uint8_t usb_fixed,fm_dy,scan,rounds,ramp,intro,midi_map,channel,pads,cc,visualizer,centre;} fm1_persist_performance;
typedef struct {uint16_t usb;uint8_t performance,visualizer;} fm1_persist_performance_bytes;
int fm1_rust_performance(uint32_t,const void*,void*);
int fm1_rust_performance_knob(const fm1_persist_performance*,uint32_t);
typedef struct {
    uint32_t now,last_action,place,remembered,playing,seamless,buttons,keys,waiting;
    uint32_t voices,settings,lazy,pending,globals,global_quiet,kit_quiet;
} fm1_autosave_input;
uint32_t fm1_rust_autosave(const fm1_autosave_input*,uint32_t*);
int fm1_rust_preserve_genre(uint8_t*,const uint8_t*,const uint8_t*,uint32_t,uint32_t,uint32_t);
void fm1_rust_piece_effects(const uint8_t*,const uint8_t*,const uint8_t*,uint8_t*);
#endif
