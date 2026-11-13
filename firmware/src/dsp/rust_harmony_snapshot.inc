/* SPDX-License-Identifier: GPL-3.0-only */
/* Read-only foreground observations: CPU0 owns every source field. The copy
 * contains no worker-private sample/voice spans. Never run Rust math under a
 * mask introduced here, and never change the caller's incoming IRQ state. */
#if defined(FM1_HARM_SNAPSHOT_TEST) || (!defined(EN_HOST) && !defined(EN_WEB))
#include "../../hal/fm1_irq_state.h"
#endif
static void harmony_readonly(fm1_harm_state_t *state,fm1_harm_config_t *config)
{
#if defined(FM1_HARM_SNAPSHOT_TEST) || (!defined(EN_HOST) && !defined(EN_WEB))
    const uint32_t flags=fm1_irq_state_save();
    harmony_read(state,config);
    fm1_irq_state_restore(flags);
#else
    /* Native/WASM foreground and render are already serialized. */
    harmony_read(state,config);
#endif
}
