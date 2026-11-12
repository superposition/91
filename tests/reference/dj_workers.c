/* SPDX-License-Identifier: GPL-3.0-only */
/* Reuse the existing worker harness with foreground DJ voices active. */
#include "amb.h"
static void dj_worker_init(void){en_init();en_fxs(EN_FXS_SM_MIX,100);en_fxs(EN_FXS_SM_LEN,2);en_fxs(EN_FXS_SM_LOOP,1);en_note(EN_V_SAMPLE,53,1);en_note(EN_V_SAMPLE,57,1);en_note(EN_V_SAMPLE,70,1);en_note(EN_V_SAMPLE,74,1);en_note(EN_V_SAMPLE,79,1);}
#define en_init dj_worker_init
#include "../host/core2_test.c"
