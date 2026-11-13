/* SPDX-License-Identifier: GPL-3.0-only */
/* Real frozen/current USB driver; the physical HAL is a retained-read double. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <assert.h>
#include <stdio.h>
#else
#define assert(value) do{if(!(value))__builtin_trap();}while(0)
#endif
/* Explicit single-CPU test provider, never a physical fallback. */
static uint32_t usb_tx_fixture_irq=256;
static uint32_t usb_tx_fixture_enter(void){uint32_t flags=usb_tx_fixture_irq;usb_tx_fixture_irq&=~256u;return flags;}
static void usb_tx_fixture_exit(uint32_t flags){usb_tx_fixture_irq=flags;}
#ifndef USB_TX_IRQ_ENTER
#define USB_TX_IRQ_ENTER() usb_tx_fixture_enter()
#define USB_TX_IRQ_EXIT(flags) usb_tx_fixture_exit(flags)
#endif
#ifndef RING_PUBLISH
#define RING_PUBLISH() __asm__ volatile("" ::: "memory")
#endif
static void fm1_delay_ms(uint32_t ms){(void)ms;}
#include "usb_source.inc"
#ifdef USB_TEST_SINK
static void emit(const void *p,uint32_t n){USB_TEST_SINK(p,n);}
#elif defined(__wasm__)
__attribute__((import_module("env"),import_name("emit"))) extern void usb_emit(const void *,uint32_t);
static void emit(const void *p,uint32_t n){usb_emit(p,n);}
#else
static void emit(const void *p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint8_t registers[8][256],index_reg,pending_data;
static const uint8_t *retained;static uint8_t expected[68];static uint32_t retained_length,cases;
static void event(uint32_t op,uint32_t a,uint32_t b){const uint32_t values[]={op,a,b};emit(values,sizeof values);}
static void complete(void){if(retained){for(uint32_t i=0;i<retained_length;i++)assert(retained[i]==expected[i]);retained=0;}registers[0][S_CSR0]=0;}
static uint32_t identity(const void *p){
    if(p==ep0buf)return 0;if(p==ep1tx)return 1;if(p==ep1rx)return 2;
#if FELUCCA_CDC
    if(p==ep2tx)return 3;if(p==ep3tx)return 4;if(p==ep3rx)return 5;
#endif
#if FELUCCA_UAC
    if(p==ep4tx[0])return 6;if(p==ep4tx[1])return 7;
#endif
    return 255;
}
static void fm1_usb_reset(void){event(1,0,0);}
static void fm1_usb_attach(void *p){event(2,identity(p),0);}
static void fm1_usb_off(void){event(3,0,0);}
static uint32_t fm1_usb_sie_on(void){return 1;}
static void fm1_usb_sie_wr_start(uint32_t r,uint32_t v){
    event(4,r,v);
    if(r==S_INDEX)index_reg=(uint8_t)v;
    else registers[r<16?0:index_reg][r]=(uint8_t)(r==S_CSR0&&index_reg==0?v&2u:v);
}
static void fm1_usb_sie_rd_start(uint32_t r){pending_data=registers[r<16?0:index_reg][r];event(5,r,pending_data);}
static uint32_t fm1_usb_sie_done(void){event(6,0,0);return 1;}
static uint32_t fm1_usb_sie_data(void){return pending_data;}
static void fm1_usb_ep0_buf(void *p){event(7,identity(p),0);}
static void fm1_usb_ep4_txbuf(void *p){event(8,identity(p),0);}
static void fm1_usb_ep4_send(void *p,uint32_t n){event(9,identity(p),n);}
static void fm1_usb_ep_txbuf(uint32_t ep,void *p){event(10,ep,identity(p));}
static void fm1_usb_host_attach(void *p){event(11,identity(p),0);}
static uint32_t fm1_usb_line(void){return 0;}
static void fm1_usb_ep_rxbuf(uint32_t ep,void *p){event(12,ep,identity(p));}
static void fm1_usb_ep0_send(void *p,uint32_t n){
    assert(!retained&&n<=64);event(13,identity(p),n);emit(p,n);
    retained=p;retained_length=n;memcpy(expected,p,n);
}
static void fm1_usb_ep_send(uint32_t ep,void *p,uint32_t n){event(14,ep,n);
#ifdef USB_TX_TEST_SEND
    USB_TX_TEST_SEND(ep,p,n);
#else
    (void)p;
#endif
}
static void fm1_usb_rx_sync(void){event(15,0,0);}
static void fm1_usb_ep_enable(uint32_t eps){event(16,eps,0);}
static uint32_t fm1_usb_sof_take(void){return 0;}
static void reset(void){
    complete();memset(&usb,0,sizeof usb);memset(registers,0,sizeof registers);index_reg=0;usb.up=1;
    memset(ep0buf,0xa5,sizeof ep0buf);
#if FELUCCA_CDC
    memset(&cdc,0,sizeof cdc);
#endif
#if FELUCCA_UAC
    memset(&uac,0,sizeof uac);
#endif
}
static void state(void){
    const uint32_t values[]={usb.config,usb.pend_addr,usb.has_pend_addr,usb.e0_tx,usb.e0_zlp,usb.e0_left,usb.setups,usb.timeouts,
#if FELUCCA_CDC
        cdc.dtr,cdc.e0_rx,
#else
        0,0,
#endif
#if FELUCCA_UAC
        uac.alt,uac.e0_rx,uac.starts,
#else
        0,0,0,
#endif
    };emit(values,sizeof values);emit(usb.last_setup,8);
#if FELUCCA_CDC
    emit(cdc.line,7);
#endif
}
static void finish(void){
    uint32_t polls=0;state();
    while(usb.e0_tx){assert(polls++<1026);complete();ep0_service();state();}
    complete();cases++;
}
static void setup(uint32_t request,uint32_t value,uint32_t iface,uint32_t length){
    reset();usb.config=(uint8_t)(cases&1u);ep0buf[0]=(uint8_t)(request>>8);ep0buf[1]=(uint8_t)request;
    ep0buf[2]=(uint8_t)value;ep0buf[3]=(uint8_t)(value>>8);ep0buf[4]=(uint8_t)iface;ep0buf[5]=0;
    ep0buf[6]=(uint8_t)length;ep0buf[7]=(uint8_t)(length>>8);registers[0][S_CSR0]=1;
    ep0_service();finish();
    /* Delayed SET_ADDRESS commits on the next poll, before CSR0 is read. */
    ep0_service();state();
#if FELUCCA_CDC || FELUCCA_UAC
    if(
#if FELUCCA_CDC
       cdc.e0_rx ||
#endif
#if FELUCCA_UAC
       uac.e0_rx ||
#endif
       0) {
        const uint32_t n=value%9;
        for(uint32_t i=0;i<n;i++)ep0buf[i]=(uint8_t)(i*13+7);
        registers[0][S_COUNT0]=(uint8_t)n;registers[0][S_CSR0]=1;ep0_service();state();complete();
    }
#endif
}
#if defined(__wasm__) && !defined(USB_CONTROL_EMBEDDED)
__attribute__((export_name("run")))
#endif
int usb_control_run(void){
    for(uint32_t request=0;request<65536;request++){
#if EN_RUST_USB
        const uint32_t length=fm1_rust_usb_descriptor_length(USB_DESCRIPTOR_FLAGS,FELUCCA_USB_PID,request);
#else
        const uint8_t *data=0;uint16_t size=0;
        const uint32_t length=get_desc(request,&data,&size)?size:0;
#endif
        emit(&length,sizeof length);
        for(uint32_t offset=0;offset<length;offset+=64){
            const uint32_t n=length-offset>64?64:length-offset;
#if EN_RUST_USB
            const int32_t copied=fm1_rust_usb_descriptor_copy(USB_DESCRIPTOR_FLAGS,FELUCCA_USB_PID,request,offset,ep0buf,n);
            assert(copied==(int32_t)n);
#else
            memcpy(ep0buf,data+offset,n);
#endif
            emit(ep0buf,n);
        }
        cases++;
    }
    static const uint32_t descriptors[]={0x0100,0x01ff,0x0200,0x02ff,0x0300,0x0301,0x0302,0x0303,0x9900};
    static const uint32_t wanted[]={0,1,7,8,9,17,18,63,64,65,101,127,128,255,256,65535};
    for(uint32_t i=0;i<9;i++)for(uint32_t k=0;k<16;k++)setup(0x8006,descriptors[i],0x0409,wanted[k]);
    static uint8_t source[1024];for(uint32_t i=0;i<sizeof source;i++)source[i]=(uint8_t)(i*37+5);
    static const uint16_t lengths[]={0,1,63,64,65,127,128,255,256,511,1024};
    for(uint32_t i=0;i<11;i++)for(uint32_t k=0;k<16;k++){
        reset();e0_send(source,lengths[i],(uint16_t)wanted[k]);finish();
    }
    static const uint32_t requests[]={0x0005,0x0009,0x8008,0x8000,0x8100,0x8200,0x010b,0x810a,0x0201,0x2120,0xa121,0x2122,0x2123,0x2201,0xa281,0xa282,0xa283,0xffff};
    static const uint32_t values[]={0,1,2,3,0x0100,0x0101,0x7fff,0xffff};
    static const uint32_t interfaces[]={0,1,2,3,0x81,0x82,0x83,0x84,0xff};
    for(uint32_t i=0;i<18;i++)for(uint32_t value=0;value<8;value++)for(uint32_t iface=0;iface<9;iface++){
        setup(requests[i],values[value],interfaces[iface],7);
    }
    /* SentStall / SetupEnd / busy and idle polls on an active transfer or
     * pending class OUT stage. A retained-read completion precedes hardware's
     * next packet, so a new SETUP never fabricates endpoint exclusivity. */
    static const uint32_t csrs[]={0,1,2,4,6,0x10,0x11,0x12,0x14,0x15};
    for(uint32_t kind=0;kind<3;kind++)for(uint32_t k=0;k<10;k++) {
#if USB_CONTROL_CONFIRMED_ABORTS
        /* Active abort flags cannot prove retained DMA retirement. Those seven
         * app cases live in usb_lease_reference.c with the stop/retention oracle;
         * idle class-stage aborts and all loader cases remain here. */
        if(kind==0 && (csrs[k]&0x14u))continue;
#endif
        reset();
        if(kind==0)e0_send(source,128,129);
        else {
            ep0buf[0]=kind==1?0x21:0x22;ep0buf[1]=kind==1?0x20:1;
            ep0buf[2]=0;ep0buf[3]=kind==1?0:1;ep0buf[4]=kind==1?0:0x84;
            ep0buf[5]=0;ep0buf[6]=7;ep0buf[7]=0;registers[0][S_CSR0]=1;
            ep0_service();
        }
        complete();
        memset(ep0buf,0,8);ep0buf[0]=0x80;ep0buf[1]=8;ep0buf[6]=1;
        registers[0][S_COUNT0]=8;registers[0][S_CSR0]=(uint8_t)csrs[k];
        ep0_service();finish();
    }
    return (int)cases;
}
#if !defined(__wasm__) && !defined(USB_CONTROL_EMBEDDED)
int main(void){return usb_control_run()<0;}
#endif
