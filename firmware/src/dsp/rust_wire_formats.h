/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
/* All ranges are caller-owned; input/output must be disjoint. The dispatcher
 * initializes the whole output capacity. Only its returned prefix is a frame. */
int32_t fm1_rust_wire(uint32_t,const uint32_t*,uint32_t,const uint8_t*,uint32_t,uint8_t*,uint32_t);
int32_t fm1_rust_wire_enc(uint8_t*,uint32_t,uint32_t);
uint32_t fm1_rust_wire_unpack7(const uint8_t*,uint32_t,uint8_t*,uint32_t);
uint32_t fm1_rust_wire_ota_valid(const uint8_t*,uint32_t);
static inline uint32_t rust_wire_rd32(const uint8_t *p)
{ return (uint32_t)p[0]|(uint32_t)p[1]<<8|(uint32_t)p[2]<<16|(uint32_t)p[3]<<24; }
static inline uint32_t rust_wire_crc16(const uint8_t *p,uint32_t n,uint32_t seed)
{
    uint8_t out[4];uint32_t args[1]={seed};
    return fm1_rust_wire(70,args,1,p,n,out,4)<0?0:rust_wire_rd32(out);
}
static inline uint32_t rust_wire_words(uint8_t *out,uint32_t capacity,uint32_t kind,const uint32_t *words,uint32_t count)
{
    uint32_t args[1]={kind};
    int32_t n=fm1_rust_wire(76,args,1,(const uint8_t*)words,count*4,out,capacity);
    return n<0?0:(uint32_t)n;
}
