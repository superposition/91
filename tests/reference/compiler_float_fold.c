/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
extern uint32_t folded_bits(void);
extern uint32_t runtime_bits(float);
int main(void){uint32_t a=folded_bits(),b=runtime_bits(6.28318530717959f);printf("{\"folded\":\"%08x\",\"runtime\":\"%08x\",\"equal\":%s}\n",a,b,a==b?"true":"false");return a==b?0:1;}
