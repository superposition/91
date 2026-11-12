/* SPDX-License-Identifier: GPL-3.0-only */
/* Identical driver for verified R01, current C0 and the actual Rust adapters. */
#include <stdint.h>
#include <string.h>
#define FM1_APP_TEST 1
#ifndef __wasm__
#include <stdio.h>
#endif
static uint32_t syncs,blits,lease_bad;
static const uint16_t *lease;
static uint32_t lease_n,lease_hash;
static uint32_t bytes_hash(const void *p,uint32_t n){const uint8_t *b=p;uint32_t h=2166136261u;while(n--)h=(h^*b++)*16777619u;return h;}
static void lcd_sync(void){if(lease&&bytes_hash(lease,lease_n*2)!=lease_hash)lease_bad++;lease=0;syncs++;}
static void lcd_blit(uint32_t x,uint32_t y,uint32_t w,uint32_t h,const uint16_t*p){(void)x;(void)y;lcd_sync();lease=p;lease_n=w*h;lease_hash=bytes_hash(p,lease_n*2);blits++;}
#ifndef APP_GFX_SOURCE
#define APP_GFX_SOURCE "../../firmware/src/gfx.c"
#endif
#include APP_GFX_SOURCE
#ifndef APP_ALPHA_SOURCE
#define APP_ALPHA_SOURCE "app_alpha.inc"
#endif
#include APP_ALPHA_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void gfx_emit(const void*,uint32_t);
static void emit(const void*p,uint32_t n){gfx_emit(p,n);}
#else
static void emit(const void*p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint32_t cases;
static void witness(uint32_t kind,uint32_t id,int result){
 uint32_t head[13]={kind,id,cv_w,cv_h,(uint32_t)cv_oy,(uint32_t)cv_y0,(uint32_t)cv_y1,cv_scan,(uint32_t)result,blits,syncs,lease_bad,bytes_hash(cv_px,sizeof cv_px)};
 emit(head,sizeof head);emit(cv_px,cv_w*cv_h*2);cases++;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int gfx_run(void){
 static const uint16_t colors[]={0,0xffff,0xf800,0x7e0,0x1f,0x4b59};
 const felucca_font_t *fonts[5]={&FONT_XS,&FONT_S,&FONT_B,&FONT_M,&FONT_L};
 uint32_t i,j,k,n;char text[258];
 for(i=0;i<256;i++){palette_set(i);emit(pal,sizeof pal);}
 for(i=0;i<16;i++){
  uint32_t w=(uint32_t[]){0,1,31,32,240,241,320,640}[i%8],h=i&8?144:7;
  cv_begin(w,h,colors[i%6]);witness(0,i,0);
 }
 for(i=0;i<4096;i++){
  int x=(int)(i*37%97)-40,y=(int)(i*53%89)-32,w=(int)(i*17%83)-20,h=(int)(i*11%71)-16;
  cv_begin(48,40,colors[(i/6)%6]);cv_oy=(int)(i%17)-8;
  cv_y0=(int)(i%7);cv_y1=40-(int)(i%5);
  switch(i%4){case 0:cv_pset(x,y,colors[i%6]);break;case 1:cv_rect(x,y,w,h,colors[i%6]);break;case 2:cv_line(x,y,w,h,colors[i%6]);break;case 3:cv_band(4,31);cv_rect(x,y,w,h,colors[i%6]);break;}
  witness(1,i,0);
 }
 for(i=0;i<5;i++)for(j=0;j<256;j++)for(k=0;k<6;k++){
  int result;cv_begin(48,40,colors[(k+2)%6]);cv_oy=(int)(j%9)-4;cv_y0=2;cv_y1=38;
  text[0]=(char)j;text[1]=0;result=cv_text((int)(j%17)-3,(int)(j%11)-2,fonts[i],text,colors[k]);
  witness(2,(i*256+j)*6+k,result);result=text_w(fonts[i],text);emit(&result,sizeof result);
 }
 for(i=0;i<65536;i++){
  cv_begin(1,1,(uint16_t)(i*109u));px_blend(0,0,(uint16_t)i,i%32);witness(3,i,0);
 }
 for(i=0;i<512;i++){
  cv_begin(31,17,(uint16_t)(i*907u));cv_scan=(uint8_t)(i&1);cv_rect(3,2,11,9,(uint16_t)(i*911u));
  cv_scanlines(i/7,i%18);witness(4,i,0);
 }
 for(i=0;i<256;i++){
  cv_begin(32,24,colors[i%6]);cv_scan=(uint8_t)(i&1);cv_text(0,0,&FONT_XS,"A bC 0123?",colors[(i+1)%6]);
  cv_blit(4,8);lcd_sync();witness(5,i,0);
 }
 for(n=0;n<=256;n++){
  cv_begin(48,40,0);for(i=0;i<n;i++)text[i]=(char)(33+i%94);text[n]=0;
  i=(uint32_t)cv_text(-100,3,&FONT_XS,text,0xffff);witness(6,n,(int)i);
 }
 lcd_sync();{uint32_t end[3]={cases,lease_bad,CV_MAX};emit(end,sizeof end);}
#if EN_RUST_APP
 {extern uint32_t fm1_rust_jam_stat(uint32_t);if(!fm1_rust_jam_stat(80)||!fm1_rust_jam_stat(81)||fm1_rust_jam_stat(83))return -1;}
#endif
 return (int)cases;
}
#ifndef __wasm__
int main(void){return gfx_run()<0 || lease_bad!=0;}
#endif
