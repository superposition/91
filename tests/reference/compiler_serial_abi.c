/* SPDX-License-Identifier: GPL-3.0-only */
/* Single-threaded diagnostic implementation of the opaque MCU ABI.
 * No hardware or producer/consumer synchronization qualification is implied. */
#include <stdint.h>
#include <stddef.h>
uint32_t fm1_mcu_atomic(uint32_t operation,uint32_t *word,uint32_t expected,uint32_t value)
{
    uint32_t old=*word;
    switch(operation){
    case 0:break;case 1:*word=value;break;case 2:*word=old+value;break;
    case 3:*word=old-value;break;case 4:if(value>old)*word=value;break;
    case 5:if(old==expected)*word=value;break;case 6:*word=value;break;
    case 7:*word=old|value;break;case 8:*word=old&value;break;case 9:*word=old^value;break;
    default:__builtin_trap();
    }
    return old;
}
void fm1_mcu_fence(void){__asm__ volatile("" ::: "memory");}
/* mrustc's divergent foreign return uses its char-sized tBANG representation. */
__attribute__((noreturn)) char fm1_mcu_panic(void){__builtin_trap();}
int memcmp(const void *a,const void *b,size_t n)
{
    const unsigned char *p=a,*q=b;
    for(;n;n--,p++,q++)if(*p!=*q)return *p-*q;
    return 0;
}
