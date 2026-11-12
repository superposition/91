/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
/* Fixed-copy codec ABI; caller storage stays in C. No pointer is retained. */
uint32_t fm1_rust_formats_crc32(const uint8_t*,uint32_t);
int32_t fm1_rust_formats_object_header(uint32_t,uint32_t,uint32_t,uint32_t,uint8_t*,uint32_t);
int32_t fm1_rust_formats_global(const int16_t*,const uint8_t*,uint32_t,const uint8_t*,uint8_t*,uint32_t);
int32_t fm1_rust_formats_settings(const uint8_t*,uint32_t,const uint8_t*,const uint8_t*,uint32_t,uint32_t,uint8_t*,uint32_t);
int32_t fm1_rust_formats_voices(const uint8_t*,uint32_t,uint8_t*,uint32_t);
int32_t fm1_rust_formats_voice(uint32_t,const uint8_t*,uint32_t,uint8_t*,uint32_t);
int32_t fm1_rust_formats_record_pack(const int16_t*,uint16_t,const uint16_t*,const uint8_t*,const uint8_t*,const uint8_t*,uint8_t*,uint32_t);
int32_t fm1_rust_formats_record_unpack(const uint8_t*,uint32_t,uint32_t,int16_t*,uint16_t*,uint16_t*,uint8_t*,uint8_t*,uint8_t*);
int32_t fm1_rust_formats_legacy(const uint8_t*,uint32_t,uint32_t,uint32_t,int16_t*,uint16_t*,uint16_t*);
uint32_t fm1_rust_formats_classify(const uint8_t*,uint32_t);
uint32_t fm1_rust_formats_sum7(const uint8_t*,uint32_t);
uint32_t fm1_rust_formats_control(uint8_t*,uint32_t,uint32_t,uint32_t,uint32_t,uint32_t);
int32_t fm1_rust_formats_restore(uint32_t,const uint8_t*,uint32_t,uint32_t,uint32_t);
int32_t fm1_rust_formats_chunk(const uint8_t*,uint32_t,uint8_t*,uint16_t*);
uint32_t fm1_rust_formats_backup_frame(uint8_t*,uint32_t,uint32_t,uint32_t,uint32_t,const uint8_t*,uint32_t);
uint32_t fm1_rust_formats_dump(uint8_t*,uint32_t,uint32_t,const int32_t*,uint32_t);
uint32_t fm1_rust_formats_editor_voice(uint8_t*,uint32_t,uint32_t,uint32_t,const uint8_t*);
