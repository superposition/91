/* SPDX-License-Identifier: GPL-3.0-only */
/* Emit exact initialized data from the selected C0 source, including binary32
 * bits. Compile with the immutable R01 include path for the independent oracle. */
#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>
#include <string.h>
#include "amb.c"
void en_midi_out(uint32_t a,uint32_t b,uint32_t c){(void)a;(void)b;(void)c;}
static void u8s(const uint8_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",p[i]);putchar(']');}
static void s8s(const int8_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%d",i?",":"",p[i]);putchar(']');}
static void u16s(const uint16_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",p[i]);putchar(']');}
static void s32s(const int32_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%d",i?",":"",p[i]);putchar(']');}
static void u32s(const uint32_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",p[i]);putchar(']');}
static void s64s(const int64_t *p,size_t n){size_t i;putchar('[');for(i=0;i<n;i++)printf("%s%" PRId64,i?",":"",p[i]);putchar(']');}
static void floats(const float *p,size_t n){size_t i;uint32_t v;putchar('[');for(i=0;i<n;i++){memcpy(&v,p+i,4);printf("%s%u",i?",":"",v);}putchar(']');}
static void names(const char *const *p,size_t n){size_t i,j;uint8_t name[16];putchar('[');for(i=0;i<n;i++){memset(name,0,16);for(j=0;j<15&&p[i][j];j++)name[j]=(uint8_t)p[i][j];if(i)putchar(',');u8s(name,16);}putchar(']');}
#define EMIT(name,fn) do{printf("\"" #name "\":");fn((const void *)(name),sizeof(name)/sizeof((name)[0]));printf(",\n");}while(0)
int main(void){
    uint32_t i;uint8_t voice[156];fm_init();puts("{");
    EMIT(FM6_LEVELLUT,u8s);EMIT(FM6_STATICS,s32s);EMIT(FM6_VELOCITY,u8s);EMIT(FM6_EXPSCALE,u8s);EMIT(FM6_PMS,u8s);EMIT(FM6_AMS,u32s);EMIT(FM6_PEG_RATE,u8s);EMIT(FM6_PEG_STEP,s8s);
    printf("\"FM6_ALG\":");u8s(&FM6_ALG[0][0],sizeof FM6_ALG);puts(",");
    EMIT(FM6_FREQ,s32s);EMIT(FM6_COARSE,s32s);EMIT(FM6_FINE,s32s);EMIT(FM6_DETUNE,s64s);EMIT(FM6_LFO_INC,u32s);EMIT(FM6_PORTA,s32s);EMIT(FM6_GLISS,s32s);
    EMIT(FM6_MKI_LOG_H,u16s);EMIT(FM6_MKI_EXP_D,u8s);EMIT(FM6_OPL_LOG_H,u16s);EMIT(FM6_OPL_EXP_D,u8s);
    EMIT(FM6_INIT,u8s);printf("\"FM6_FACTORY\":");u8s(&FM6_FACTORY[0][0],sizeof FM6_FACTORY);puts(",");printf("\"FM6_DY\":");u8s(&FM6_DY[0][0],sizeof FM6_DY);puts(",");
    printf("\"FM6_ROM\":[");for(i=0;i<FM6_NROM;i++){if(i)putchar(',');fm6_rom_patch(&FM6_ROM[i],voice);u8s(voice,155);}puts("],");
    EMIT(FM_MAP,u8s);EMIT(FM_CAP,u8s);EMIT(FM_GAIN,floats);EMIT(FM_GAIN_DY,floats);printf("\"FM_DX_NAME\":");names(FM_DX_NAME,EN_NFMFACT);puts(",");printf("\"FM6_DY_NAME\":");names(FM6_DY_NAME,FM6_NDY);puts(",");
    EMIT(FM6_SIN,floats);EMIT(FM6_MKI_LOG,u16s);EMIT(FM6_MKI_EXP,u16s);EMIT(FM6_OPL_LOG,u16s);EMIT(FM6_OPL_EXP,u16s);
    printf("\"layout\":{\"eg\":%zu,\"peg\":%zu,\"voice\":%zu,\"slot\":%zu,\"lfo\":%zu,\"slots\":%zu,\"ram_luts\":%zu}\n}",sizeof(fm6_eg_t),sizeof(fm6_peg_t),sizeof(fm6_voice_t),sizeof(fm_slot_t),sizeof(fm6_lfo_t),sizeof fmx,sizeof FM6_SIN+sizeof FM6_MKI_LOG+sizeof FM6_MKI_EXP+sizeof FM6_OPL_LOG+sizeof FM6_OPL_EXP);
    return 0;
}
