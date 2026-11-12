/* SPDX-License-Identifier: GPL-3.0-only */
/* The existing full-instrument FX driver, with the real output registered as a
 * retained DMA span. Only the external bus/consumer is modeled, not the DSP. */
#include <stdint.h>
#include <assert.h>
#include <string.h>
#include "fm1_rust_hal.h"
static void hal_render(int32_t *,uint32_t,uint32_t);
static uint32_t half_words=1024,render_calls,stores,dma_control=0x8800,stop_fences;
static int32_t dma[4096];
#if EN_RUST_HAL
uint32_t fm1_hal_test_mmio(uint32_t kind,uint32_t address,uint32_t value){
    assert(address==0x12e00u);
    if(kind==16)return dma_control;
    assert(kind==144);dma_control=(value&~0x8000u)|(dma_control&0x8000u);return 0;
}
void fm1_hal_test_store_step(uint32_t index,int32_t value){assert(index<2u*half_words);assert(dma[index]==value);stores++;}
void fm1_hal_audio_stop_fence(void){stop_fences++;}
uint32_t fm1_hal_audio_stop_confirm(void){return !(dma_control&0x800u);}
#endif
#define TEST_RENDER hal_render
#define main existing_effects_main
#include "effects_reference.c"
#undef main
static void hal_render(int32_t *out,uint32_t frames,uint32_t gain){
    const uint32_t half=render_calls&1u;
    const uint32_t other=(half^1u)*half_words;
    int32_t previous[1024];memcpy(previous,dma+other,half_words*4u);
#if EN_RUST_HAL
    if(!render_calls)assert(fm1_rust_hal_audio_config(dma,half_words,1));
    dma_control=0x800u|((half^1u)<<15);
    const uint32_t generation=fm1_rust_hal_audio_begin(half);assert(generation);
#endif
    gap_on=(uint8_t)(render_calls%29u<8u);hush=(uint8_t)(render_calls%47u<17u);
    en_render(dma+half*half_words,frames,gain);
#if EN_RUST_HAL
    assert(fm1_rust_hal_audio_end(generation));assert(stop_fences==0);
#endif
    assert(!memcmp(previous,dma+other,half_words*4u));
    memcpy(out,dma+half*half_words,frames*8u);
    record(&gap_g,4);record(&hush_g,4);record((const void *)&en_hushed,1);
    render_calls++;
}
int main(int argc,char **argv){
    if(argc>3)half_words=(uint32_t)atoi(argv[3]);assert(half_words==512||half_words==1024);
    const int result=existing_effects_main(argc,argv);
#if EN_RUST_HAL
    assert(render_calls>0&&stores>0);fprintf(stderr,"HAL retained halves:%u calls:%u stores:%u stop_fences:%u\n",half_words,render_calls,stores,stop_fences);
#endif
    return result;
}
