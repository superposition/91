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
/* Scoped immutable descriptor, not retained state. On PI32v2 this is92B of
 * caller stack: flags,21 raw roots and a no-reentry publication barrier. */
typedef struct {
    uint32_t flags;
    void *slots[21];
    void (*publish)(void);
} fm1_usb_transport_t;
_Static_assert(offsetof(fm1_usb_transport_t,slots)==sizeof(void *) &&
               offsetof(fm1_usb_transport_t,publish)==22u*sizeof(void *) &&
               sizeof(fm1_usb_transport_t)==23u*sizeof(void *),"transport context ABI");
int32_t fm1_rust_usb_sysex(const fm1_usb_transport_t *,const void *input,uint32_t count);
int32_t fm1_rust_usb_device_receive(const fm1_usb_transport_t *,const void *input,uint32_t count);
uint32_t fm1_rust_usb_descriptor_length(uint32_t flags,uint32_t pid,uint32_t request);
int32_t fm1_rust_usb_descriptor_copy(uint32_t flags,uint32_t pid,uint32_t request,uint32_t offset,void *output,uint32_t capacity);
uint32_t fm1_rust_usb_ep0_start(uint32_t length,uint32_t wanted);
uint32_t fm1_rust_usb_ep0_packet(uint32_t left,uint32_t zlp);
uint32_t fm1_rust_usb_ep0_request(uint32_t flags,uint32_t pid,uint32_t first,uint32_t second,uint32_t config);
enum {
    FM1_USB_STALL=0,FM1_USB_ACK=1,FM1_USB_ADDRESS=2,FM1_USB_DESCRIPTOR=3,
    FM1_USB_CONFIG=4,FM1_USB_GET_CONFIG=5,FM1_USB_STATUS=6,FM1_USB_STREAM=7,
    FM1_USB_GET_STREAM=8,FM1_USB_CLEAR_HALT=9,FM1_USB_SET_LINE=10,
    FM1_USB_GET_LINE=11,FM1_USB_DTR=12,FM1_USB_SET_RATE=13,FM1_USB_GET_RATE=14
};
#ifdef __cplusplus
}
#endif
