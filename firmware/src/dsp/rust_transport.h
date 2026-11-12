/* SPDX-License-Identifier: GPL-3.0-only */
/* Transitional render-owned state. Zero initialize; mutate only through Rust.
 * All calls are serialized by the same render owner; no borrow survives a call. */
#pragma once
#include <stdint.h>
typedef struct {
    float pulse_left;
    uint32_t pulse, external_age, control_acc;
} fm1_transport_t;
_Static_assert(sizeof(fm1_transport_t) == 16, "Rust transport ABI size");
_Static_assert(_Alignof(fm1_transport_t) == 4, "Rust transport ABI alignment");
extern float fm1_rust_transport_step_length(uint32_t);
extern void fm1_rust_transport_start(fm1_transport_t *, uint32_t);
extern void fm1_rust_transport_pulse(fm1_transport_t *);
extern uint32_t fm1_rust_transport_tick(const fm1_transport_t *);
extern void fm1_rust_transport_receive(fm1_transport_t *);
extern uint32_t fm1_rust_transport_expired(fm1_transport_t *, uint32_t);
extern uint32_t fm1_rust_transport_due(const fm1_transport_t *);
extern void fm1_rust_transport_schedule(fm1_transport_t *, float);
extern uint32_t fm1_rust_transport_clock_limit(const fm1_transport_t *, uint32_t, float, uint32_t);
extern uint32_t fm1_rust_transport_wait_limit(uint32_t, int32_t, uint32_t);
extern uint32_t fm1_rust_transport_advance(fm1_transport_t *, uint32_t, uint32_t);
