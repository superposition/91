/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
typedef struct {
    void (*boot)(uint32_t,uint32_t);
    void (*service)(void);
    void (*render)(int32_t*,uint32_t,uint32_t);
    void (*frame)(void);
    void (*input)(void);
    double (*nanoseconds)(void);
} fm1_web_callbacks;
extern uint32_t fm1_rust_web_get(uint32_t);
extern void fm1_rust_web_set(uint32_t,uint32_t,uint32_t);
extern void fm1_rust_web_boot(const fm1_web_callbacks*);
extern void fm1_rust_web_advance(uint32_t,const fm1_web_callbacks*);
extern uint16_t *fm1_rust_web_frame(void);
extern void fm1_rust_web_fill(uint32_t,uint32_t,uint32_t,uint32_t,uint16_t);
extern void fm1_rust_web_blit(uint32_t,uint32_t,uint32_t,uint32_t,const uint16_t*);
extern uint32_t fm1_rust_web_host_value(uint32_t,uint32_t);
extern int32_t fm1_rust_web_midi_in(uint32_t*);
extern void fm1_rust_web_midi_out(uint32_t);
extern uint8_t *fm1_rust_web_store_ptr(uint32_t);
extern int32_t fm1_rust_web_store_len(uint32_t);
extern void fm1_rust_web_store_set_len(uint32_t,int32_t);
extern int32_t fm1_rust_web_store_load(uint32_t,void*,uint32_t);
extern int32_t fm1_rust_web_store_save(uint32_t,const void*,uint32_t,uint32_t,uint32_t);
extern uint32_t fm1_rust_web_store_writes(void);
