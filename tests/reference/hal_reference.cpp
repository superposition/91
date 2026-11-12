/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <map>
#include <string>
static std::map<uint32_t,uint32_t> registers;
static uint32_t seed,timer,adc_reads,ready_after,hash,events;
static FILE *trace;
static void word(uint32_t value){for(int i=0;i<4;i++){uint8_t b=(uint8_t)(value>>(8*i));hash=(hash^b)*16777619u;if(trace)fputc(b,trace);}}
static void event(uint32_t kind,uint32_t addr,uint32_t value){word(kind);word(addr);word(value);events++;}
static uint32_t read_reg(uint32_t address,uint32_t width){
    auto p=registers.find(address);uint32_t value=p==registers.end()?(address*1664525u)^seed:p->second;
    if(address==0x10804u){timer+=24000u;value=timer;}
    if(address==0x13100u){value&=~0x80u;if(++adc_reads>=ready_after)value|=0x80u;}
    if(width!=32)value&=(1u<<width)-1;
    event(width,address,value);return value;
}
static void write_reg(uint32_t address,uint32_t width,uint32_t value){if(width!=32)value&=(1u<<width)-1;registers[address]=value;event(128u|width,address,value);}
template<class T> struct Cell {
    uint32_t address;
    operator T() const {return (T)read_reg(address,8u*sizeof(T));}
    Cell &operator=(T value){write_reg(address,8u*sizeof(T),value);return *this;}
    Cell &operator|=(uint32_t value){return *this=(T)((T)*this|value);}
    Cell &operator&=(uint32_t value){return *this=(T)((T)*this&value);}
};
template<class T> struct Pointer {uint32_t address;Cell<T> operator*()const{return {address};}Cell<T> operator[](uint32_t n)const{return {address+n*(uint32_t)sizeof(T)};}};
template<class T> Pointer<T> proxy(uintptr_t address){return {(uint32_t)address};}
#if EN_RUST_HAL
extern "C" uint32_t fm1_hal_test_mmio(uint32_t kind,uint32_t address,uint32_t value){
    if(kind&128u){write_reg(address,kind&127u,value);return 0;}
    return read_reg(address,kind);
}
extern "C" void fm1_hal_test_store_step(uint32_t,int32_t) {}
#endif
#include "fm1_time.h"
#include "fm1_gpio.h"
#include "fm1_irq.h"
#include "fm1_timer.h"
#include "fm1_adc.h"
#include "fm1_audio.h"
#include "fm1_lcd_hw.h"
int main(int argc,char **argv){
    trace=argc>1?fopen(argv[1],"wb"):0;if(argc>1&&!trace)return 2;
    for(uint32_t op=0;op<10;op++)for(uint32_t variant=0;variant<32;variant++){
        registers.clear();seed=variant*0x9e3779b9u;timer=0xffffffffu-variant*997u;adc_reads=0;ready_after=variant&1?3u:0xffffffffu;
        registers[0x10800]=(variant&1)?0x4009u:0u;hash=2166136261u;events=0;uint32_t result=0;
        void (*handler)(void)=(void(*)(void))(uintptr_t)(0x02001200u+4u*variant);
        switch(op){
        case 0:fm1_time_init();break;
        case 1:fm1_timer5_start(handler,variant);break;
        case 2:fm1_adc_init();break;
        case 3:result=(uint32_t)fm1_adc_read(variant);break;
        case 4:fm1_audio_init((int32_t *)0x01c08000u,variant&1?512u:1024u,handler,variant);break;
        case 5:fm1_lcd_hw_init();break;
        case 6:fm1_lcd_baud(variant);break;
        case 7:fm1_lcd_send_cmd((uint8_t)(variant*17u));break;
        case 8:fm1_lcd_send_data((void *)(uintptr_t)(0x01c20000u+variant*32u),variant*240u);break;
        case 9:fm1_irq_attach(variant*4u,handler,variant);result=1;break;
        }
        word(result);printf("{\"operation\":%u,\"variant\":%u,\"result\":%u,\"events\":%u,\"hash\":\"%08x\"}\n",op,variant,result,events,hash);
    }
    if(trace)fclose(trace);return 0;
}
