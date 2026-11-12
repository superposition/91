/* SPDX-License-Identifier: GPL-3.0-only */
/* Reuse the existing worker harness with foreground DJ voices active. */
#include "amb.h"
static void dj_worker_init(void){en_init();en_fxs(EN_FXS_SM_MIX,100);en_fxs(EN_FXS_SM_LEN,2);en_fxs(EN_FXS_SM_LOOP,1);en_note(EN_V_SAMPLE,53,1);en_note(EN_V_SAMPLE,57,1);en_note(EN_V_SAMPLE,70,1);en_note(EN_V_SAMPLE,74,1);en_note(EN_V_SAMPLE,79,1);}
#define en_init dj_worker_init
#define main dj_worker_main
#include "../host/core2_test.c"
#undef main
int main(int argc,char **argv)
{
    int result=dj_worker_main(argc,argv);
#if EN_RUST_WORKER
    extern uint32_t fm1_rust_jam_stat(uint32_t);
    /* Both ports must execute in the same actual audio run. A matching C
     * worker fallback is insufficient evidence for this combined boundary. */
    if(!result && (!fm1_rust_jam_stat(56) || !fm1_rust_jam_stat(57)
        || !fm1_rust_jam_stat(59) || fm1_rust_jam_stat(62)
        || fm1_rust_jam_stat(63) || !fm1_rust_jam_stat(72)
        || !fm1_rust_jam_stat(73))) {
        fprintf(stderr,"DJ/Rust-worker execution witness failed\n");
        return 1;
    }
#endif
    return result;
}
