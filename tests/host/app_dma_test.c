/* SPDX-License-Identifier: GPL-3.0-only */
/* Read-retaining LCD double: sync must precede the next Rust pixel borrow. */
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#ifndef EN_RUST_APP
#define EN_RUST_APP 1
#endif
static const uint16_t *leased;
static uint32_t count,sync_count,expected,fail_ready;
static uint32_t sum(void){uint32_t i,h=2166136261u;for(i=0;i<count;i++)h=(h^leased[i])*16777619u;return h;}
static void lcd_sync(void){sync_count++;if(leased&&sum()!=expected){puts("pixels changed before DMA ended");__builtin_trap();}leased=0;}
static int lcd_ready(void){if(fail_ready)return 0;lcd_sync();return 1;}
#define LCD_READY() lcd_ready()
static void lcd_blit(uint32_t x,uint32_t y,uint32_t w,uint32_t h,const uint16_t*p){(void)x;(void)y;lcd_sync();leased=p;count=w*h;expected=sum();}
#include "../../firmware/src/gfx.c"
#include "app_alpha.inc"
int main(void){uint32_t before;
 cv_begin(8,8,0);cv_rect(0,0,8,8,0x1234);cv_blit(0,0);before=sync_count;
 (void)text_w(&FONT_XS,"READ ONLY");if(sync_count!=before||!leased)return 1;
 cv_band(0,8);if(sync_count!=before||!leased)return 2;
 cv_pset(0,0,0xffff);if(sync_count!=before+1||leased)return 3;
 cv_blit(0,0);before=sync_count;cv_scan=0;cv_scanlines(0,0);if(sync_count!=before||!leased)return 4;
 cv_scan=1;cv_scanlines(0,0);if(sync_count!=before+1||leased)return 5;
 cv_blit(0,0);before=sync_count;cv_begin(8,8,0);if(sync_count!=before+1||leased)return 6;
 cv_scan=1;cv_rect(0,0,8,8,0x1234);cv_blit(0,0);before=sync_count;fail_ready=1;
 cv_blit(0,0);if(cv_dma_pending!=2||sync_count!=before||!leased||sum()!=expected)return 12; /* scan failure cannot launch a blit */
 fail_ready=0;cv_begin(8,8,0);cv_scan=0;
 cv_rect(0,0,8,8,0x1234);cv_blit(0,0);expected=sum();fail_ready=1;
 {extern uint32_t fm1_rust_jam_stat(uint32_t);uint32_t calls=fm1_rust_jam_stat(80),frames=sync_count;
  cv_begin(8,8,0xffff);if(cv_dma_pending!=2||!leased||sum()!=expected)return 8;
  cv_pset(0,0,0xffff);cv_rect(0,0,8,8,0);cv_line(0,0,7,7,0xffff);px_blend(0,0,0xffff,8);cv_text(0,0,&FONT_XS,"NO",0xffff);cv_blit(0,0);cv_blit_from(0,0,1);
  if(fm1_rust_jam_stat(80)!=calls||sync_count!=frames||!leased||sum()!=expected)return 9;
  fail_ready=0;cv_pset(0,0,0xffff);if(fm1_rust_jam_stat(80)!=calls||!leased)return 10; /* fresh begin required */
  cv_begin(4,4,0xf800);if(cv_dma_pending||leased||cv_w!=4||cv_h!=4||cv_px[0]!=0xf8)return 11;
 }
 {extern uint32_t fm1_rust_jam_stat(uint32_t);if(fm1_rust_jam_stat(83))return 7;}
 puts("DMA confirmed completion, failed-ready quarantine, no typed borrow/reuse and fresh-begin recovery passed");return 0;
}
