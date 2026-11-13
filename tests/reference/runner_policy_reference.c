/* SPDX-License-Identifier: GPL-3.0-only */
/* Full fixed game state/callback policy, not a rewritten C implementation. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#define FM1_APP_TEST 1
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/fastmath.h"
static uint32_t now,callback_hash=2166136261u,callback_count,reenter;
static int32_t encoders[NE];
volatile uint32_t en_bars;
volatile uint8_t en_playing;
volatile float en_drum_glow[EN_NDRUM];
static void lcd_sync(void){}
static void lcd_blit(uint32_t x,uint32_t y,uint32_t w,uint32_t h,const uint16_t*p){(void)x;(void)y;(void)w;(void)h;(void)p;}
static void lcd_fill(uint32_t,uint32_t,uint32_t,uint32_t,uint16_t);
#include "../../firmware/src/gfx.c"
static struct {uint64_t fx_mask;uint32_t trk_m,trk_s,fx_menu,help,sig[3],act_t,frame,btn,keys,shift_keys,btn_used,down_t[NB];} ui;
static const uint8_t WHITE_KEY[16]={0,2,4,6,7,9,11,12,14,16,18,19,21,23,24,26};
static const uint8_t BLACK_KEY[11]={1,3,5,8,10,13,15,17,20,22,25};
#define K_BG 0x1234u
static uint16_t theme_colors[4]={0xabdc,0xffff,0x7bef,0x5abf};
#define K_PULSE theme_colors[0]
#define K_TEXT theme_colors[1]
#define K_DIM theme_colors[2]
#define K_ACC theme_colors[3]
/* Drawing is deliberately not invoked by this policy driver. Actual game
 * framebuffer output is covered separately by native/scenario/WASM rendering. */
#ifdef RUNNER_DRAW_TEST
#ifndef APP_RUNNER_WIDGETS
#define APP_RUNNER_WIDGETS "runner_widgets.inc"
#endif
#include APP_RUNNER_WIDGETS
#else
static void cv_round(int32_t x,int32_t y,int32_t w,int32_t h,int32_t r,uint16_t c){(void)x;(void)y;(void)w;(void)h;(void)r;(void)c;}
static void text_c(int32_t x,int32_t y,const felucca_font_t*f,const char*s,uint16_t c){(void)x;(void)y;(void)f;(void)s;(void)c;}
static uint16_t mix(uint16_t a,uint16_t b,int t){(void)b;(void)t;return a;}
static int q16(float x){return (int)(x*16);}
static void itoa_u(uint32_t v,char*s){(void)v;s[0]=0;}
static void px_blend(int32_t x,int32_t y,uint16_t c,uint32_t a){(void)x;(void)y;(void)c;(void)a;}
#endif
static void midi_in(void);
/* This frozen runner corpus has no Synth Jam owners. The actual full-app
 * Synth takeover test covers release; calling it here would change the corpus. */
