/* SPDX-License-Identifier: GPL-3.0-only */
/* Vendor-object ABI evidence; compare this section with the generated-C
 * natural32 UI reference prefix. No code executes on the instrument. */
#include <stddef.h>
#include "../../firmware/src/dsp/rust_ui_state.h"
_Static_assert(sizeof(fm1_ui_state)==1320,"PI32 UI carrier size");
_Static_assert(__alignof__(fm1_ui_state)==4,"PI32 UI carrier alignment");
#define UI_TARGET_FIELD(name) offsetof(fm1_ui_state,name),sizeof(((fm1_ui_state*)0)->name),
const uint32_t fm1_ui_target_layout[FM1_UI_LAYOUT_WORDS]
    __attribute__((used,section(".fm1_ui_layout")))={
    sizeof(fm1_ui_state),__alignof__(fm1_ui_state),FM1_UI_FIELDS(UI_TARGET_FIELD)
};
#undef UI_TARGET_FIELD
