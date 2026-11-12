/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#define FM1_IRQ_STATE_TEST 1
static uint32_t icfg,reads,writes,fences;
#define FM1_IRQ_STATE_READ(v) ((v)=icfg,reads++)
#define FM1_IRQ_STATE_WRITE(v) (icfg=(v),writes++)
#define FM1_IRQ_STATE_FENCE() (fences++)
#include "fm1_irq_state.h"
int main(void){
    for(uint32_t i=0;i<65536;i++){
        const uint32_t incoming=(i*1664525u)^0x83a40000u;
        icfg=incoming;reads=writes=fences=0;
        const uint32_t outer=fm1_irq_state_save();
        assert(outer==incoming&&icfg==(incoming&~0x100u));
        const uint32_t inner=fm1_irq_state_save();
        assert(inner==(incoming&~0x100u)&&icfg==inner);
        fm1_irq_state_restore(inner);assert(icfg==(incoming&~0x100u));
        fm1_irq_state_restore(outer);assert(icfg==incoming);
        assert(reads==2&&writes==4&&fences==2);
    }
    puts("65536 exact incoming IRQ words: nested save/restore never enables a masked caller");return 0;
}