int en_jam_option(int field){if(field!=JAM_SYNTH)__builtin_trap();return 0;}
void en_jam_release(void){__builtin_trap();}
#ifndef APP_RUNNER_SOURCE
#define APP_RUNNER_SOURCE "../../firmware/src/app/runner.c"
#endif
#include APP_RUNNER_SOURCE
static void record(uint32_t kind,uint32_t a,uint32_t b){callback_hash=(callback_hash^kind)*16777619u;callback_hash=(callback_hash^a)*16777619u;callback_hash=(callback_hash^b)*16777619u;callback_count++;}
uint32_t plat_ms(void){record(1,now,0);return now;}
int32_t plat_enc(int role){int32_t v=encoders[role];encoders[role]=0;record(2,(uint32_t)role,(uint32_t)v);return v;}
void en_fx(uint64_t mask){record(3,(uint32_t)mask,(uint32_t)(mask>>32));if(reenter)kr.best+=17u;}
void en_trk(uint32_t mute,uint32_t solo){record(4,mute,solo);if(reenter)kr.best^=0x01020304u;}
void en_play(int on){record(5,(uint32_t)on,0);en_playing=(uint8_t)on;if(reenter)kr.score+=3u;}
void plat_glow(int on){record(6,(uint32_t)on,0);}
void plat_leds(uint32_t buttons,uint32_t keys){record(7,buttons,keys);}
static void midi_in(void){record(8,ui.btn,ui.keys);}
static void lcd_fill(uint32_t x,uint32_t y,uint32_t w,uint32_t h,uint16_t c){record(9,x^y,w^h);record(10,c,0);}
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void policy_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){policy_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t cases;
static void word(uint32_t value){emit(&value,4);}
static void real(float value){uint32_t bits;memcpy(&bits,&value,4);word(bits);}
static void witness(uint32_t id){uint32_t i;
 word(id);word(callback_hash);word(callback_count);
 word(kr.on);word(kr.over);word(kr.lane);word(kr.layer);word(kr.ships);
 real(kr.start_beat);real(kr.view);real(kr.beat);word(kr.s16);word(kr.s16_ms);word(kr.step_ms);
 real(kr.kick_prev);real(kr.kick);real(kr.lp_until);real(kr.flash);real(kr.crash);
 word(kr.score);word(kr.best);word(kr.gates);word(kr.crashes);word(kr.rows_born);
 word(kr.banner_until);word(kr.start_ms);word(kr.over_ms);word(kr.rng);emit(kr.banner,20);
 for(i=0;i<16;i++){real(kr.row[i].beat);word(kr.row[i].wall);word(kr.row[i].gate);word(kr.row[i].done);word(kr.row[i].taken);}
 word((uint32_t)ui.fx_mask);word((uint32_t)(ui.fx_mask>>32));word(ui.trk_m);word(ui.trk_s);
 word(ui.fx_menu);word(ui.help);word(ui.act_t);word(ui.frame);word(ui.btn);word(ui.keys);word(ui.shift_keys);word(ui.btn_used);
 for(i=0;i<3;i++)word(ui.sig[i]);for(i=0;i<NB;i++)word(ui.down_t[i]);for(i=0;i<NE;i++)word((uint32_t)encoders[i]);cases++;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int policy_run(void){uint32_t i,j,k,l,seed=0x38ac27d1u;
#ifdef RUNNER_DRAW_TEST
 for(i=0;i<512;i++){
  memset(&kr,0,sizeof kr);now=10000+i*100;en_playing=(uint8_t)(i&1);kr.on=1;kr.over=(uint8_t)(i%5==0);
  kr.lane=(uint8_t)(i%8);kr.layer=(uint8_t)(i/8%6);kr.ships=(uint8_t)(i/48%4);kr.beat=(float)i*0.0625f;kr.view=(float)(i%64)*0.03125f-1.0f;
  kr.score=i*17777u%1000000u;kr.best=i*7213u%1000000u;kr.start_ms=now-(i%3?4000u:5000u);
  kr.kick=(float)(i%7)/7.0f;kr.flash=(float)(i%11)/11.0f;kr.crash=(float)(i%13)/13.0f;
  kr_say("HATS - FULL TRACK",1400);if(i%3==0)kr.banner_until=now-1;
  for(j=0;j<16;j++){kr.row[j].beat=kr.beat+(float)((int)(j%12)-2)*0.7f;kr.row[j].wall=(uint8_t)(i*37+j*61);kr.row[j].gate=(uint8_t)(i*29+j*41);kr.row[j].taken=(uint8_t)((i+j)%3==0);}
  for(j=0;j<4;j++)theme_colors[j]=(uint16_t)(i*3123u+j*9711u);cv_scan=(uint8_t)(i&1);
  for(j=0;j<2;j++){
   runner_draw((int)j);word(i);word(j);word(cv_w);word(cv_h);word((uint32_t)cv_oy);word((uint32_t)cv_y0);word((uint32_t)cv_y1);word(cv_scan);
   emit(cv_px,cv_w*cv_h*2);cv_blit(0,j*120);cases++;
  }
 }
#else
 for(i=0;i<512;i++){
  memset(&kr,0,sizeof kr);memset(&ui,0,sizeof ui);now=i*7919u;en_bars=i*17u;en_playing=(uint8_t)(i&1);reenter=i&1;
  kr.rng=i*2654435761u;kr.best=i*43u;kr.rows_born=i*103u;kr.step_ms=i%3?0:137u;
  ui.fx_mask=((uint64_t)0xfedcba98u<<32)|i;ui.trk_m=i;ui.trk_s=i>>1;ui.fx_menu=ui.help=1;
  runner_start();runner_leds();witness(i);runner_stop();witness(i|0x10000u);
 }
 reenter=0;
 for(i=0;i<8;i++)for(j=0;j<6;j++)for(k=0;k<4;k++)for(l=0;l<8;l++){
  memset(&kr,0,sizeof kr);kr.on=1;kr.lane=(uint8_t)i;kr.layer=(uint8_t)j;kr.ships=(uint8_t)k;
  kr.beat=32;kr.s16=128;kr.step_ms=125;kr.view=0.1875f;kr.score=l&1?0xfffffffdu:123u;kr.best=42;
  now=1000;en_bars=128;en_playing=0;kr.s16_ms=1000;
  kr.row[0].beat=32;kr.row[0].wall=(uint8_t)(l&1?1u<<i:0);kr.row[0].gate=(uint8_t)(l&2?1u<<i:0);kr.row[0].done=(uint8_t)((l>>2)&1);
  runner_update();runner_leds();witness(0x20000u+cases);
 }
 memset(&kr,0,sizeof kr);memset(&ui,0,sizeof ui);en_bars=0;now=0;runner_start();
 for(i=0;i<65536;i++){
  seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;now+=13u+(seed&15u);en_bars=now/125u;en_playing=(uint8_t)(i%101u!=0);
  en_drum_glow[0]=(i%9u==0)?1.0f:(i%9u==1?0.8f:0.2f);for(j=1;j<EN_NDRUM;j++)en_drum_glow[j]=(float)((i+j)%7u)*0.1f;
  encoders[EN_SELECT]=(i%17u==0)?(i&1?-0x7fffffff:0x7fffffff):0;
  for(j=1;j<NE;j++)encoders[j]=(int32_t)(seed>>j);
  reenter=i%331u==0;ui.frame=i;
  runner_input((i%59u==0?1u<<B_OCTDN:0)|(i%61u==0?1u<<B_OCTUP:0),i%71u==0?1u<<3:0);
  runner_update();runner_leds();witness(0x40000u+i);
  if(kr.over&&i%19u==0){now+=901;runner_input(0,1u<<5);runner_input(0,0);}
 }
 for(i=0;i<4096;i++){
  now=0xfffffff0u+i;en_bars=0xfffffff0u+(i/16);en_playing=(uint8_t)(i&1);kr.s16_ms=now-125;kr.s16=en_bars-1;
  kr.ships=3;kr.layer=(uint8_t)(i%6);kr.lane=(uint8_t)(i%8);kr.over=0;kr.on=1;kr.beat=(float)en_bars*0.25f;
  kr.lp_until=kr.beat+(float)((int)(i%3)-1);en_drum_glow[0]=(i&1)?1.0f:0.0f;ui.frame=i;
  runner_update();kr_fx();runner_leds();witness(0x50000u+i);
 }
#endif
#if EN_RUST_APP
 {extern uint32_t fm1_rust_jam_stat(uint32_t);if(!fm1_rust_jam_stat(82)||fm1_rust_jam_stat(83))return -1;}
#endif
 return (int)cases;
}
#ifndef __wasm__
int main(void){return policy_run()<0;}
#endif
