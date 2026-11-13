/* SPDX-License-Identifier: GPL-3.0-only */
/* Complete actual app/engine consumers driven by a deterministic panel corpus.
 * Every object/field is captured explicitly, never C/Rust structure padding. */
#include <stdint.h>
#define EN_WEB 1
static void web_audio(const int32_t*p,uint32_t n){(void)p;(void)n;}
#define main unused_host_parser_main
#ifndef UI_INPUT_HOST
#define UI_INPUT_HOST "ui_input_current.inc"
#endif
#include UI_INPUT_HOST
#undef main
#undef __attribute__
#include "../../firmware/src/dsp/rust_ui_state.h"
static uint8_t record[16384];static uint32_t used,overflow;
static void bytes(const volatile void*ptr,uint32_t n){const volatile uint8_t*p=ptr;if(n>sizeof record-used){overflow=1;return;}while(n--)record[used++]=*p++;}
static void word(uint32_t n){uint8_t b[4]={(uint8_t)n,(uint8_t)(n>>8),(uint8_t)(n>>16),(uint8_t)(n>>24)};bytes(b,4);}
static uint32_t input_hash(uint32_t h,const void*ptr,uint32_t n){const uint8_t*p=ptr;while(n--)h=(h^*p++)*16777619u;return h;}
#define RAW(x) bytes(&(x),sizeof(x))
static void capture_project(void){unsigned s;
 RAW(proj.par);RAW(proj.pool);RAW(proj.acl);RAW(proj.cml);RAW(proj.prg);RAW(proj.prk);RAW(proj.bs);
 RAW(proj.leds);RAW(proj.piece);RAW(proj.genre);RAW(proj.edited);
 for(s=0;s<NPIECES;s++){
  RAW(proj.slot[s].par);RAW(proj.slot[s].pool);RAW(proj.slot[s].pats);RAW(proj.slot[s].mod);
  RAW(proj.slot[s].acl);RAW(proj.slot[s].cml);RAW(proj.slot[s].prg);RAW(proj.slot[s].prk);RAW(proj.slot[s].bs);
 }
 RAW(glob.magic);RAW(glob.tune);RAW(glob.midi);RAW(glob.theme);RAW(glob.rsv);RAW(glob.sl);RAW(glob.sl_on);RAW(glob.sl_rsv);
 RAW(en_pats);word((uint32_t)project_pending());word((uint32_t)project_pending_globals());
}
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t random_word(uint32_t*s){*s=*s*1664525u+1013904223u;return *s;}
#ifdef UI_PERFORMANCE_CORPUS
#include "ui_performance_cases.inc"
#endif
#ifdef UI_AUTOMATIC_CORPUS
#include "ui_automatic_cases.inc"
#endif
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void){unsigned n,i,phase;uint32_t random=91;static int32_t pcm[512];
 const uint32_t delta[]={0,1,11,12,24,25,44,45,249,250,349,350,399,400,449,450,499,500,599,600,899,900,1499,1500};
 const uint32_t masks[]={0,1u<<B_HOME,1u<<B_ENV,1u<<B_LFO,1u<<B_GLO,1u<<B_ARP,1u<<B_SEQ,1u<<B_PLAY,1u<<B_REC,
  1u<<B_FX,1u<<B_SAVE,1u<<B_EDIT,1u<<B_OCTDN,1u<<B_OCTUP,(1u<<B_SEL)|(1u<<B_FX),(1u<<B_SEL)|(1u<<B_REC),
  (1u<<B_SEL)|(1u<<B_ENV),(1u<<B_SEL)|(1u<<B_SEQ),(1u<<B_SEL)|(1u<<B_EDIT),(1u<<B_SEL)|(1u<<B_OCTUP),
  (1u<<B_GLO)|(1u<<B_SEL),(1u<<B_HOME),(1u<<B_SEL),(1u<<B_GLO)};
 for(i=0;i<OBJ_NOBJ;i++)store_len[i]=-1;
 ui_splash_ms=0;boot();en_jam_set(JAM_ON,0);en_render(pcm,256,2048);
 for(n=0;n<4096;n++){
  used=0;word(n);phase=n%64;now_ms+=delta[n%24];
  if(phase==0){
   held_btn=held_keys=0;ui_input_only();
   ui.view=(uint8_t)((n/64)%NVIEWS);ui.fx_menu=(n/64)%3==0;ui.fx_page=ui.fx_menu?(uint8_t)((n/64)%36):0;
   ui.saver_on=(n/64)%7==0;rm.on=0;
   en_jam_set(JAM_ON,ui.view>=V_JAMHELP);en_render(pcm,256,2048);
  }
  if(phase%4==0)held_btn=masks[(random_word(&random)>>16)%(sizeof masks/sizeof masks[0])];
  if(phase%4==3)held_btn=0;
  if(phase%5==0)held_keys=random_word(&random)&((1u<<NKEYS)-1u);
  if(phase%5==3)held_keys=0;
  for(i=0;i<NE;i++)if((random_word(&random)&7u)==0)enc_acc[i]=(int32_t)((random_word(&random)>>16)%33u)-16;
#ifdef UI_PERFORMANCE_CORPUS
  performance_case(n,&random);
#endif
#ifdef UI_AUTOMATIC_CORPUS
  automatic_case(n,&random,pcm);
#endif
#if EN_RUST_APP
  {const uint32_t calls=fm1_rust_jam_stat(82);
   ui_input_only();
   if(n==0 && fm1_rust_jam_stat(82)==calls)return -2; /* real input consumer witness */
  }
#else
  ui_input_only();
#endif
  en_render(pcm,256,2048);RAW(pcm);
  ui_frame();en_render(pcm,256,2048);RAW(pcm);
#define UI_FIELD(name) RAW(ui.name);
  FM1_UI_FIELDS(UI_FIELD)
#undef UI_FIELD
  capture_project();RAW(held_btn);RAW(held_keys);RAW(enc_acc);RAW(lit_btn);RAW(lit_keys);
  RAW(en_playing);RAW(en_frozen);RAW(en_sec);RAW(en_sec_req);RAW(en_drum_pos);RAW(en_dd_changes);
  RAW(en_jam_on);RAW(en_jam_bar);RAW(en_jam_beat);RAW(en_jam_section);RAW(en_jam_pending);RAW(en_jam_length);RAW(en_jam_revision);
  RAW(store_len);RAW(store_lock);RAW(store_writes);RAW(rm.on);RAW(rm.lit_b);RAW(rm.lit_k);
  word(input_hash(2166136261u,store,sizeof store));word(input_hash(2166136261u,fb,sizeof fb));
  if(overflow)return -1;
#ifdef UI_CAPTURE_FRAME
  if(n==UI_CAPTURE_FRAME){emit(fb,sizeof fb);return (int)n+1;}
#else
  emit(record,used);
#endif
 }
#ifdef UI_AUTOMATIC_CORPUS
 if(automatic_seen!=automatic_required)return -4;
#endif
 return 4096;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
