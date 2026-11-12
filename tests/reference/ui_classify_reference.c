/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#include "../../firmware/src/dsp/amb.h"
#include "../../firmware/src/app/plat.h"
#include "../../firmware/src/dsp/rust_ui.h"
#ifndef UI_CLASSIFY_SOURCE
#define UI_CLASSIFY_SOURCE "ui_classify_frozen.inc"
#endif
#include UI_CLASSIFY_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){int32_t n,v;for(n=0;n<770;n++){
 v=n==768?(-0x7fffffff-1):n==769?0x7fffffff:n-256;
 uint32_t record[5]={(uint32_t)v,(uint32_t)voice_of(v),(uint32_t)kit_view(v),(uint32_t)oct_par(v),(uint32_t)group_of(v)};
 emit(record,sizeof record);
 }
#if EN_RUST_APP
 for(n=4;n<100;n++)if(fm1_rust_ui_classify((uint32_t)n,n)!=-1)return -1;
 if(fm1_rust_ui_classify(0xffffffffu,0)!=-1)return -2;
#endif
 return 770;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
