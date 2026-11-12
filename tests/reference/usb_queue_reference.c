/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <assert.h>
#include <stdio.h>
#else
#define assert(v) do{if(!(v))__builtin_trap();}while(0)
#endif
#include "dsp/rust_usb.h"
static uint32_t irq_flags,write_steps,target,armed,pending,in_irq,irq_count;
static uint32_t midi_in_q[64],midi_out_q[64];
static volatile uint32_t mi_w,mi_r,mo_w,mo_r;
static struct {uint8_t config;}usb;static uint8_t uh_out;
#define MQ 64u
static void point(uint32_t step);
#define FM1_IRQ_STATE_TEST 1
#define FM1_IRQ_STATE_READ(v) do{(v)=irq_flags;point(0);}while(0)
#define FM1_IRQ_STATE_WRITE(v) do{uint32_t step=(write_steps++&1u)?6u:1u;irq_flags=(v);point(step);}while(0)
#define FM1_IRQ_STATE_FENCE() point(5)
#include "fm1_irq_state.h"
static uint32_t irq_save(void){
#if EN_RUST_HAL
 return fm1_irq_state_save();
#else
 point(0);irq_flags&=~256u;point(1);return 0;
#endif
}
static void irq_restore(uint32_t flags){
#if EN_RUST_HAL
 fm1_irq_state_restore(flags);
#else
 (void)flags;point(5);irq_flags|=256u;point(6);
#endif
}
static void usb_transport_publish(void){point(3);}
#define RING_PUBLISH() usb_transport_publish()
#include "queue_source.inc"
static void point(uint32_t step){
    if(armed&&step==target){armed=0;pending=1;}
    if(pending&&(irq_flags&256u)&&!in_irq){
        uint32_t saved=irq_flags;pending=0;in_irq=1;irq_flags&=~256u;
        plat_midi_out(0x22222222u);irq_count++;
#if EN_RUST_USB || EN_RUST_HAL
        assert(irq_flags==(saved&~256u));
#endif
        irq_flags=saved;in_irq=0;
    }
}
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void queue_emit(const void *,uint32_t);
static void emit(const void*p,uint32_t n){queue_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_queue_run(void){
    static const uint32_t bases[]={0,63,0xffffffc0u,0xfffffffcu};
    static const uint32_t stages[]={0,1,3,5,6};
    static const uint32_t flags[]={0,0x80000001u,0x12345678u,0xffffffffu};
    uint32_t cases=0;
    for(uint32_t f=0;f<4;f++)for(uint32_t gate=0;gate<2;gate++)for(uint32_t b=0;b<4;b++)for(uint32_t fill=0;fill<=64;fill++)for(uint32_t hook=0;hook<5;hook++)for(uint32_t enabled=0;enabled<4;enabled++){
        uint32_t initial=(flags[f]&~256u)|(gate<<8),word=0;
        irq_flags=initial;write_steps=0;target=stages[hook];armed=1;pending=in_irq=irq_count=0;
        usb.config=(uint8_t)(enabled&1u);uh_out=(uint8_t)(enabled>>1);mo_r=bases[b];mo_w=mo_r+fill;
        for(uint32_t i=0;i<64;i++)midi_out_q[i]=0xcccc0000u+i;
        for(uint32_t i=0;i<fill;i++)midi_out_q[(mo_r+i)%64]=0x10000000u+i;
        plat_midi_out(0x11111111u);
#if EN_RUST_USB || EN_RUST_HAL
        assert(irq_flags==initial); /* including a caller whose IRQ gate was clear */
#else
        assert(irq_flags==(initial|256u)); /* explicit inherited C0/HAL0 witness */
#endif
        irq_flags|=256u;point(99);armed=0;
        const uint32_t state[]={mo_w,mo_r,irq_count};emit(state,sizeof state);emit(midi_out_q,sizeof midi_out_q);
        mi_w=mo_w;mi_r=mo_r;memcpy(midi_in_q,midi_out_q,sizeof midi_in_q);
        uint32_t count=0;
        while(plat_midi_in(&word)){assert(count++<64);emit(&word,4);}
        emit(&count,4);assert(mi_r==mi_w);cases++;
    }
    return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_queue_run()<0;}
#endif
