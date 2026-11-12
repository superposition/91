/* SPDX-License-Identifier: GPL-3.0-only */
/* Existing app ABI plus synchronous platform thunks; no C host state, clock,
 * payload buffers, scenario parser, recorder or renderer loop. */
#include <stdint.h>
#include <time.h>
#include "rust_web_platform.h"
#ifndef EN_RUST_STORAGE
#define EN_RUST_STORAGE 0
#endif
#include "../firmware/src/app/plat.h"
static void lcd_sync(void) {}
static void lcd_fill(uint32_t x,uint32_t y,uint32_t w,uint32_t h,uint16_t c) {fm1_rust_web_fill(x,y,w,h,c);}
static void lcd_blit(uint32_t x,uint32_t y,uint32_t w,uint32_t h,const uint16_t *p) {fm1_rust_web_blit(x,y,w,h,p);}
uint32_t plat_ms(void){return fm1_rust_web_get(0);}
uint32_t plat_buttons(void){return fm1_rust_web_get(1);}
uint32_t plat_keys(void){return fm1_rust_web_get(2);}
uint32_t plat_master(void){return fm1_rust_web_get(3);}
uint32_t plat_battery(void){return fm1_rust_web_get(4);}
int plat_usb_power(void){return (int)fm1_rust_web_get(5);}
uint32_t plat_seed(void){return fm1_rust_web_get(9);}
extern int32_t fm1_rust_web_encoder(int32_t);
int32_t plat_enc(int role){return fm1_rust_web_encoder(role);}
void plat_keys_dim(uint32_t keys){fm1_rust_web_set(5,keys,0);}
void plat_leds(uint32_t buttons,uint32_t keys){fm1_rust_web_set(6,buttons,keys);}
void plat_glow(int on){fm1_rust_web_set(7,(uint32_t)on,0);}
void plat_play_red(int on){(void)on;}
int plat_midi_in(uint32_t *packet){return fm1_rust_web_midi_in(packet);}
void plat_midi_out(uint32_t packet){fm1_rust_web_midi_out(packet);}
void en_midi_out(uint32_t st,uint32_t a,uint32_t b){plat_midi_out(st>>4|st<<8|a<<16|b<<24);}
int plat_store_load(uint32_t object,void *dst,uint32_t max){return fm1_rust_web_store_load(object,dst,max);}
int plat_store_save(uint32_t object,const void *src,uint32_t n){return fm1_rust_web_store_save(object,src,n,0,EN_RUST_STORAGE);}
int plat_store_restore(uint32_t object,const void *src,uint32_t n){return fm1_rust_web_store_save(object,src,n,1,EN_RUST_STORAGE);}
int plat_store_locked(void){return (int)fm1_rust_web_get(12);}
int plat_store_seamless(void){return (int)fm1_rust_web_get(11);}
void plat_usb_host(int on){fm1_rust_web_set(8,(uint32_t)on,0);}
void plat_usb_host_diag(char *out){out[0]=0;}
int plat_usb_host_state(void){return (int)fm1_rust_web_get(13);}
int plat_usb_host_msg(void){return (int)fm1_rust_web_get(14);}
uint32_t plat_cpu_pct(void){
#ifdef EN_HOST_FIXED_CPU
    return EN_HOST_FIXED_CPU;
#else
    return fm1_rust_web_get(10);
#endif
}
uint32_t plat_xruns(void){return 0;}
uint32_t plat_late(void){return 0;}
uint32_t plat_peak_pct(void){return fm1_rust_web_get(10);}
uint32_t plat_cpu1_pct(void){return 0;}
uint32_t plat_core2(void){return 0;}
uint32_t fm1_sim_host_value(uint32_t id,uint32_t index){return fm1_rust_web_host_value(id,index);}
#define FM1_SIM_SOURCE_SUPPLIED 1
#define SIM_GFX "../firmware/src/gfx.c"
#define SIM_APP "../firmware/src/app/app.h"
#define SIM_PROJECT "../firmware/src/app/project.c"
#define SIM_SESSION "../firmware/src/app/jam_session.c"
#define SIM_UI "../firmware/src/app/ui.c"
#define SIM_USB "../firmware/src/app/jam_usb.h"
#include "rust_sim_app.c"
static double web_nanoseconds(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return (double)t.tv_sec*1e9+(double)t.tv_nsec;}
static const fm1_web_callbacks web_callbacks={fm1_sim_boot,fm1_sim_service,en_render,ui_frame,ui_input_only,web_nanoseconds};
static void run_ms(uint32_t ms){fm1_rust_web_advance(ms,&web_callbacks);}
