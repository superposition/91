/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_UI_CONTROLLER_H
#define FM1_RUST_UI_CONTROLLER_H
#include "rust_ui_state.h"
typedef struct {uint32_t drift_seen,calm_at,step_at,last_xruns,last_late,window_at,window_count;} fm1_ui_frame_state;
typedef char fm1_ui_frame_words_stay_28[sizeof(fm1_ui_frame_state)==28?1:-1];
enum {UIQ_NOW=0x100,UIQ_BUTTONS,UIQ_KEYS,UIQ_SPLASH_DURATION,UIQ_THEME,UIQ_PLAYING,UIQ_DRIFT_CHANGES,UIQ_NEXT_PATTERN,UIQ_JAM_ON,UIQ_XRUNS,UIQ_LATE,UIQ_SHED_LEVEL,UIQ_GAME_ON};
enum {UIA_SPLASH,UIA_CLEAR_SCREEN,UIA_PLAY,UIA_THEME,UIA_STORE,UIA_DRIFT,UIA_REMOTE,UIA_JAM_SYNC,UIA_INPUT,UIA_SCENE,UIA_DJ,UIA_AUTO_FX,UIA_MIX,UIA_SAMPLE,UIA_KEY,UIA_GLIDE,UIA_SAVER,UIA_JAM,UIA_SHED,UIA_MANUAL_VOICE,UIA_AUTOSAVE,UIA_GAME_UPDATE,UIA_GAME_DRAW,UIA_GAME_LEDS,UIA_SCAN,UIA_JAM_SNAPSHOT,UIA_HEADER,UIA_MAIN,UIA_KNOBS,UIA_LEDS};
/* Synchronous callbacks run only between exclusive UI/aux borrows. No retained
 * callback data or pointer. Foreground serialized; IRQ never accesses carriers. */
int32_t fm1_rust_ui_frame(fm1_ui_state*,fm1_ui_frame_state*,uint32_t(*)(uint32_t,uint32_t,uint32_t),uint32_t);
#endif
