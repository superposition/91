/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_RUNNER_H
#define FM1_RUST_RUNNER_H
#include <stdint.h>
#include "rust_app.h"
typedef struct {float beat;uint8_t wall,gate,done,taken;} fm1_runner_row;
typedef struct {
    uint8_t on,over,lane,layer,ships,reserved[3];
    float start_beat,view,beat;
    uint32_t s16,s16_ms,step_ms;
    float kick_prev,kick,lp_until,flash,crash;
    uint32_t score,best,gates,crashes,rows_born;
    uint32_t banner_until,start_ms,over_ms,rng;
    char banner[20];
    fm1_runner_row row[16];
} fm1_runner_state;
typedef struct {uint32_t now,bars,playing;float kick;} fm1_runner_observation;
typedef char fm1_runner_state_size[(sizeof(fm1_runner_state)==236)?1:-1];
/* All state/input/output ranges are initialized, disjoint and foreground-owned.
 * Calls retain no pointer and dispatch no callback/platform/engine effect. */
int32_t fm1_rust_runner(uint32_t,fm1_runner_state*,const fm1_runner_observation*,uint32_t*);
int32_t fm1_rust_runner_leds(const fm1_runner_state*,const uint32_t*,const float*,uint32_t,const uint8_t*,const uint8_t*,uint32_t*);
/* pixels has exclusive capacity slots after confirmed LCD_READY; the active
 * strip is initialized before typed reads. fonts3 and theme4 are immutable. */
int32_t fm1_rust_runner_draw(const fm1_runner_state*,uint32_t,uint32_t,uint32_t,uint16_t*,uint32_t,fm1_canvas_state*,const uint16_t*,const fm1_canvas_font*);
#endif
