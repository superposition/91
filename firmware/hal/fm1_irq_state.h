/* SPDX-License-Identifier: GPL-3.0-only */
/* Thin PI32v2 IRQ-state binding. ICFG bit8 is the SDK global gate. Save/restore
 * every incoming bit, with compiler/device barriers; never unconditionally sti.
 * No calls are emitted while XIP is unavailable. */
#pragma once
#include <stdint.h>
#ifndef FM1_IRQ_STATE_TEST
#define FM1_IRQ_STATE_READ(v) __asm__ volatile("%0 = icfg" : "=r"(v) :: "memory")
#define FM1_IRQ_STATE_WRITE(v) __asm__ volatile("icfg = %0\n\tcsync" :: "r"(v) : "memory")
#define FM1_IRQ_STATE_FENCE() __asm__ volatile("csync" ::: "memory")
#endif
static inline __attribute__((always_inline)) uint32_t fm1_irq_state_save(void)
{
    uint32_t flags,masked;
    FM1_IRQ_STATE_READ(flags);masked=flags&~0x100u;
    FM1_IRQ_STATE_WRITE(masked);return flags;
}
static inline __attribute__((always_inline)) void fm1_irq_state_restore(uint32_t flags)
{
    FM1_IRQ_STATE_FENCE();FM1_IRQ_STATE_WRITE(flags);
}
