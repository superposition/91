#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise actual HAL control functions with hosted register/time substitutes.

This checks retirement/publication decisions, not target instruction semantics.
"""
from pathlib import Path
import sys
repo=Path(__file__).resolve().parents[2]
core=(repo/'firmware/hal/fm1_core1.h').read_text()
atomic=(repo/'firmware/hal/fm1_rust_atomic.h').read_text()
def function(source,signature):
    start=source.index(signature)
    brace=source.index('{',start)
    depth=1;end=brace+1
    while depth:
        depth+=(source[end]=='{')-(source[end]=='}');end+=1
    return source[start:end]
functions='\n'.join(function(core,s) for s in ['static uint32_t fm1_core1_load(', 'static int fm1_core1_submit(', 'static void fm1_core1_retire(', 'static int fm1_core1_join('])
recovery=function(atomic,'static void fm1_rust_atomic_stopped_owner(')
recovery=recovery.replace('__asm__ volatile("csync" ::: "memory");','barriers++;')
source=r'''
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define EN_CORE2 1
#define AW_TIMEOUT_TICKS 10u
#define AW_CHECK(fn,ctx,req) ((uint32_t)(uintptr_t)(fn)^(uint32_t)(uintptr_t)(ctx)^(req)*0x9e3779b1u)
typedef void (*fm1_core1_fn)(void *);
static struct {volatile uint32_t request,complete;fm1_core1_fn fn;void*context;uint32_t check,timeouts,max_wait_ticks;int32_t avg_ticks;} aw;
static struct {uint32_t count;} fm1_core1_fault;
static int aw_online;static int32_t aw_published_avg;static uint32_t aw_t0,ticks,stops,barred,barriers,expect_held;
static volatile uint8_t fm1_rust_atomic_lock;
static uint32_t fm1_rust_atomic_recoveries;
static uint32_t fm1_ticks(void){return ++ticks;}
static uint32_t aw_load(volatile uint32_t*p){return *p;}
static void aw_store(volatile uint32_t*p,uint32_t v){*p=v;}
static int fm1_core1_online(void){return aw_online;}
static uint32_t fm1_cnum(void){return 0;}
static void fm1_mcu_park(void){for(;;);}
static void fm1_core1_stop(void){if(expect_held){assert(fm1_rust_atomic_lock==1);expect_held=0;}stops++;}
static void fm1_core1_bar_set(void){assert(stops);barred=1;}
'''+functions+'\n'+recovery+r'''
static void reset(void){memset(&aw,0,sizeof aw);fm1_core1_fault.count=0;aw_online=1;aw_published_avg=0;aw_t0=ticks=stops=barred=barriers=expect_held=0;fm1_rust_atomic_lock=0;fm1_rust_atomic_recoveries=0;}
static void work(void*p){(void)p;}
int main(void){
    reset();aw.request=aw.complete=7;aw.avg_ticks=13930;
    assert(fm1_core1_join()==1 && aw_published_avg==13930 && fm1_core1_load()==10);
    aw.avg_ticks=278600;assert(fm1_core1_load()==10); /* unpublished worker telemetry */
    reset();aw.request=aw.complete=7;fm1_core1_fault.count=1;
    assert(!fm1_core1_submit(work,0));
    assert(fm1_core1_join()==0 && !aw_online && stops==1 && barred && !aw_published_avg);
    reset();aw.request=8;aw.complete=7;fm1_core1_fault.count=1;
    assert(fm1_core1_join()==0 && aw.complete==8 && !aw_online && stops==1);
    reset();aw.request=8;aw.complete=7;
    assert(fm1_core1_join()==0 && aw.timeouts==1 && stops==1 && !aw_online);
    reset();aw.request=aw.complete=17;fm1_rust_atomic_lock=1;expect_held=1;
    fm1_rust_atomic_stopped_owner();
    assert(stops==1 && barred && fm1_core1_fault.count==1 && fm1_rust_atomic_recoveries==1 && !fm1_rust_atomic_lock && barriers==2);
    assert(aw.request==17 && aw.complete==17); /* recovery cannot forge a successful join */
    assert(fm1_core1_join()==0 && !aw_online && stops==2);
    puts("HAL: completed-only telemetry, fault-before-complete precheck, refusal, timeout, confirmed-stop lock recovery passed; target instruction proof is separate");
    return 0;
}
'''
Path(sys.argv[1]).write_text(source)
