/* SPDX-License-Identifier: GPL-3.0-only */
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
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit")))extern void state_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){state_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int state_run(void)
{
 unsigned n,i;static int32_t pcm[512];char current[32],next[32];
 for(i=0;i<OBJ_NOBJ;i++)store_len[i]=-1;
 ui_splash_ms=0;boot();
 for(n=0;n<64;n++){
  en_jam_set(JAM_ON,n&1u);en_jam_set(JAM_KEY,(int)(n%24));en_jam_set(JAM_HOLD,(n>>2)&1u);
  en_play((n>>1)&1u);set_view(V_JAMHELP);
  now_ms+=31;
  for(i=0;i<4;i++)en_render(pcm,256,2048);
  memset(current,0,sizeof current);memset(next,0,sizeof next);
  en_jam_name(0,current);en_jam_name(1,next);
#if EN_RUST_HANDOFF
  {en_jam_snapshot_t s;
   if(!en_jam_snapshot(&s) || strcmp(current,s.chord) || strcmp(next,s.next) ||
      s.on!=en_jam_on || s.playing!=en_playing || s.chord[31] || s.next[31])return -1;
  }
#endif
  ui_frame();
  {uint8_t flags[4]={(uint8_t)n,en_jam_on,en_playing,0};emit(flags,4);}
  emit(current,sizeof current);emit(next,sizeof next);emit(pcm,sizeof pcm);emit(fb,sizeof fb);
 }
 return 64;
}
#ifndef __wasm__
int main(void){return state_run()<0;}
#endif
