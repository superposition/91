/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef FM1_RUST_APP_H
#define FM1_RUST_APP_H
#include <stdint.h>
typedef struct {uint32_t width,height;int32_t origin,top,bottom;uint32_t scan;} fm1_canvas_state;
typedef struct {uint32_t height,pad,first,last;const uint8_t *advance,*width;const uint16_t *offset;const uint8_t *data;uint32_t length;} fm1_canvas_font;
/* Synchronous, no retained pointer. DMA must be complete/confirmed stopped before a mutable
 * pixel borrow; only the active initialized prefix is used after begin. */
int32_t fm1_rust_canvas(uint32_t,uint16_t*,uint32_t,fm1_canvas_state*,const int32_t*,const fm1_canvas_font*,const uint8_t*,uint32_t);
int32_t fm1_rust_text_width(const fm1_canvas_font*,const uint8_t*,uint32_t);
void fm1_rust_palette(uint32_t,uint16_t*);
#endif
