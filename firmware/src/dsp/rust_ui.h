/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_UI_H
#define FM1_RUST_UI_H
#include "rust_ui_state.h"
/* Initialized/aligned foreground UI carrier, disjoint immutable96 defaults.
 * No callback or retained pointer; dispatch engine/platform effects afterward. */
int32_t fm1_rust_ui_initialize(fm1_ui_state*,const int16_t*,uint16_t);
uint32_t fm1_rust_ui_layout(const fm1_ui_state*,uint32_t*,uint32_t);
#endif
