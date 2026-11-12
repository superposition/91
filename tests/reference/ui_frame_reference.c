/* SPDX-License-Identifier: GPL-3.0-only */
/* Exact frozen foreground function and production callback. All effects read
 * and mutate real UI fields, exercising ordering and access between borrows. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/rust_ui_controller.h"
static fm1_ui_state ui;
static uint32_t seed,now,buttons,keys,xruns,late,ui_splash_ms=3200,used,overflow;
static uint8_t record[4096];
static uint32_t events[110],reject_snapshot;
static en_jam_snapshot_t jam_view;
static struct{int16_t par[P_NPARAMS];}proj;
static struct{uint32_t on,lit_b,lit_k;}rm;
static struct{uint32_t on;}kr;
typedef struct{uint16_t bg;}theme_t;
static const theme_t THEMES[EN_NTHEME]={{1},{2},{3},{4},{5},{6}};
static const theme_t*th=THEMES;
#define K_BG (th->bg)
static uint8_t cv_scan;
volatile uint8_t en_playing,en_pat_next,en_jam_on,en_shed_level;
volatile uint32_t en_dd_changes;
static void bytes(const void*p,uint32_t n){if(n>sizeof record-used){overflow=1;return;}memcpy(record+used,p,n);used+=n;}
static void word(uint32_t n){uint8_t b[4]={(uint8_t)n,(uint8_t)(n>>8),(uint8_t)(n>>16),(uint8_t)(n>>24)};bytes(b,4);}
static void event(uint32_t op,uint32_t a,uint32_t b){events[op]++;word(op);word(a);word(b);word(ui.frame);word(ui.mood);word(ui.saver_on);}
uint32_t plat_ms(void){return now;}
uint32_t plat_buttons(void){return buttons;}
uint32_t plat_keys(void){return keys;}
uint32_t plat_xruns(void){return xruns;}
uint32_t plat_late(void){return late;}
void plat_glow(int on){event(101,(uint32_t)on,0);}
void plat_leds(uint32_t b,uint32_t k){event(102,b,k);}
void en_play(int on){event(103,(uint32_t)on,0);en_playing=(uint8_t)on;ui.mood++;}
void en_shed(int level){event(104,(uint32_t)level,0);en_shed_level=(uint8_t)level;ui.frame+=3;}
void en_set(int p,int value){event(105,(uint32_t)p,(uint32_t)value);ui.mood+=2;}
int en_jam_snapshot(en_jam_snapshot_t*out){(void)out;return !reject_snapshot;}
static void lcd_fill(uint32_t x,uint32_t y,uint32_t w,uint32_t h,uint16_t col){(void)x;(void)y;(void)w;(void)h;event(106,col,0);}
static void cv_blit(uint32_t x,uint32_t y){event(107,x,y);}
static void say(const char*a,const char*b){uint32_t h=2166136261u;while(a&&*a)h=(h^(uint8_t)*a++)*16777619u;while(b&&*b)h=(h^(uint8_t)*b++)*16777619u;event(108,h,0);ui.msg_until=now+17;}
#define SIMPLE(name,id) static void name(void){event(id,0,0);ui.mood++;}
SIMPLE(splash_tick,1)
static void store_tick(int force){event(2,(uint32_t)force,0);}
static void rm_tick(void){event(3,0,0);if(seed&32)rm.on=0;}
SIMPLE(jam_helper_sync,4)
static void input(void){event(5,0,0);if(seed&4){ui.view=(uint8_t)((ui.view+1)%43);ui.jam_voice=(uint8_t)((ui.jam_voice+1)&3);}if(seed&64)kr.on^=1;}
SIMPLE(scene_tick,6)
SIMPLE(dj_tick,7)
SIMPLE(af_tick,8)
SIMPLE(mix_tick,9)
SIMPLE(smp_tick,10)
SIMPLE(key_tick,11)
SIMPLE(glide_tick,12)
SIMPLE(saver_tick,13)
SIMPLE(jam_tick,14)
SIMPLE(autosave,15)
static void runner_update(void){event(16,0,0);if(seed&128)kr.on=0;}
static void runner_draw(int band){event(17,(uint32_t)band,0);}
SIMPLE(runner_leds,18)
SIMPLE(draw_header,19)
static void draw_main(void){event(20,0,0);if(seed&256)ui.saver_on^=1;}
SIMPLE(draw_knobs,21)
SIMPLE(leds,22)
#include "ui_frame_frozen.inc"
#if EN_RUST_APP
#include "ui_frame_port.inc"
static fm1_ui_frame_state frame_state;
#endif
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){uint32_t i,j;const uint32_t jumps[]={0,1,3999,4000,4001,9999,10000,10001,19999,20000,20001,0xfffffff0u};
 now=0xffff0000u;
 for(i=0;i<8192;i++){
  seed=i;used=0;word(i);now+=jumps[i%12];
  memset(&ui,(int)(i*137u),sizeof ui);memset(&proj,0,sizeof proj);
  ui.splash=(i%4)==0;ui.boot_t=now-(i%6400);ui.theme=(uint8_t)(i%9);ui.frame=i%13==0?0xffffffffu:i;
  ui.view=(uint8_t)(i%NVIEWS);ui.jam_man=(uint8_t)(i%5);ui.jam_voice=(uint8_t)(i%8);ui.saver_on=(i/3)&1;ui.msg_until=now+(i&1?10u:0u);
  proj.par[P_THEME]=(int16_t)((int)(i%10)-2);
  for(j=0;j<P_NPARAMS;j++)if(j!=P_THEME)proj.par[j]=(int16_t)(j*17u+i);
  buttons=i%8==0?0:1u<<(i%14);keys=i%16==0?0:1u<<(i%27);
  en_playing=(i>>1)&1;en_pat_next=(uint8_t)(i%6);en_dd_changes=i/5;en_shed_level=i%4;en_jam_on=i%3==0;
  xruns=i/17;late=i/3;rm.on=i%11==0;rm.lit_b=i;rm.lit_k=i*3;kr.on=i%7==0;
  th=&THEMES[i%EN_NTHEME];cv_scan=0;
#if EN_RUST_APP
  if(fm1_rust_ui_frame(&ui,&frame_state,ui_frame_port,(uint32_t)V_JAM|((uint32_t)V_JAMHELP<<8)|((uint32_t)V_JAMARR<<16)|((uint32_t)V_JAMSWELL<<24))<0)return -1;
#else
  reference_frame();
#endif
#define F(name) bytes(&ui.name,sizeof ui.name);
  FM1_UI_FIELDS(F)
#undef F
  word((uint32_t)(th-THEMES));word((uint32_t)(int32_t)proj.par[P_PATTERN]);word(en_shed_level);word(en_playing);word(cv_scan);word(rm.on);word(kr.on);
  if(overflow)return -2;emit(record,used);
 }
 if(!events[1]||!events[3]||!events[16]||!events[17]||!events[19]||!events[21]||!events[104]||!events[105])return -7;
#if EN_RUST_APP
 {uint32_t saved=used;
  if(fm1_rust_ui_frame(0,&frame_state,ui_frame_port,0)!=-1||fm1_rust_ui_frame(&ui,0,ui_frame_port,0)!=-1||fm1_rust_ui_frame(&ui,&frame_state,0,0)!=-1||saved!=used)return -3;
  extern uint32_t fm1_rust_jam_stat(uint32_t);if(fm1_rust_jam_stat(83)!=3||fm1_rust_jam_stat(82)!=8192)return -4;
 }
 /* Current handoff extension: retain the last picture if capture is refused. */
 memset(&ui,0,sizeof ui);memset(events,0,sizeof events);used=0;seed=0;
 ui.view=V_JAMHELP;proj.par[P_THEME]=0;rm.on=kr.on=en_playing=0;en_jam_on=1;reject_snapshot=1;
 if(fm1_rust_ui_frame(&ui,&frame_state,ui_frame_port,(uint32_t)V_JAM|((uint32_t)V_JAMHELP<<8)|((uint32_t)V_JAMARR<<16)|((uint32_t)V_JAMSWELL<<24))<0)return -5;
 if(ui.frame!=1||events[19]||events[20]||events[21]||events[22]!=1)return -6;
#endif
 return 8192;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
