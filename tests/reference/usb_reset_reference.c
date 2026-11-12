/* SPDX-License-Identifier: GPL-3.0-only */
/* Checks deliberately remain active when an optimized compiler defines NDEBUG. */
#undef NDEBUG
#include <stdint.h>
static uint32_t reset_enter(void);static void reset_exit(uint32_t);
static void publication(void);static void digest(const void*,uint32_t);
static void observe_tx(uint32_t,void*,uint32_t);
#define USB_TX_IRQ_ENTER() reset_enter()
#define USB_TX_IRQ_EXIT(flags) reset_exit(flags)
#define RING_PUBLISH() publication()
#define USB_TEST_SINK digest
#define USB_TX_TEST_SEND observe_tx
#define USB_CONTROL_EMBEDDED 1
#include "usb_control_reference.c"
#ifdef __wasm__
#define CHECK(v) do{if(!(v))__builtin_trap();}while(0)
#else
#define CHECK(v) do{if(!(v)){fprintf(stderr,"reset check line %u\n",(unsigned)__LINE__);__builtin_trap();}}while(0)
#endif
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void reset_emit(const void*,uint32_t);
static void output(const void*p,uint32_t n){reset_emit(p,n);}
#else
static void output(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t checksum=2166136261u,events,irq,armed,pending,in_reset,in_send,hook,wanted,mode;
static uint32_t pubs,before_reset,applied,idles,now_ms,enter_count,source_bytes,stale_witnesses;
static const uint8_t *tx_root;static uint8_t tx_copy[64];static uint32_t tx_length;
static void digest(const void*p,uint32_t n){const uint8_t*b=p;for(uint32_t i=0;i<n;i++)checksum=(checksum^b[i])*16777619u;events++;}
static void tx_complete(void){if(tx_root){for(uint32_t i=0;i<tx_length;i++)CHECK(tx_root[i]==tx_copy[i]);tx_root=0;}registers[1][S_TXCSR1]=0;}
static void observe_tx(uint32_t ep,void*p,uint32_t n){CHECK(ep==1&&n<=64&&!tx_root);tx_root=p;tx_length=n;memcpy(tx_copy,p,n);source_bytes+=n;digest(p,n);}
static void apply_reset(void){
    const uint32_t saved=irq;irq&=~256u;in_reset=1;applied++;before_reset=pubs;
    registers[0][S_INTRUSB]=4;registers[0][S_INTRTX1]=0;registers[0][S_INTRRX1]=0;
    usb_poll();registers[0][S_INTRUSB]=0;
    CHECK(so_r==so_w);
    if(mode==2&&usb.up){
        memset(ep0buf,0,8);ep0buf[1]=9;ep0buf[2]=1;registers[0][S_CSR0]=1;
        ep0_service();complete();CHECK(usb.config==1);
    }
    in_reset=0;irq=saved;
}
static void point(uint32_t at){
    if(in_send&&!in_reset&&armed&&at==hook&&pubs==wanted){armed=0;pending=1;}
    if(pending&&(irq&256u)&&!in_reset){pending=0;apply_reset();}
}
static uint32_t reset_enter(void){uint32_t saved=irq;point(0);irq&=~256u;enter_count++;point(1);return saved;}
static void reset_exit(uint32_t flags){point(5);irq=flags;point(6);}
static void publication(void){if(in_send&&!in_reset){pubs++;point(3);}}
static uint32_t ota_now_ms(void){return now_ms;}
static void ota_idle(void){
#if EN_RUST_USB
    CHECK(irq&256u); /* no wait/idle under even an incoming disabled IRQ gate */
#endif
    idles++;now_ms++;point(8);tx_complete();ep1_tx();
}
static void run(uint32_t size,uint32_t blocking,uint32_t backlog,uint32_t stage,uint32_t packet,uint32_t reset_mode,uint32_t generation,uint32_t initial){
    static uint8_t message[640];tx_complete();reset();
    irq=initial;pubs=before_reset=applied=idles=enter_count=source_bytes=0;pending=in_reset=0;
    so_r=0xfffffffcu;so_w=so_r+backlog;mo_r=mo_w=0;usb.config=1;usb.resets=generation;
    for(uint32_t i=0;i<64;i++)sx_out_q[i]=0x11110000u+i;
    for(uint32_t i=0;i<size;i++)message[i]=(uint8_t)(i&127u);if(size)message[0]=0xf0;if(size>1)message[size-1]=0xf7;
    sx_busy=0;checksum=2166136261u;events=0;hook=stage;wanted=packet;mode=reset_mode;armed=reset_mode!=0;now_ms=0xfffffff0u;
    if(generation==0xffffffffu)usb_poll();
    in_send=1;int result;
#if !EN_RUST_USB
    /* No fabricated dead-clock legacy experiment while the app IRQ gate is off. */
    if(!(initial&256u)&&blocking&&(size+2u)/3u>64u-backlog)result=-1;
    else
#endif
    result=blocking?ota_wire_send(message,size):usb_sysex_try(message,size);
    in_send=0;
#if EN_RUST_USB
    CHECK(irq==initial&&sx_busy==0);
    if(applied&&before_reset){CHECK(so_w==so_r);CHECK(result==-1 || (before_reset==(size+2u)/3u && pubs==before_reset));}
    if(generation==0xffffffffu){CHECK(result==-1&&usb.resets==0xffffffffu&&usb.detached&&!usb.up&&so_w==so_r+backlog);}
    if(usb.resets==0xffffffffu){uint32_t retries=usb.retries;usb_retry(now_ms+2000);CHECK(usb.retries==retries);}
    if(!(initial&256u))CHECK(idles==0);
#else
    if(applied&&before_reset&&so_w!=so_r)stale_witnesses++;
#endif
    const uint32_t state[]={size,blocking,backlog,stage,packet,reset_mode,generation,initial,(uint32_t)result,irq,pubs,before_reset,applied,idles,usb.resets,usb.up,usb.config,usb.detached,so_w,so_r,sx_busy,checksum,events,source_bytes,enter_count};
    output(state,sizeof state);output(sx_out_q,sizeof sx_out_q);cases++;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_reset_run(void){
    static const uint32_t generations[]={0,0xfffffffdu,0xfffffffeu};
    static const uint32_t stages[]={0,1,3,5,6,8};
    for(uint32_t g=0;g<3;g++)for(uint32_t reset_mode=1;reset_mode<=2;reset_mode++)for(uint32_t kind=0;kind<3;kind++){
        uint32_t size=kind==0?192u:640u,blocking=kind!=0,backlog=kind==2?64u:0u,packets=(size+2u)/3u;
        for(uint32_t s=0;s<6;s++)for(uint32_t packet=0;packet<=packets;packet++)run(size,blocking,backlog,stages[s],packet,reset_mode,generations[g],0xa5000101u);
    }
    for(uint32_t gate=0;gate<2;gate++)for(uint32_t n=0;n<=192;n++)for(uint32_t backlog=0;backlog<=64;backlog+=16){
        run(n,0,backlog,3,1,0,0,0xa5000001u|(gate<<8));
        run(n,1,backlog,3,1,0,0xffffffffu,0xa5000001u|(gate<<8));
    }
#if !EN_RUST_USB
    CHECK(stale_witnesses>0); /* the unmodified frozen driver's reset-tail bug */
#endif
    output(&stale_witnesses,4);return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_reset_run()<0;}
#endif
