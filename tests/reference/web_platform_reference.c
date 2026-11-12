/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual C0 host against the new browser host through its unchanged C ABI. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#define EN_WEB 1
#include "../../web/emu/amb_web.c"
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void wp_emit(const void*,uint32_t);
static void wp_bytes(const void*p,uint32_t n){wp_emit(p,n);}
#else
static void wp_bytes(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static void wp_word(uint32_t x){wp_bytes(&x,4);}
static uint32_t wp_host(uint32_t id,uint32_t index){
#if EN_RUST_WEB
    return fm1_rust_web_host_value(id,index);
#else
    switch(id){case 0:return store_writes;case 1:return clock_out_count;case 2:return midi_out_count;
    case 3:return (uint32_t)host_uh_want;case 4:return (uint32_t)rs_valid;case 5:return (uint32_t)store_lock;
    case 6:return cc_out_n[index&127u];case 7:return (uint32_t)last_vel;case 8:return last_out;
    case 9:return lit_btn;case 10:return dim_keys;case 11:return lit_keys;default:return 0;}
#endif
}
static void wp_state(void){unsigned i;wp_word(plat_ms());wp_word(plat_buttons());wp_word(plat_keys());
    wp_word(plat_master());wp_word(plat_seed());wp_word(plat_battery());wp_word((uint32_t)plat_usb_power());
    wp_word(web_blits());wp_word(web_cpu());wp_word((uint32_t)plat_store_seamless());
    for(i=0;i<12;i++)wp_word(wp_host(i,i*17));
    wp_word((uint32_t)plat_usb_host_state());wp_word((uint32_t)plat_usb_host_msg());
    wp_word((uint32_t)plat_usb_host_msg());}
static uint32_t wp_random=0x91cafeu;
static uint32_t wp_next(void){wp_random^=wp_random<<13;wp_random^=wp_random>>17;wp_random^=wp_random<<5;return wp_random;}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int platform_run(void){
    static uint16_t pixels[240*240];uint8_t source[3841],out[3841];unsigned i,j,obj,n,role;
    static const unsigned lengths[]={0,1,63,64,511,3840,3841};
    static const unsigned capacity[]={0,1,31,3840,3841,0xffffffffu};
    static const uint32_t xy[]={0,1,239,240,0xfffffffeu,0xffffffffu};
    for(i=0;i<3841;i++)source[i]=(uint8_t)(i*73u+19u);
    for(i=0;i<240*240;i++)pixels[i]=(uint16_t)(i*1337u);
    wp_state();
    for(i=0;i<1024;i++){
        web_buttons(wp_next());web_keys(wp_next());web_master(wp_next());web_seed(wp_next());
        for(role=0;role<7;role++){int32_t delta=(int32_t)(wp_next()%8193u)-4096;
            web_enc((int)role,delta);web_enc((int)role,-13);wp_word((uint32_t)plat_enc((int)role));wp_word((uint32_t)plat_enc((int)role));}
        web_enc(-1,10);web_enc(7,10);plat_keys_dim(wp_next());
        {uint32_t buttons=wp_next(),keys=wp_next();plat_leds(buttons,keys);}
        plat_usb_host((int)(i%3));plat_glow((int)(i%3));
        plat_midi_out(9u|0x90u<<8|(i%128u)<<16|(i%128u)<<24);
        plat_midi_out(11u|0xb0u<<8|(i%128u)<<16|127u<<24);plat_midi_out(15u|0xf8u<<8);
        wp_state();
    }
    for(obj=0;obj<33;obj++){
        wp_word((uint32_t)web_store_len(obj));wp_word(web_store(obj)!=0);
        for(i=0;i<sizeof lengths/sizeof *lengths;i++){
            n=lengths[i];wp_word((uint32_t)plat_store_save(obj,source,n));
            for(j=0;j<sizeof capacity/sizeof *capacity;j++){
                memset(out,0xcd,sizeof out);wp_word((uint32_t)plat_store_load(obj,out,capacity[j]));wp_bytes(out,sizeof out);
            }
        }
        wp_word(web_store_writes());
    }
    for(i=0;i<512;i++){
        uint32_t x=xy[i%6],y=xy[i/6%6],w=i%31,h=i/31%19;
        lcd_fill(x,y,w,h,(uint16_t)(i*771u));lcd_blit(x,y,w,h,pixels);
        wp_word(web_blits());wp_bytes(web_fb(),240u*240u*2u);
    }
    wp_word((uint32_t)plat_store_restore(4,source,64));
    wp_word((uint32_t)plat_store_save(5,source,64));
    wp_word((uint32_t)plat_store_restore(33,source,64));
    wp_state();return 1;
}
#ifndef __wasm__
int main(void){return platform_run()!=1;}
#endif
