/* SPDX-License-Identifier: GPL-3.0-only */
/* Execute the independently extracted R01 helper; UBSan records its boundary. */
#include <stdint.h>
static struct {uint32_t tap_t[4];uint8_t tap_n;} ui;
static struct {int16_t par[6];} proj;
#define P_TEMPO 5
static uint32_t plat_ms(void){return 100;}
static void knob_set(int p,int v){proj.par[p]=(int16_t)v;}
static void itoa_u(uint32_t n,char*b){(void)n;*b=0;}
static void say(const char*a,const char*b){(void)a;(void)b;}
#include "ui_tap_frozen.inc"
int main(void){tap_tempo();tap_tempo();return 0;}
