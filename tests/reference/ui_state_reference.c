/* SPDX-License-Identifier: GPL-3.0-only */
/* Exact R01 carrier/defaults against the active Rust ABI and initializer.
 * Each field is emitted independently; C/Rust padding is never serialized. */
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/rust_ui.h"
static int16_t fixture_defaults[EN_NFXS];
#define EN_FXS_DEF fixture_defaults
static struct {uint16_t pat;} GX_PRE[1];
#if EN_RUST_APP
static fm1_ui_state ui;
#else
#ifndef UI_STATE_SOURCE
#define UI_STATE_SOURCE "ui_state_frozen.inc"
#endif
#include UI_STATE_SOURCE
#endif
static uint32_t rs_state,callback_hash;
void en_fm_set_dy(int on){
 callback_hash=callback_hash*16777619u^(uint32_t)on;
 callback_hash=callback_hash*16777619u^ui.mood;
 callback_hash=callback_hash*16777619u^ui.fxs[95];
 callback_hash=callback_hash*16777619u^rs_state;
 ui.frame+=17u; /* callback can access state only after the Rust borrow returns */
}
#ifndef UI_INIT_SOURCE
#define UI_INIT_SOURCE "ui_init_frozen.inc"
#endif
#include UI_INIT_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static void word(uint32_t v){emit(&v,4);}
static uint32_t fields_hash(void){uint32_t h=2166136261u,i;
#define HASH_FIELD(name) {const uint8_t*p=(const uint8_t*)&ui.name;for(i=0;i<sizeof ui.name;i++)h=(h^p[i])*16777619u;}
 FM1_UI_FIELDS(HASH_FIELD)
#undef HASH_FIELD
 return h;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){uint32_t seed,i;
 struct align_probe{char prefix;__typeof__(ui) value;};
 uint32_t layout[FM1_UI_LAYOUT_WORDS+2],expected[FM1_UI_LAYOUT_WORDS],at=2;
 expected[0]=sizeof ui;expected[1]=offsetof(struct align_probe,value);
#define FIELD_LAYOUT(name) expected[at++]=offsetof(__typeof__(ui),name);expected[at++]=sizeof ui.name;
 FM1_UI_FIELDS(FIELD_LAYOUT)
#undef FIELD_LAYOUT
 if(at!=FM1_UI_LAYOUT_WORDS)return -1;
 emit(expected,sizeof expected);
#if EN_RUST_APP
 for(i=0;i<FM1_UI_LAYOUT_WORDS+2;i++)layout[i]=0xa5a5a5a5u;
 if(fm1_rust_ui_layout(&ui,layout+1,FM1_UI_LAYOUT_WORDS)!=FM1_UI_LAYOUT_WORDS)return -2;
 for(i=0;i<FM1_UI_LAYOUT_WORDS;i++)if(expected[i]!=layout[i+1])return -3;
 if(layout[0]!=0xa5a5a5a5u||layout[FM1_UI_LAYOUT_WORDS+1]!=0xa5a5a5a5u)return -3;
#else
 (void)layout;
#endif
 for(seed=0;seed<1024;seed++){
  memset(&ui,(int)(seed*53u),sizeof ui);rs_state=0xabcdefu;callback_hash=seed;
  for(i=0;i<EN_NFXS;i++)fixture_defaults[i]=(int16_t)(seed*137u+i*997u);
  GX_PRE[0].pat=(uint16_t)(seed*9973u);
#if EN_RUST_APP
  if(fm1_rust_ui_initialize(&ui,EN_FXS_DEF,GX_PRE[0].pat)<0)return -4;
  rs_state=0;en_fm_set_dy(1);
#else
  frozen_ui_init();
#endif
  word(seed);word(rs_state);word(callback_hash);
#define FIELD_BYTES(name) emit(&ui.name,sizeof ui.name);
  FM1_UI_FIELDS(FIELD_BYTES)
#undef FIELD_BYTES
 }
#if EN_RUST_APP
 for(i=0;i<FM1_UI_LAYOUT_WORDS+2;i++)layout[i]=0xa5a5a5a5u;
 if(fm1_rust_ui_layout(0,layout+1,FM1_UI_LAYOUT_WORDS)||fm1_rust_ui_layout(&ui,0,FM1_UI_LAYOUT_WORDS)||fm1_rust_ui_layout(&ui,layout+1,FM1_UI_LAYOUT_WORDS-1))return -5;
 for(i=0;i<FM1_UI_LAYOUT_WORDS+2;i++)if(layout[i]!=0xa5a5a5a5u)return -6;
 {uint32_t before=fields_hash();if(fm1_rust_ui_initialize(0,EN_FXS_DEF,0)!=-1||fm1_rust_ui_initialize(&ui,0,0)!=-1||fields_hash()!=before)return -8;}
 {extern uint32_t fm1_rust_jam_stat(uint32_t);if(!fm1_rust_jam_stat(82)||fm1_rust_jam_stat(83)!=5)return -7;}
#endif
 return 1024;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
