/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual Rust C ABI with externally advancing DMA observations. DSP words are
 * separately qualified against frozen C by hal_audio_reference.c. */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "fm1_rust_hal.h"
extern void fm1_rust_hal_test_reset(void);
extern void fm1_rust_fx_output(const float *,const float *,uint32_t,int32_t *,int16_t *,float,float,uint32_t);
extern uint32_t fm1_rust_fx_fades(float *,uint32_t,uint32_t,int32_t *,int16_t *,uint32_t);
static int32_t dma[2048];static int16_t usb[32];static float left[16],right[16];
static uint32_t control_word,steps,advance_at,stopping,stores,after_reclaim,stop_fences,confirmed,half_words,chosen_half;
static void step(void){if(++steps==advance_at)control_word^=0x8000u;}
uint32_t fm1_hal_test_mmio(uint32_t kind,uint32_t address,uint32_t value){
    assert(address==0x12e00u);uint32_t result=control_word;
    if(kind==144){control_word=(value&~0x8000u)|(control_word&0x8000u);if(!(value&0x800u))stopping=1;}
    else assert(kind==16);
    step();return result;
}
void fm1_hal_test_store_step(uint32_t index,int32_t value){
    assert(!stopping);assert(index>=chosen_half*half_words&&index<chosen_half*half_words+32);
    assert(dma[index]==value);stores++;
    if((((control_word>>15)&1u)^1u)!=chosen_half)after_reclaim++;
    step();
}
void fm1_hal_audio_stop_fence(void){stop_fences++;step();}
uint32_t fm1_hal_audio_stop_confirm(void){step();return confirmed&&!(control_word&0x800u);}
static void setup(uint32_t half,uint32_t words,uint32_t stop){
    fm1_rust_hal_test_reset();chosen_half=half;half_words=words;confirmed=stop;
    control_word=0x800u|((half^1u)<<15);steps=stopping=stores=after_reclaim=stop_fences=0;advance_at=UINT32_MAX;
    for(uint32_t i=0;i<2048;i++)dma[i]=0x13579;
    for(uint32_t i=0;i<16;i++){left[i]=(float)((int)i-7)*.11f;right[i]=-left[i]*.7f;}
}
static void execute(uint32_t stage,int32_t *out){
    float fade[2]={.75f,.4f};
    if(stage==0)fm1_rust_fx_output(left,right,16,out,usb,1.7f,.8f,0);
    else if(stage==1)fm1_rust_fx_fades(fade,1,0,out,usb,16);
    else fm1_rust_hal_audio_silence(out,16);
}
int main(void){
    uint32_t cases=0;
    for(uint32_t words=512;words<=1024;words*=2)for(uint32_t half=0;half<2;half++)for(uint32_t stop=0;stop<2;stop++)for(uint32_t stage=0;stage<3;stage++){
        setup(half,words,stop);int32_t expected[32],initial[32];int16_t expected_usb[32],initial_usb[32];
        fm1_rust_fx_output(left,right,16,initial,usb,1.7f,.8f,0);memcpy(expected,initial,sizeof expected);memcpy(initial_usb,usb,sizeof usb);
        execute(stage,expected);memcpy(expected_usb,usb,sizeof usb);
        memcpy(dma+half*words,initial,sizeof initial);assert(fm1_rust_hal_audio_config(dma,words,1));
        uint32_t generation=fm1_rust_hal_audio_begin(half);assert(generation);steps=0;
        memcpy(usb,initial_usb,sizeof usb);execute(stage,dma+half*words);assert(fm1_rust_hal_audio_end(generation));
        const uint32_t boundaries=steps;
        assert(!memcmp(expected,dma+half*words,sizeof expected));assert(!memcmp(usb,expected_usb,sizeof usb));assert(after_reclaim==0&&stop_fences==0);
        for(uint32_t at=1;at<=boundaries;at++){
            setup(half,words,stop);memcpy(dma+half*words,initial,sizeof initial);assert(fm1_rust_hal_audio_config(dma,words,1));
            generation=fm1_rust_hal_audio_begin(half);assert(generation);steps=0;advance_at=at;
            memcpy(usb,initial_usb,sizeof usb);execute(stage,dma+half*words);fm1_rust_hal_audio_end(generation);
            assert(after_reclaim<=1);assert(stop_fences<=1);
            for(uint32_t i=0;i<2u*words;i++)if(i<half*words||i>=half*words+32)assert(dma[i]==0x13579);
            if(stopping){
                const uint32_t old=stores;execute(stage,dma+half*words);assert(stores==old);
                assert(!fm1_rust_hal_audio_begin(half));assert(!fm1_rust_hal_audio_pause());assert(!fm1_rust_hal_audio_restart());
                if(!stop){assert(!fm1_rust_hal_audio_reusable());assert(!fm1_rust_hal_audio_config(dma,words,0));confirmed=1;}
                assert(fm1_rust_hal_audio_reusable());assert(!fm1_rust_hal_audio_restart());
                assert(fm1_rust_hal_audio_config(dma,words,1));
            }
            cases++;
        }
    }
    setup(0,1024,1);assert(!fm1_rust_hal_audio_pause());assert(!fm1_rust_hal_audio_restart());
    assert(fm1_rust_hal_audio_config(dma,1024,1));assert(fm1_rust_hal_audio_pause());assert(stopping&&!(control_word&0x800u));
    memset(dma,0,sizeof dma);assert(fm1_rust_hal_audio_restart());stopping=0;control_word|=0x800u;
    uint32_t a=fm1_rust_hal_audio_begin(0);assert(a&&fm1_rust_hal_audio_end(a));uint32_t b=fm1_rust_hal_audio_begin(0);assert(b&&b!=a);
    assert(!fm1_rust_hal_audio_end(a)&&fm1_rust_hal_audio_rendering());assert(fm1_rust_hal_audio_end(b));
    a=fm1_rust_hal_audio_begin(0);assert(a);const uint32_t old=stores;execute(0,dma+1024);assert(stores==old&&stopping);
    setup(0,1024,0);assert(!fm1_rust_hal_audio_config(dma,1024,0));
    for(uint32_t stage=0;stage<3;stage++)execute(stage,dma);
    for(uint32_t i=0;i<2048;i++)assert(dma[i]==0x13579);
    assert(stores==0&&!fm1_rust_hal_audio_reusable());
    assert(!fm1_rust_hal_audio_begin(0)&&stopping);
    confirmed=1;assert(fm1_rust_hal_audio_reusable());assert(fm1_rust_hal_audio_config(dma,1024,1));
    printf("actual output/fade/silence ABI:%u advancement boundaries, confirmed/unknown stop, retained halves, late retirement and stale generations pass\n",cases);
    return 0;
}
