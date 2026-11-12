/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <map>
#include <string>
#include <cstring>
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
#include <assert.h>
#define EN_RUST_HAL 1
#include "fm1_lcd_hw.h"
static uint32_t operations[10],last_single;static bool complete;
uint32_t fm1_rust_hal_io(uint32_t operation,uint32_t a,uint32_t,uint32_t,uint32_t){
    assert(operation<10);operations[operation]++;if(operation==9){last_single=a;return complete?1u:0u;}return 1;
}
#include "../../firmware/src/lcd.c"
int main(void){
    uint8_t pixels[8]={1,2,3,4,5,6,7,8};seed=91;hash=2166136261u;
    for(uint32_t i=0;i<64;i++)lcd_small[i]=(uint8_t)(i*7);
    for(uint32_t i=0;i<240;i++)lcd_fillbuf[i]=(uint16_t)(i*29);
    lcd_data(pixels,8);assert(lcd_busy==1&&operations[8]==1);
    assert(!lcd_ready()&&lcd_busy==2&&last_single==0);
    uint8_t small[64];uint16_t fill[240];memcpy(small,lcd_small,sizeof small);memcpy(fill,lcd_fillbuf,sizeof fill);
    const uint32_t commands=operations[7],data=operations[8],initializations=operations[5];
    lcd_cmd(0x29);lcd_data(pixels,8);lcd_window(0,0,9,9);lcd_fill(0,0,20,20,0xffff);lcd_init();
    assert(lcd_busy==2&&last_single==1&&operations[7]==commands&&operations[8]==data&&operations[5]==initializations);
    assert(!memcmp(small,lcd_small,sizeof small)&&!memcmp(fill,lcd_fillbuf,sizeof fill));
    complete=true;assert(lcd_ready()&&lcd_busy==0&&last_single==1);
    lcd_data(pixels,8);assert(lcd_busy==1&&operations[8]==data+1);
    complete=true;assert(lcd_ready()&&lcd_busy==0&&last_single==0);
    const uint32_t ready_polls=operations[9];assert(lcd_ready()&&operations[9]==ready_polls);
    puts("display unknown completion retains buffers and refuses commands/init/data; only confirmed completion releases");return 0;
}
