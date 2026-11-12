/* SPDX-License-Identifier: GPL-3.0-only */
/* PI32v2 atomic ABI. Every operation is SC; IRQ state is restored exactly.
 * Hardware testset and ICFG bit8 follow the AC79 SDK asm/cpu.h and asm/irq.h.
 * This backend is enabled only for the experimental Rust MCU image. */
#pragma once
#include <stdint.h>
#if EN_RUST_MCU_ATOMICS
static volatile uint8_t fm1_rust_atomic_lock __attribute__((aligned(32)));
static uint32_t fm1_rust_atomic_recoveries;
static inline uint32_t fm1_rust_atomic_try(void)
{
    uint32_t acquired;
    __asm__ volatile("testset b[%1]\n\tifeq goto 1f\n\t%0 = 1\n\tgoto 2f\n1:\n\t%0 = 0\n2:\n\tcsync"
                     : "=&r"(acquired) : "r"(&fm1_rust_atomic_lock) : "memory");
    return acquired;
}
static void fm1_rust_atomic_stopped_owner(void)
{
#if EN_CORE2
    if(!fm1_cnum() && fm1_core1_online()) {
        /* No byte lock is cleared while its other possible owner can run.
         * Preserve the outstanding request so the later join reports loss;
         * setting aw.complete here would falsely report successful output. */
        fm1_core1_stop();
        fm1_core1_bar_set();
        fm1_core1_fault.count++;
        fm1_rust_atomic_recoveries++;
        __asm__ volatile("csync" ::: "memory");
        fm1_rust_atomic_lock=0;
        __asm__ volatile("csync" ::: "memory");
        return;
    }
#endif
    /* A stuck CPU0 owner cannot be recovered by CPU1. Park without acquiring
     * anything: CPU0's bounded join holds CPU1, or the watchdog resets CPU0.
     * Same-core reentry is also fail-stop rather than breaking exclusion. */
    for(;;) __asm__ volatile("nop" ::: "memory");
}
void fm1_mcu_fence(void) { __asm__ volatile("csync" ::: "memory"); }
uint32_t fm1_mcu_atomic(uint32_t op,uint32_t *word,uint32_t expected,uint32_t value)
{
    uint32_t flags,masked,old,spins=0;
    __asm__ volatile("%0 = icfg" : "=r"(flags) :: "memory");
    masked=flags & ~0x100u;
    __asm__ volatile("icfg = %0\n\tcsync" :: "r"(masked) : "memory");
    while(!fm1_rust_atomic_try()) {
        if(++spins==65536u) {fm1_rust_atomic_stopped_owner();spins=0;}
    }
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
