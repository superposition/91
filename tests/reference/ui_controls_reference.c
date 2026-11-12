/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/rust_ui.h"
static fm1_ui_state ui;
static uint32_t now;
static uint8_t accel_off;
uint32_t plat_ms(void){return now;}
#ifndef UI_ACCEL_SOURCE
#define UI_ACCEL_SOURCE "ui_accel_frozen.inc"
#endif
#include UI_ACCEL_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t field_hash(void){uint32_t h=2166136261u,i;
#define F(name) {const uint8_t*p=(const uint8_t*)&ui.name;for(i=0;i<sizeof ui.name;i++)h=(h^p[i])*16777619u;}
 FM1_UI_FIELDS(F)
#undef F
 return h;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){unsigned role,wrap,dt,step,range,run,off,i,n=0;
 static const uint32_t deltas[]={0,1,11,12,24,25,44,45,59,60,100,1000,65535,0xffffffffu};
 static const int32_t steps[]={-65535,-256,-127,-16,-8,-3,-2,-1,0,1,2,3,127,255,256,65535,268435455,-268435456};
 static const int ranges[]={-1,0,24,25,99,100,255};
 static const uint8_t runs[]={0,1,3,4,254,255};
 for(role=0;role<7;role++)for(wrap=0;wrap<2;wrap++)for(dt=0;dt<sizeof deltas/sizeof deltas[0];dt++)for(step=0;step<sizeof steps/sizeof steps[0];step++)for(range=0;range<7;range++)for(run=0;run<6;run++)for(off=0;off<2;off++){
  uint32_t words[10];int32_t result;memset(&ui,(int)(n*137u),sizeof ui);
  now=wrap?0xfffffff0u:500;accel_off=(uint8_t)off;
  for(i=0;i<7;i++){ui.enc_t[i]=n*17u+i;ui.enc_run[i]=(uint8_t)(n+i);}
  ui.enc_t[role]=now-deltas[dt];ui.enc_run[role]=runs[run];
  result=accel((int)role,steps[step],ranges[range]);
  words[0]=n++;words[1]=(uint32_t)result;words[2]=field_hash();for(i=0;i<7;i++)words[i+3]=ui.enc_t[i];
  emit(words,sizeof words);emit(ui.enc_run,sizeof ui.enc_run);
 }
#if EN_RUST_APP
 {int32_t value=1234;uint32_t before=field_hash();
  if(fm1_rust_ui_accel(0,0,1,100,0,0,&value)!=-1||fm1_rust_ui_accel(&ui,0,1,100,0,0,0)!=-1||fm1_rust_ui_accel(&ui,7,1,100,0,0,&value)!=-1||fm1_rust_ui_accel(&ui,0xffffffffu,1,100,0,0,&value)!=-1||field_hash()!=before||value!=1234)return -1;
  ui.enc_t[0]=0;ui.enc_run[0]=4;before=field_hash();
  if(fm1_rust_ui_accel(&ui,0,0x7fffffff,255,1,0,&value)!=-1||fm1_rust_ui_accel(&ui,0,(-0x7fffffff-1),255,1,0,&value)!=-1||field_hash()!=before||value!=1234)return -2;
  extern uint32_t fm1_rust_jam_stat(uint32_t);if(fm1_rust_jam_stat(83)!=6||fm1_rust_jam_stat(82)!=n)return -3;
 }
#endif
 return (int)n;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
