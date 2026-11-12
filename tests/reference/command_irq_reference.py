#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise the real post/guard and Rust queue with a deferred CPU0 IRQ model.

The register substitutions test transaction scope, not PI32 instruction timing.
"""
from pathlib import Path
import sys

repo=Path(__file__).resolve().parents[2]
def function(source,signature):
    start=source.index(signature);brace=source.index('{',start);end=brace+1;depth=1
    while depth:
        depth+=(source[end]=='{')-(source[end]=='}');end+=1
    return source[start:end]
atomic=(repo/'firmware/hal/fm1_rust_atomic.h').read_text()
engine=(repo/'firmware/src/dsp/amb.c').read_text()
guards='\n'.join(function(atomic,s)for s in ['uint32_t fm1_mcu_command_enter(', 'void fm1_mcu_command_exit('])
post=function(engine,'static void post(')
source=r'''
#include <assert.h>
#include <setjmp.h>
#include <stdint.h>
#include <stdio.h>
#define EN_RUST_MCU_ATOMICS 1
extern void fm1_rust_commands_reset(void);
extern int32_t fm1_rust_commands_push(uint32_t);
extern int32_t fm1_rust_commands_pop(uint32_t *);
extern uint32_t fm1_rust_jam_stat(uint32_t);
static void post(uint32_t,int,int);
static uint32_t icfg=0x02050100u,core,enters,exits,observed;
static int armed,fired,pending,in_isr,parked;
static jmp_buf park_target;
static uint32_t fm1_cnum(void){return core;}
static __attribute__((noreturn)) void fm1_mcu_park(void){parked=1;longjmp(park_target,1);}
static uint32_t fm1_rust_irq_mask(void){uint32_t f=icfg;icfg&=~0x100u;enters++;return f;}
static void fm1_rust_irq_restore(uint32_t f){
    icfg=f;exits++;
    if(pending&&!in_isr&&(icfg&0x100u)){pending=0;in_isr=1;post(12,2,0x4004);in_isr=0;}
}
uint32_t fm1_mcu_atomic(uint32_t op,uint32_t *word,uint32_t expected,uint32_t value){
    uint32_t old=*word;
    assert(((uintptr_t)word&3u)==0);
    if(armed){assert(!(icfg&0x100u));observed++;if(op==0&&!fired){fired=1;pending=1;}}
    switch(op){case 0:break;case 1:*word=value;break;case 2:*word=old+value;break;
    case 3:*word=old-value;break;case 4:if(value>old)*word=value;break;
    case 5:if(old==expected)*word=value;break;case 6:*word=value;break;
    case 7:*word=old|value;break;case 8:*word=old&value;break;case 9:*word=old^value;break;
    default:assert(0);}
    return old;
}
void fm1_mcu_fence(void){__asm__ volatile("" ::: "memory");}
@GUARDS@
@POST@
int main(void){
    uint32_t word=0;
    fm1_rust_commands_reset();
    armed=1;post(12,1,0x6342);armed=0;
    assert(fired&&!pending&&!in_isr&&observed>8&&enters==2&&exits==2);
    assert(icfg==0x02050100u);
    assert(fm1_rust_commands_pop(&word)==1&&word==0x0c016342u);
    assert(fm1_rust_commands_pop(&word)==1&&word==0x0c024004u);
    assert(fm1_rust_commands_pop(&word)==0);
    assert(fm1_rust_jam_stat(6)==2&&fm1_rust_jam_stat(7)==2);
    assert(fm1_rust_jam_stat(8)==0&&fm1_rust_jam_stat(9)==2);
    /* A caller that already masked IRQs must remain masked. The deferred
     * render publication runs only when that outer owner enables them. */
    fm1_rust_commands_reset();enters=exits=observed=0;fired=pending=0;
    icfg=0x3a550000u;armed=1;post(12,1,0x6342);armed=0;
    assert(icfg==0x3a550000u&&pending&&enters==1&&exits==1);
    assert(fm1_rust_commands_pop(&word)==1&&word==0x0c016342u);
    assert(fm1_rust_commands_pop(&word)==0);
    armed=1;fm1_rust_irq_restore(0x3a550100u);armed=0;
    assert(!pending&&icfg==0x3a550100u);
    assert(fm1_rust_commands_pop(&word)==1&&word==0x0c024004u);
    assert(fm1_rust_commands_pop(&word)==0);
    icfg=0x02050100u;
    fm1_rust_commands_reset();core=1;
    if(!setjmp(park_target)){post(12,3,0x1234);assert(0);}
    core=0;assert(parked&&icfg==0x02050100u);
    assert(fm1_rust_commands_pop(&word)==0&&fm1_rust_jam_stat(6)==0);
    puts("Real MCU post guard defers nested render publication; both commands survive in order, ICFG restores exactly, CPU1 cannot publish.");
    return 0;
}
'''.replace('@GUARDS@',guards).replace('@POST@',post)
Path(sys.argv[1]).write_text(source)
