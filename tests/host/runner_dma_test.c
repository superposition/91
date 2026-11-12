/* SPDX-License-Identifier: GPL-3.0-only */
/* Execute the actual game draw adapter while the LCD owns its old source. */
static int test_ready(void);
#define LCD_READY() test_ready()
#define RUNNER_DRAW_TEST 1
#define main unused_policy_main
#include "../reference/runner_policy_reference.c"
#undef main
static const uint16_t *held;
static uint32_t fail_ready,held_hash;
static uint32_t pixels_hash(void){uint32_t i,h=2166136261u;for(i=0;i<240u*120u;i++)h=(h^held[i])*16777619u;return h;}
static int test_ready(void){if(held&&pixels_hash()!=held_hash)__builtin_trap();if(fail_ready)return 0;held=0;return 1;}
int main(void){
 extern uint32_t fm1_rust_jam_stat(uint32_t);uint32_t calls,game_calls;
 now=1000;kr.on=1;kr.ships=3;runner_draw(0);cv_blit(0,0);
 held=cv_px;held_hash=pixels_hash();calls=fm1_rust_jam_stat(80);game_calls=fm1_rust_jam_stat(82);
 fail_ready=1;runner_draw(1);cv_blit(0,120);
 if(cv_dma_pending!=2||held!=cv_px||pixels_hash()!=held_hash)return 1;
 if(fm1_rust_jam_stat(80)!=calls||fm1_rust_jam_stat(82)!=game_calls)return 2;
 fail_ready=0;cv_pset(0,0,0xffff);if(held!=cv_px||pixels_hash()!=held_hash)return 3;
 runner_draw(1);if(held||cv_dma_pending||cv_w!=240||cv_h!=120||cv_oy!=-120)return 4;
 if(fm1_rust_jam_stat(83))return 5;
 puts("Game draw preserves retained DMA source until a fresh confirmed-ready frame");return 0;
}
