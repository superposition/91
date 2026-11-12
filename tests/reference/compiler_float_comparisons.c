/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
extern uint32_t expression_cmp32(void),constant_cmp32(void),runtime_cmp32(float,float,float);
extern uint32_t expression_cmp64(void),constant_cmp64(void),runtime_cmp64(double,double,double);
int main(void){uint32_t f=0x7fc00000u;uint64_t d=0x7ff8000000000000ull;float qf;double qd;memcpy(&qf,&f,4);memcpy(&qd,&d,8);
    uint32_t a=expression_cmp32(),b=constant_cmp32(),c=runtime_cmp32(0.0f,-0.0f,qf),x=expression_cmp64(),y=constant_cmp64(),z=runtime_cmp64(0.0,-0.0,qd);
    printf("{\"type\":\"f32\",\"expression\":\"%08x\",\"const_function\":\"%08x\",\"runtime\":\"%08x\"}\n",a,b,c);
    printf("{\"type\":\"f64\",\"expression\":\"%08x\",\"const_function\":\"%08x\",\"runtime\":\"%08x\"}\n",x,y,z);
    return a!=0x20b1||b!=0x20b1||c!=0x20b1||x!=0x20b1||y!=0x20b1||z!=0x20b1;}
