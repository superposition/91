/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Stateless, before .data/.bss init. Stack-local copied records only. */
extern uint32_t fm1_rust_hal_boot(uint32_t *,uint32_t,uint32_t);
extern uint32_t fm1_rust_hal_readable(uint32_t,uint32_t);
extern uint32_t fm1_rust_hal_flash(uint32_t,uint32_t,uint32_t);
/* Exact-width MMIO; caller retains exclusion and external DMA lifetimes. */
extern uint32_t fm1_rust_hal_io(uint32_t,uint32_t,uint32_t,uint32_t,uint32_t);
extern uint32_t fm1_rust_hal_audio_config(int32_t *,uint32_t,uint32_t);
extern uint32_t fm1_rust_hal_audio_begin(uint32_t);
extern uint32_t fm1_rust_hal_audio_end(uint32_t);
extern uint32_t fm1_rust_hal_audio_reusable(void);
extern uint32_t fm1_rust_hal_audio_pause(void);
extern uint32_t fm1_rust_hal_audio_restart(void);
extern void fm1_hal_audio_stop_fence(void);
extern uint32_t fm1_hal_audio_stop_confirm(void);
extern uint32_t fm1_rust_hal_audio_rendering(void);
extern void fm1_rust_hal_audio_silence(int32_t *,uint32_t);
#ifdef __cplusplus
}
#endif
