/* SPDX-License-Identifier: GPL-3.0-only */
/* Real driver SysEx/RX path; shared physical HAL double retains its roots. */
#define USB_CONTROL_EMBEDDED 1
#include "usb_control_reference.c"
#if FELUCCA_OTA
static uint32_t ota_now_ms(void){return cases;}
static void ota_idle(void){}
#endif
static uint32_t random_word=0x345629u,bytes_seen,packets_seen;
static uint32_t random32(void){random_word=random_word*1664525u+1013904223u;return random_word;}
static uint32_t hash(const void *p,uint32_t n){const uint8_t*b=p;uint32_t v=2166136261u;for(uint32_t i=0;i<n;i++)v=(v^b[i])*16777619u;return v;}
static void trace(void){
    const uint32_t values[]={usb.sx_len,usb.sx_on,usb.uboot_req,usb.ota_req,usb.tel_req,usb.rx_pkts,mi_w,mi_r,hash(usb.sysex,16),
#if USB_DX
    px_pos,px_on,px_w,px_r,dx_pos,dx_on,dx_state,dx_kind,hash(px_buf,16),hash(dx_buf,DX_MAX),
#else
    0,0,0,0,0,0,0,0,0,0,
#endif
#if FELUCCA_OTA
    sx_pos,sx_frame_len,sx_ready,sx_collect,hash(sx_frame,640),
#else
    0,0,0,0,0,
#endif
    };emit(values,sizeof values);emit(midi_in_q,sizeof midi_in_q);
#if USB_DX
    emit(px_q,sizeof px_q);
#endif
    cases++;
}
static void payload(void){emit(usb.sysex,16);
#if USB_DX
    emit(px_buf,16);emit(dx_buf,DX_MAX);
#endif
#if FELUCCA_OTA
    emit(sx_frame,640);
#endif
}
static void release(void){
#if USB_DX
    dx_state=0;px_r=px_w;
#endif
#if FELUCCA_OTA
    sx_ready=0;
#endif
    mi_r=mi_w;
}
static void byte(uint8_t b){sysex_byte(b);bytes_seen++;if((bytes_seen&7u)==0||b==0xf0||b==0xf7)trace();}
static void frame(const uint8_t*p,uint32_t n){byte(0xf0);for(uint32_t i=0;i<n;i++)byte(p[i]);byte(0xf7);payload();}
static void packet(const uint8_t*p,uint32_t n){
    complete();memcpy(ep1rx,p,n<64?n:64);registers[1][S_RXCSR1]=1;registers[1][S_RXCSR2]=0;
    registers[1][S_RXCOUNT1]=(uint8_t)n;registers[1][S_RXCOUNT2]=(uint8_t)(n>>8);
    ep1_rx();packets_seen++;trace();
}
#if defined(__wasm__) && !defined(USB_TRANSPORT_EMBEDDED)
__attribute__((export_name("run")))
#endif
int usb_transport_run(void){
    reset();memset(midi_in_q,0,sizeof midi_in_q);mi_w=mi_r=0xfffffff0u;
#if USB_DX
    memset(dx_buf,0,DX_MAX);memset(px_buf,0,16);memset(px_q,0,sizeof px_q);
    dx_pos=px_pos=0;dx_on=dx_state=dx_kind=px_on=0;px_w=px_r=0xfffffff0u;
#endif
#if FELUCCA_OTA
    memset(sx_frame,0,640);sx_pos=sx_frame_len=0;sx_ready=sx_collect=0;
#endif
    static const uint8_t commands[][8]={{0x7d,0x50,0x50,1,1,2,3,4},{0x7d,0x50,0x50,2},{0x7d,0x50,0x56,0x7e,15},{0x7d,0x50,0x42,1,31},{0x7d,0x50,0x42,4,31,1,2},{0x7d,0x50,0x42,5},{0x7d,0x50,0x42,6},{0x22,0x24,0x35,0x7d},{0x22,0x24,0x35,0x7f},{0x7d,0x50,0x46,0},{0x7d,0x50,0x46,1},{0x7d,0x50,0x46,2}};
    static const uint8_t sizes[]={8,4,5,5,7,4,4,4,4,4,4,4};
    for(uint32_t i=0;i<12;i++){release();frame(commands[i],sizes[i]);}
    /* Every byte mutation, including status values inside a USB SysEx packet. */
    for(uint32_t i=0;i<12;i++)for(uint32_t pos=0;pos<sizes[i];pos++)for(uint32_t v=0;v<256;v++){
        uint8_t m[8];memcpy(m,commands[i],8);m[pos]=(uint8_t)v;release();frame(m,sizes[i]);
    }
    /* All raw input/CIN/cable/status pairs through the actual bounded EP1 RX. */
    for(uint32_t i=0;i<4096;i++){
        uint8_t p[64];for(uint32_t j=0;j<16;j++){uint32_t v=i*16+j;p[4*j]=(uint8_t)(v>>8);p[4*j+1]=(uint8_t)v;p[4*j+2]=(uint8_t)random32();p[4*j+3]=(uint8_t)random32();}
        if((i&3u)==0)release();packet(p,64);
    }
    for(uint32_t n=0;n<=80;n++){uint8_t p[80];for(uint32_t i=0;i<80;i++)p[i]=(uint8_t)random32();packet(p,n);}
    /* Full short-command queue drops, wrapping indices and complete DX/OTA
     * lease backpressure, including overlong exact-capacity boundaries. */
    release();for(uint32_t i=0;i<40;i++)frame(commands[0],8);
    static uint8_t m[4114];
    static const uint32_t lengths[]={0,1,4,5,6,12,15,16,17,639,640,641,4101,4102,4111,4112,4113,4114};
    static const uint8_t headers[][5]={{0x43,0,9,32,0},{0x43,0,0,1,27},{0x7d,0x50,0x56,0},{0x7d,0x50,0x42,3},{0x7d,0x50,0x60,0}};
    for(uint32_t h=0;h<5;h++)for(uint32_t k=0;k<18;k++){
        for(uint32_t i=0;i<4114;i++)m[i]=(uint8_t)(i%127);memcpy(m,headers[h],5);release();frame(m,lengths[k]);
        /* Keep ready, start another input and prove the pending length/payload
         * still belong to the queued message. Short commands remain independent. */
#if USB_DX
        uint32_t old_n=dx_pos,old_hash=hash(dx_buf,DX_MAX);uint8_t ready=dx_state;
#endif
        frame(commands[6],4);
#if USB_DX
        if(ready){assert(dx_pos==old_n&&dx_state==ready&&hash(dx_buf,DX_MAX)==old_hash);}
#endif
    }
    /* A read publication can be wrapped/empty/full without interpreting its
     * slot contents as new ownership. The next packet admission stays bounded. */
    trace();payload();event(90,bytes_seen,packets_seen);return (int)cases;
}
#if !defined(__wasm__) && !defined(USB_TRANSPORT_EMBEDDED)
int main(void){return usb_transport_run()<0;}
#endif
