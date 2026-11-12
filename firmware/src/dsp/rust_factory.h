/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
/* Immutable metadata lives in Rust. Payload bridges copy into C-owned storage. */
const en_param_t *fm1_rust_factory_parameters(void);
const uint8_t *const *fm1_rust_factory_names(uint32_t category);
const uint8_t *fm1_rust_factory_name(uint32_t category,uint32_t index);
const uint16_t *fm1_rust_factory_modes(void);
int32_t fm1_rust_factory_donor(int32_t genre);
int32_t fm1_rust_factory_piece(int32_t genre,int32_t piece,int16_t *output,uint32_t capacity,uint16_t *pool);
int32_t fm1_rust_factory_bank(int32_t genre,int32_t piece,uint32_t kind,uint8_t *output,uint32_t capacity);
int32_t fm1_rust_factory_arc(int32_t piece,uint8_t *output,uint32_t capacity);
int32_t fm1_rust_factory_line(uint32_t kind,uint32_t decode,const uint8_t *input,uint32_t length,uint8_t *output,uint32_t capacity);
