/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
/* Included after core1, flash IRQ bindings and plat.h. No wait, lock acquisition
 * or worker buffer access is permitted in this IRQ-masked admission path. */
#if EN_RUST_HAL
extern int en_flash_begin(int confirmed_stopped);
extern void en_flash_end(void);
extern int en_flash_window_ready(int confirmed_stopped);
#ifndef FM1_FLASH_CORE1_STOPPED
/* A backend may provide a positive physical hold/bus-quiescence fact, including
 * before engine init and in Safe Mode. aw_online=0/safe_mode alone proves none. */
#define FM1_FLASH_CORE1_STOPPED() 0
#endif
#ifndef FM1_FLASH_BACKEND_IDLE
static int fm1_flash_backend_idle(void)
{
    if(FM1_FLASH_CORE1_STOPPED())return 1;
#if EN_CORE2
    /* aw.complete is published in .dsp_text only AFTER the XIP callback returns.
     * Rust stage/completed flags are deliberately insufficient. Offline/fault
     * paths require the distinct confirmed physical stop fact above. */
    return aw_online && !fm1_core1_fault.count && !aw_load(&aw.rejected) &&
           aw_load(&aw.ready) && aw_load(&aw.request)==aw_load(&aw.complete);
#else
    return 0;
#endif
}
#define FM1_FLASH_BACKEND_IDLE() fm1_flash_backend_idle()
#endif
int plat_store_begin(void)
{
    const uint32_t flags=irq_save();
    const int ok=FM1_FLASH_BACKEND_IDLE() && en_flash_begin(FM1_FLASH_CORE1_STOPPED());
    irq_restore(flags);
    return ok ? 0 : PLAT_STORE_BUSY;
}
void plat_store_end(void)
{
    const uint32_t flags=irq_save();
    en_flash_end();
    irq_restore(flags);
}
/* The caller already holds exact IRQ exclusion; admission itself remains held
 * across the transaction's short windows and interrupt-enabled erase gaps. */
static int fm1_flash_window_ready(void)
{
    return FM1_FLASH_BACKEND_IDLE() && en_flash_window_ready(FM1_FLASH_CORE1_STOPPED());
}
#endif
