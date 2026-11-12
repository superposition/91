/* SPDX-License-Identifier: GPL-3.0-only */
#define USB_TRANSPORT_EMBEDDED 1
#include "usb_transport_reference.c"
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_transport_edge_run(void){
    reset();static uint8_t m[4114];
    static const uint8_t headers[][5]={{0x43,0,9,32,0},{0x43,0,0,1,27},{0x7d,0x50,0x56,0},{0x7d,0x50,0x56,127},{0x7d,0x50,0x42,3},{0x7d,0x50,0x60,0}};
    static const uint32_t lengths[]={4102,161,133,4,12,6};
    for(uint32_t k=0;k<6;k++){
        for(uint32_t i=0;i<4114;i++)m[i]=(uint8_t)(i%127);memcpy(m,headers[k],5);release();frame(m,lengths[k]);
#if USB_DX
        assert(dx_state==2&&dx_kind==k+1&&dx_pos==lengths[k]);
        uint32_t before=hash(dx_buf,DX_MAX);
#endif
        byte(0xf0);byte(0xf7);trace();payload();
#if USB_DX
        assert(dx_state==2&&dx_kind==k+1&&dx_pos==lengths[k]&&hash(dx_buf,DX_MAX)==before);
#endif
    }
    event(90,bytes_seen,packets_seen);return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_transport_edge_run()<0;}
#endif
