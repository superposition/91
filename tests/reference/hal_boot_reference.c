/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual cstart body with deterministic memory sections and callback probes. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <setjmp.h>
#include "fm1_rust_hal.h"
#define BOOTGUARD_MAGIC 0x42475244u
#define STK_W(a) ((uint32_t *)(void *)(a))
#define FM1_GUARD_STACK 1
#define FM1_GUARD_WRITE 2
#define FM1_GUARD_BUS 4
#define FM1_GUARD_PC 8
struct bootguard_s{uint32_t magic,failed,pending;}bootguard;
static struct {uint8_t p3_rst;uint32_t rst_src;uint8_t wdt_con;}fm1_boot;
static uint32_t bss[6],pool[6],data[6],rt[6],dsp[6],hot[6];
static uint32_t load_data[4]={1,2,3,4},load_rt[4]={5,6,7,8},load_dsp[4]={9,10,11,12},load_hot[4]={13,14,15,16};
#define _bss_start (bss+1)
#define _bss_end (bss+5)
#define _pool_start (pool+1)
#define _pool_end (pool+5)
#define _data_start (data+1)
#define _data_end (data+5)
#define _data_load load_data
#define _rt_start (rt+1)
#define _rt_end (rt+5)
#define _rt_load load_rt
#define _dsp_start (dsp+1)
#define _dsp_end (dsp+5)
#define _dsp_load load_dsp
#define _rh_start (hot+1)
#define _rh_end (hot+5)
#define _rh_load load_hot
#define _ustack_lo ((char *)0x01c74100u)
#define _ustack_top ((char *)0x01c7a000u)
#define _sstack_lo ((char *)0x01c7a100u)
#define _sstack_top ((char *)0x01c7c000u)
#define _cpu1_stacks_lo ((char *)0x01c01760u)
static uint8_t safe_mode;static uint32_t held,barred,unknown,gate_calls,action,hash=2166136261u;static FILE *trace;static jmp_buf exit_boot;
static void word(uint32_t value){for(uint32_t i=0;i<4;i++){uint8_t b=(uint8_t)(value>>(8*i));hash=(hash^b)*16777619u;if(trace)fputc(b,trace);}}
static void event(uint32_t kind,uint32_t value){word(kind);word(value);}
static void fm1_time_init(void){event(1,0);}
static void fm1_reset_reason(void){event(2,0);fm1_boot.p3_rst=0x61;fm1_boot.rst_src=0x13579bdf;fm1_boot.wdt_con=0x4d;}
static void fm1_wdt_arm(uint32_t value){event(3,value);}
static void fm1_core1_stop(void){held=1;event(4,0);}
static int fm1_hal_dma_boot_quiescent(void){assert(held);gate_calls++;return !unknown;}
static void fm1_enter_uboot(void){event(5,0);action=2;longjmp(exit_boot,1);}
static void fm1_irq_init(void){assert(held&&bss[1]==0xaaaaaaaa);event(6,0);}
static void fm1_mailbox_clear(void){
    for(uint32_t i=1;i<5;i++){assert(bss[i]==0&&pool[i]==0);assert(data[i]==i&&rt[i]==i+4&&dsp[i]==i+8&&hot[i]==i+12);}
    event(7,0);
}
static void stk_paint(uint32_t *lo,uint32_t *hi){event(8,(uint32_t)(uintptr_t)lo);event(9,(uint32_t)(uintptr_t)hi);}
static void fm1_guard_enable(uint32_t flags){event(10,flags);}
static int fm1_core1_barred(void){event(11,barred);return barred;}
static void fm1_core1_boot(void){assert(held&&!bootguard.failed&&!barred);event(12,0);}
static void fm1_main(void){event(13,safe_mode);assert(fm1_boot.p3_rst==0x61&&fm1_boot.rst_src==0x13579bdf&&fm1_boot.wdt_con==0x4d);action=safe_mode?1:0;longjmp(exit_boot,1);}
#include "cstart.inc"
int main(int argc,char **argv){
    const uint32_t fails[]={0,1,2,3,4,0xffffffffu},pending[]={0,1,2,0xffffffffu};uint32_t rows=0;
    if(argc>1){trace=fopen(argv[1],"wb");assert(trace);}
    unknown=argc>2;
    for(uint32_t valid=0;valid<2;valid++)for(uint32_t f=0;f<6;f++)for(uint32_t p=0;p<4;p++)for(barred=0;barred<2;barred++){
        bootguard.magic=valid?BOOTGUARD_MAGIC:0x12345678;bootguard.failed=fails[f];bootguard.pending=pending[p];
        held=gate_calls=action=0;
        uint32_t *const sections[]={bss,pool,data,rt,dsp,hot};
        for(uint32_t j=0;j<6;j++)for(uint32_t k=0;k<6;k++)sections[j][k]=0xaaaaaaaau;
        if(!setjmp(exit_boot))fm1_cstart();
#if EN_RUST_HAL
        if(unknown){assert(action==2&&bss[1]==0xaaaaaaaau);}
        else if(action!=2)assert(gate_calls==1);
        if(gate_calls)assert(bootguard.magic==BOOTGUARD_MAGIC&&bootguard.pending==1);
#endif
        for(uint32_t j=0;j<6;j++){assert(sections[j][0]==0xaaaaaaaau&&sections[j][5]==0xaaaaaaaau);for(uint32_t k=0;k<6;k++)word(sections[j][k]);}
        word(bootguard.magic);word(bootguard.failed);word(bootguard.pending);word(action);rows++;
    }
    if(trace)fclose(trace);printf("{\"core2\":%u,\"rows\":%u,\"hash\":\"%08x\"}\n",(uint32_t)EN_CORE2,rows,hash);return 0;
}
