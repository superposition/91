/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_PERSISTENCE_H
#define FM1_RUST_PERSISTENCE_H
#include <stdint.h>
typedef struct {
    int (*read)(uint32_t, void *, uint32_t);
    int (*erase)(uint32_t);
    int (*program)(uint32_t, const void *, uint32_t);
} fm1_store_hooks;
/* All callbacks finish synchronously. No re-entry or retained pointer. The
 * caller's 3840-byte scratch and data are separate live RAM ranges. */
int fm1_rust_store(uint32_t save, uint32_t object, void *data, uint32_t length,
                   uint8_t *scratch, const fm1_store_hooks *hooks);
#endif
