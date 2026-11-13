/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/rust_ui_buttons.h"
static fm1_ui_state ui;
static uint32_t seed,now,used,overflow,events[220],hold;
static uint8_t record[4096];
static struct{int16_t par[P_NPARAMS];}proj;
volatile uint8_t en_playing,en_frozen,en_jam_on,en_sec,en_sec_req;
#define NFXP 35
#define VZ_N 10
static const char*VZ_NAME[VZ_N]={"PLASMA","KALEIDO","BARS","FLUX","DISCO","LIFE","ORBIT","TRIP","THEORY","AUTO"};
static struct{char name[8];}FXP[NFXP];
static void bytes(const void*p,uint32_t n){if(n>sizeof record-used){overflow=1;return;}memcpy(record+used,p,n);used+=n;}
static void word(uint32_t n){uint8_t b[4]={(uint8_t)n,(uint8_t)(n>>8),(uint8_t)(n>>16),(uint8_t)(n>>24)};bytes(b,4);}
static void event(uint32_t op,uint32_t a,uint32_t b){events[op]++;word(op);word(a);word(b);word(ui.frame);word(ui.home_t);word(ui.fx_menu);}
uint32_t plat_ms(void){return now;}
static void say(const char*a,const char*b){uint32_t h=2166136261u;const char*original=a;while(a&&*a)h=(h^(uint8_t)*a++)*16777619u;while(b&&*b)h=(h^(uint8_t)*b++)*16777619u;event(201,h,0);ui.msg_until=now+37;
 if((seed&16)&&!strcmp(original,"FREEZE"))en_frozen^=1;
}
void en_play(int on){event(202,(uint32_t)on,0);en_playing=(uint8_t)on;ui.frame++;}
void en_seed(uint32_t value){event(203,value,0);ui.mood++;}
void en_next(void){event(204,0,0);if(seed&8)en_playing^=1;}
void en_freeze(int on){event(205,(uint32_t)on,0);en_frozen=(uint8_t)on;}
void en_section(int cmd){event(206,(uint32_t)cmd,0);en_sec_req=(uint8_t)(cmd+1);}
void en_jam_get(jp_settings_t*s){memset(s,0,sizeof *s);s->hold=(uint8_t)hold;}
/* The frozen button corpus predates Synth Jam; its new guard must stay off. */
int en_jam_option(int field){if(field!=JAM_SYNTH)__builtin_trap();return 0;}
void en_jam_set(int field,int value){event(207,(uint32_t)field,(uint32_t)value);if(field==JAM_HOLD)hold=(uint32_t)value;ui.mood++;}
void en_param_text(int p,int value,char*b){uint32_t x=(uint32_t)value;(void)p;b[0]=(char)('A'+x%26);b[1]=(char)('A'+(x>>8)%26);b[2]=0;}
static int jam_session_save(void){event(208,0,0);ui.frame+=2;return seed&32?PLAT_STORE_BUSY:0;}
static void set_view(int v){event(209,(uint32_t)v,0);ui.view=(uint8_t)v;if(v<35)ui.last_dial=(uint8_t)v;if(seed&64)ui.home_t+=17;}
static void scene_reset(void){event(210,0,0);ui.fx_lock=0;ui.fx_menu=0;ui.mood++;}
static void knob_set(int p,int value){event(211,(uint32_t)p,(uint32_t)value);proj.par[p]=(int16_t)value;ui.frame++;}
#ifndef UI_BUTTON_SOURCE
#define UI_BUTTON_SOURCE "ui_button_frozen.inc"
#endif
#include UI_BUTTON_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t fields_hash(void){uint32_t h=2166136261u,i;
#define H(name) {const uint8_t*p=(const uint8_t*)&ui.name;for(i=0;i<sizeof ui.name;i++)h=(h^p[i])*16777619u;}
 FM1_UI_FIELDS(H)
#undef H
 return h;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){unsigned v,b,flags,time,i,n=0;const uint32_t deltas[]={0,1,249,250,349,350,449,450,451,900,0xfffffff0u,0xffffffffu};
 for(i=0;i<NFXP;i++){FXP[i].name[0]='F';FXP[i].name[1]='X';FXP[i].name[2]=(char)('0'+i/10);FXP[i].name[3]=(char)('0'+i%10);FXP[i].name[4]=0;}
 for(v=0;v<NVIEWS;v++)for(b=0;b<NB;b++)for(flags=0;flags<4;flags++)for(time=0;time<12;time++){
  seed=n;used=0;word(n++);memset(&ui,(int)(seed*137u),sizeof ui);
  now=time%3?0xffffff00u:0;ui.view=(uint8_t)v;ui.fx_menu=flags&1;ui.fx_page=flags&2?(uint8_t)(seed%35+1):0;
  ui.fx_lastpg=(uint8_t)(seed%36);ui.home_t=flags&2?now-deltas[time]:0;ui.fx_open_t=flags&1?now-deltas[time]:0;
  for(i=0;i<8;i++)ui.grp_last[i]=(uint8_t)(seed*17u+i);ui.lane=(uint8_t)seed;ui.viz=(uint8_t)seed;
  for(i=0;i<P_NPARAMS;i++)proj.par[i]=(int16_t)(seed*31u+i);
  en_playing=seed&1;en_jam_on=(seed>>2)&1;en_frozen=(seed>>3)&1;en_sec=(seed>>4)%3;en_sec_req=(seed>>6)%4;hold=(seed>>7)&1;
  button((int)b);
#define F(name) bytes(&ui.name,sizeof ui.name);
  FM1_UI_FIELDS(F)
#undef F
  word(en_playing);word(en_frozen);word(en_sec_req);word(hold);
  for(i=0;i<P_NPARAMS;i++)bytes(&proj.par[i],sizeof proj.par[i]);
  if(overflow)return -1;emit(record,used);
 }
 if(!events[201]||!events[202]||!events[203]||!events[204]||!events[205]||!events[206]||!events[207]||!events[208]||!events[209]||!events[210]||!events[211])return -2;
#if EN_RUST_APP
 {uint32_t before=fields_hash(),saved=used;
  if(fm1_rust_ui_button(0,0,ui_button_port)!=-1||fm1_rust_ui_button(&ui,0,0)!=-1||fm1_rust_ui_button(&ui,14,ui_button_port)!=-1||fm1_rust_ui_button(&ui,0xffffffffu,ui_button_port)!=-1||fields_hash()!=before||used!=saved)return -3;
  extern uint32_t fm1_rust_jam_stat(uint32_t);if(fm1_rust_jam_stat(83)!=4||fm1_rust_jam_stat(82)!=n)return -4;
 }
 memset(&ui,0,sizeof ui);used=0;en_jam_on=0;ui.fx_menu=1;ui.fx_open_t=now-1;ui.fx_lastpg=255;
 if(fm1_rust_ui_button(&ui,B_FX,ui_button_port)<0||ui.fx_page!=1)return -5;
#endif
 return (int)n;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
