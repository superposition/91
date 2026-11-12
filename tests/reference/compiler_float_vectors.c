/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
extern uint32_t folded32(uint32_t),runtime32(uint32_t,float,float,float);
extern uint64_t folded64(uint32_t),runtime64(uint32_t,double,double,double);
static const struct{const char *name;float a,b,c;} f32[]={
    {"noise-factor",6.28318530717959f,300,0},{"literal-operands",16777217.0f,16777216.0f,0},
    {"add-chain",1,1e-8f,1},{"multiply-add",.7f,.7f,.1f},{"division-chain",.004f,44100,1},
    {"subtract-literal",1,.9993f,0},{"noncontracted",1.00000011920928955078125f,1.00000011920928955078125f,1.0000002384185791015625f},
    {"negative-zero",-0.0f,3,0},{"positive-zero",0.0f,3,0},{"underflow-chain",1e-30f,1e-20f,1e20f},
    {"overflow-chain",1e30f,1e20f,1e20f},{"signed-cancellation",1,1,-1},{"literal-to-integer",16777217.0f,0,0},
    {"exact-control",1.25f,2,.5f},{"const-item-noise",6.28318530717959f,300,0},{"const-item-chain",1,1e-8f,1},{"const-function",.7f,.7f,.1f},
    {"remainder-positive",5.5f,2,0},{"remainder-negative-a",-5.5f,2,0},{"remainder-negative-b",5.5f,-2,0},{"remainder-negative-both",-5.5f,-2,0},{"remainder-positive-zero",0.0f,2,0},{"remainder-negative-zero",-0.0f,2,0}};
static const struct{const char *name;double a,b,c;} f64[]={
    {"noise-control",6.28318530717959,300,0},{"literal-control",1.0000000000000002,1,0},
    {"add-chain-control",1,1e-17,1},{"noncontracted-control",1.0000000000000002,1.0000000000000002,1.0000000000000004},
    {"negative-zero-control",-0.0,3,0},{"exact-control",1.25,2,.5},{"f32-widen-control",(double)16777217.0f,0,0},{"const-item-control",1,1e-17,1},
    {"remainder-positive",5.5,2,0},{"remainder-negative-a",-5.5,2,0},{"remainder-negative-b",5.5,-2,0},{"remainder-negative-both",-5.5,-2,0},{"remainder-positive-zero",0.0,2,0},{"remainder-negative-zero",-0.0,2,0}};
int main(void){unsigned i,bad=0;for(i=0;i<sizeof f32/sizeof*f32;i++){uint32_t a=folded32(i),b=runtime32(i,f32[i].a,f32[i].b,f32[i].c);bad+=a!=b;printf("{\"type\":\"f32\",\"case\":%u,\"name\":\"%s\",\"folded\":\"%08x\",\"runtime\":\"%08x\",\"equal\":%s}\n",i,f32[i].name,a,b,a==b?"true":"false");}
    for(i=0;i<sizeof f64/sizeof*f64;i++){uint64_t a=folded64(i),b=runtime64(i,f64[i].a,f64[i].b,f64[i].c);bad+=a!=b;printf("{\"type\":\"f64\",\"case\":%u,\"name\":\"%s\",\"folded\":\"%016llx\",\"runtime\":\"%016llx\",\"equal\":%s}\n",i,f64[i].name,(unsigned long long)a,(unsigned long long)b,a==b?"true":"false");}
    return bad?1:0;}
