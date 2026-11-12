/* SPDX-License-Identifier: GPL-3.0-only */
/* Execute the original OTA primitives or their production Rust adapters. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define FELUCCA_ID "FM-1_7000302"
#ifndef DEVICE_OTA_SOURCE
#define DEVICE_OTA_SOURCE "../../firmware/src/ota.c"
#endif
#include DEVICE_OTA_SOURCE
static int ota_wire_send(const uint8_t*p,uint32_t n){(void)p;(void)n;return 0;}
static int ota_frame_get(const uint8_t**p,uint32_t*n){(void)p;(void)n;return 0;}
static void ota_frame_done(void){}
static uint32_t ota_now_ms(void){return 0;}
static void ota_idle(void){}
static int ota_erase(uint32_t p){(void)p;return -1;}
static int ota_prog(uint32_t p,const void*b,uint32_t n){(void)p;(void)b;(void)n;return -1;}
static int ota_fread(uint32_t p,void*b,uint32_t n){(void)p;(void)b;(void)n;return -1;}
static void ota_show(uint32_t p,int32_t c){(void)p;(void)c;}
static void ota_commit(const uint8_t*p){(void)p;}
static void word(uint32_t v){unsigned i;for(i=0;i<4;i++)putchar((int)(v>>(i*8)&255));}
int main(void)
{
    uint8_t input[641],encoded[736],decoded[643],cipher[641];
    uint32_t n,i,seed,packed,limit;
    for(n=0;n<=640;n++) {
        for(i=0;i<n;i++) input[i]=(uint8_t)(i*157u+n*31u+(i>>2));
        for(seed=0;seed<4;seed++) word(ota_crc16(input,n,seed*21845u));
        memcpy(cipher,input,n);ota_jl_enc(cipher,n);fwrite(cipher,1,n,stdout);
        packed=ota_pack7(input,n,encoded);word(packed);fwrite(encoded,1,packed,stdout);
        /* Prefix size, masked high bits and untouched output tails matter. */
        for(i=0;i<packed;i++)encoded[i]|=(uint8_t)((i&1u)*128u);
        for(limit=0;limit<5;limit++) {
            uint32_t cap=limit==4?642u:limit*37u;
            memset(decoded,0xa5,sizeof decoded);
            word(ota_unpack7(encoded,packed,decoded,cap));
            fwrite(decoded,1,sizeof decoded,stdout);
        }
    }
    return ferror(stdout)!=0;
}
