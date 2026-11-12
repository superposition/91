/* SPDX-License-Identifier: GPL-3.0-only */
/* PI32v2 atomic ABI. Every operation is SC; IRQ state is restored exactly.
 * Hardware testset and ICFG bit8 follow the AC79 SDK asm/cpu.h and asm/irq.h.
 * This backend is enabled only for the experimental Rust MCU image. */
#pragma once
#include <stdint.h>
#if EN_RUST_MCU_ATOMICS
static volatile uint8_t fm1_rust_atomic_lock __attribute__((aligned(32)));
void fm1_mcu_fence(void) { __asm__ volatile("csync" ::: "memory"); }
uint32_t fm1_mcu_atomic(uint32_t op,uint32_t *word,uint32_t expected,uint32_t value)
{
    uint32_t flags,masked,old;
    __asm__ volatile("%0 = icfg" : "=r"(flags) :: "memory");
    masked=flags & ~0x100u;
    __asm__ volatile("icfg = %0\n\tcsync" :: "r"(masked) : "memory");
    __asm__ volatile("1:\n\ttestset b[%0]\n\tifeq goto 1b\n\tcsync" :: "r"(&fm1_rust_atomic_lock) : "memory");
    old=*word;
    switch(op) {
    case 0:break;
    case 1:*word=value;break;
    case 2:*word=old+value;break;
    case 3:*word=old-value;break;
    case 4:if(value>old)*word=value;break;
    case 5:if(old==expected)*word=value;break;
    case 6:*word=value;break;
    case 7:*word=old|value;break;
    case 8:*word=old&value;break;
    case 9:*word=old^value;break;
    }
    __asm__ volatile("csync" ::: "memory");
    fm1_rust_atomic_lock=0;
    __asm__ volatile("csync\n\ticfg = %0\n\tcsync" :: "r"(flags) : "memory");
    return old;
}
#endif
