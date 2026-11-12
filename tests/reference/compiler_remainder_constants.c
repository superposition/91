/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
extern uint32_t const_rem32(uint32_t);extern uint64_t const_rem64(uint32_t);
int main(void){unsigned i;for(i=0;i<6;i++)printf("{\"case\":%u,\"f32\":\"%08x\",\"f64\":\"%016llx\"}\n",i,const_rem32(i),(unsigned long long)const_rem64(i));return 0;}
