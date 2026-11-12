/* SPDX-License-Identifier: GPL-3.0-only */
/* Real C/Rust UART consumer and actual USB MIDI queue. Hardware never starts. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#define RING_PUBLISH() __asm__ volatile("" ::: "memory")
static void fm1_delay_ms(uint32_t ms){(void)ms;}
#include "usb.c"
#include "midi_uart.c"
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void trs_emit(const void *,uint32_t);
static void emit(const void *p,uint32_t n){trs_emit(p,n);}
#else
static void emit(const void *p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t cases,dma_write,seed=0x91ca7e31u;
static uint32_t next(void){seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;return seed;}
static void reset(uint32_t index){
    memset(&um,0,sizeof um);memset(midi_in_q,0,sizeof midi_in_q);
    mi_r=mi_w=index;dma_write=0;
#if EN_RUST_USB
    fm1_rust_usb_trs_clear(um_ring);
#else
    for(uint32_t i=0;i<128;i++)um_ring[i]=UM_EMPTY;
#endif
}
static void witness(uint32_t kind,uint32_t index){
    const uint32_t values[]={kind,index,um.rd,um.bytes,um.drops,um.msgs,um.st,um.need,um.got,um.d0,um.sysex,mi_w,mi_r};
    uint8_t ring[128];for(uint32_t i=0;i<128;i++)ring[i]=um_ring[i];
    emit(values,sizeof values);emit(ring,sizeof ring);emit(midi_in_q,sizeof midi_in_q);cases++;
}
static void drain(uint32_t count){
    while(count-- && mi_r!=mi_w){const uint32_t event[]={0x4d494449u,midi_in_q[mi_r%MQ]};emit(event,sizeof event);mi_r++;}
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_trs_run(void){
    for(uint32_t context=0;context<16;context++)for(uint32_t byte=0;byte<256;byte++){
        reset(context&1?0xfffffff0u:0);
        um.st=(uint8_t)(context&2?0xc3:0x94);um.need=(uint8_t)(context&2?1:2);
        um.got=(uint8_t)(context&4?1:0);um.d0=37;um.sysex=(uint8_t)(context&8?1:0);
        if(context&1)mi_w=mi_r+MQ;
        um.bytes=0xffffffffu;um.drops=0xffffffffu;um.msgs=0xffffffffu;
        um_byte(byte);um_byte(60);um_byte(0xf8);um_byte(99);um_byte(0xf7);um_byte(37);
        witness(0,context*256+byte);drain(MQ);
    }
    reset(0xffffffe0u);
    for(uint32_t turn=0;turn<8192;turn++){
        const uint32_t n=next()%258u;
        for(uint32_t i=0;i<n;i++)um_ring[(dma_write+i)&127u]=(uint8_t)next();
        dma_write+=n;
        uint32_t count=n;
        if(turn%7==0 && count)count--; /* observed DMA count lags actual content */
        if(turn%17==0)count=0;
        if(turn%137==0)count=0xffffffffu; /* original wrapping drop/read arithmetic */
        uart_midi_take(count);
        witness(1,turn);drain(next()%97u);
    }
    drain(MQ);witness(2,0);
    return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_trs_run()<0;}
#endif
