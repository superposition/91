/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
static void tx_observe(uint32_t ep,void *p,uint32_t n);
#define USB_TX_TEST_SEND tx_observe
#define USB_CONTROL_EMBEDDED 1
#include "usb_control_reference.c"
static const uint8_t *leased[4];static uint8_t sent[4][64];static uint32_t sizes[4],packets,now_ms,idles,drain;
static void tx_observe(uint32_t ep,void *p,uint32_t n){
    assert(ep>0&&ep<4&&n<=64&&!leased[ep]);leased[ep]=p;sizes[ep]=n;memcpy(sent[ep],p,n);emit(p,n);packets++;
}
static void tx_check(uint32_t ep){if(leased[ep])for(uint32_t i=0;i<sizes[ep];i++)assert(leased[ep][i]==sent[ep][i]);}
static void tx_complete(uint32_t ep){tx_check(ep);leased[ep]=0;registers[ep][S_TXCSR1]=0;}
#if FELUCCA_OTA
static uint32_t ota_now_ms(void){return now_ms;}
static void ota_idle(void){now_ms++;idles++;if(drain){tx_complete(1);ep1_tx();}}
#endif
static uint32_t hash(const void*p,uint32_t n){const uint8_t*b=p;uint32_t v=2166136261u;for(uint32_t i=0;i<n;i++)v=(v^b[i])*16777619u;return v;}
static void tx_reset(uint32_t base,uint32_t midi,uint32_t sysex,uint32_t busy,uint32_t csr){
    tx_complete(1);reset();memset(ep1tx,0xa5,sizeof ep1tx);usb.config=1;mo_r=base;mo_w=base+midi;
    for(uint32_t i=0;i<64;i++)midi_out_q[i]=0xcccc0000u+i;
    for(uint32_t i=0;i<midi;i++)midi_out_q[(base+i)%64]=0x90803c09u+i;
#if FELUCCA_OTA
    so_r=base;so_w=base+sysex;sx_busy=(uint8_t)busy;
    for(uint32_t i=0;i<64;i++)sx_out_q[i]=0xdddd0000u+i;
    for(uint32_t i=0;i<sysex;i++)sx_out_q[(base+i)%64]=0x7d50f004u+i;
#else
    (void)sysex;(void)busy;
#endif
    registers[1][S_TXCSR1]=(uint8_t)csr;
    if(csr&1u){leased[1]=ep1tx;sizes[1]=64;memcpy(sent[1],ep1tx,64);}
}
static void tx_state(int result){
    const uint32_t state[]={mo_w,mo_r,usb.tx_pkts,packets,hash(midi_out_q,sizeof midi_out_q),hash(ep1tx,64),idles,now_ms,(uint32_t)result,
#if FELUCCA_OTA
    so_w,so_r,sx_busy,hash(sx_out_q,sizeof sx_out_q),
#else
    0,0,0,0,
#endif
    };emit(state,sizeof state);cases++;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_tx_run(void){
    static const uint32_t fills[]={0,1,15,16,17,31,32,47,48,63,64};
    static const uint32_t bases[]={0,63,0xffffffc0u,0xfffffffcu};
    static const uint32_t csrs[]={0,1,0x80,0x81};
    for(uint32_t b=0;b<4;b++)for(uint32_t m=0;m<11;m++)for(uint32_t s=0;s<11;s++)for(uint32_t busy=0;busy<2;busy++)for(uint32_t c=0;c<4;c++){
        tx_reset(bases[b],fills[m],fills[s],busy,csrs[c]);
        ep1_tx();tx_check(1);tx_state(0);ep1_tx();tx_check(1);tx_state(0);
        tx_complete(1);ep1_tx();tx_check(1);tx_state(0);
    }
    for(uint32_t n=0;n<=64;n++){tx_reset(0xfffffffeu,n,64-n,n&1u,0);ep1_tx();tx_check(1);tx_state(0);}
#if FELUCCA_OTA
    static uint8_t message[640];
    for(uint32_t n=0;n<=640;n++)for(uint32_t backlog=0;backlog<3;backlog++)for(uint32_t config=0;config<2;config++)for(uint32_t mode=0;mode<2;mode++)for(uint32_t feed=0;feed<2;feed++){
        tx_reset(0xfffffffcu,16,backlog==0?0:backlog==1?63:64,0,0);
#if USB_TX_ACTIVE_CONFIG_ONLY
        if(!config)continue;
#endif
        usb.config=(uint8_t)config;now_ms=n&1u?0xfffffff0u:100;idles=0;drain=feed;
        for(uint32_t i=0;i<n;i++)message[i]=(uint8_t)(i&127u);if(n)message[0]=0xf0;if(n>1)message[n-1]=0xf7;
        const int result=mode?ota_wire_send(message,n):usb_sysex_try(message,n);
        tx_check(1);tx_state(result);tx_complete(1);
        /* No producer remains: drain and observe every remaining byte. */
        for(uint32_t i=0;i<9;i++){ep1_tx();tx_check(1);tx_state(result);tx_complete(1);}
    }
#endif
    return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_tx_run()<0;}
#endif
