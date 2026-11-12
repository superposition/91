/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
#include <stddef.h>
typedef struct {
    uint32_t rd,bytes,drops,msgs;
    uint8_t st,need,got,d0,sysex;
} fm1_trs_state_t;
_Static_assert(sizeof(fm1_trs_state_t)==24 && offsetof(fm1_trs_state_t,st)==16 && offsetof(fm1_trs_state_t,sysex)==20,"TRS parser ABI");
#ifdef __cplusplus
extern "C" {
#endif
void fm1_rust_usb_trs_clear(volatile uint8_t *ring);
void fm1_rust_usb_trs_byte(fm1_trs_state_t *,uint32_t,int (*send)(uint32_t));
uint32_t fm1_rust_usb_trs_take(fm1_trs_state_t *,volatile uint8_t *,uint32_t,int (*send)(uint32_t));
#ifdef __cplusplus
}
#endif
