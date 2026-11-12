/* SPDX-License-Identifier: GPL-3.0-only */
/* Immutable numeric voice recipes from the selected real engine. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t s,uint32_t a,uint32_t b){(void)s;(void)a;(void)b;}
static void words(const char *name,const void *p,size_t n){size_t i;const uint8_t *b=p;printf("\"%s\":[",name);for(i=0;i<n;i++){uint32_t v;memcpy(&v,b+4*i,4);printf("%s%u",i?",":"",v);}putchar(']');}
static void bytes(const char *name,const void *p,size_t n){size_t i;const uint8_t *b=p;printf("\"%s\":[",name);for(i=0;i<n;i++)printf("%s%u",i?",":"",(unsigned)b[i]);putchar(']');}
#define W(v) words(#v,v,sizeof(v)/4)
#define B(v) bytes(#v,v,sizeof(v))
int main(void){putchar('{');W(KITS);putchar(',');W(PAD_SOUND);putchar(',');W(DRONE_SOUND);putchar(',');W(DRONE_FR);putchar(',');W(ACID_SOUND);putchar(',');W(ACID_FM);putchar(',');W(CM_SOUND);putchar(',');W(PAD_PAN);putchar(',');W(BREATH);putchar(',');W(START_T);putchar(',');W(VOWEL);putchar(',');W(HAT_INC);putchar(',');W(LFO_RATE_BEATS);putchar(',');B(DRUM_GM);putchar(',');B(EN_TRIM);putchar(',');W(SINT);puts("}");return 0;}
