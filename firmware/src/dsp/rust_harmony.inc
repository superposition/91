/* SPDX-License-Identifier: GPL-3.0-only */
/* Engine/render/drain numeric capture/publication only. Callback dispatch occurs after
 * every Rust borrow has ended; then the next phase recaptures state and RNG. */
#include "rust_harmony_state.inc"
static uint32_t harmony_transition(uint32_t kind,uint32_t a,uint32_t b,uint32_t d)
{
    fm1_harm_state_t s;fm1_harm_config_t c;uint32_t action;
    harmony_read(&s,&c);action=fm1_rust_harm_step(&s,&c,&rng,kind,a,b,d);harmony_write(&s);
    return action;
}
static int harmony_bars(void)
{
    return fm1_rust_harm_meter_bars(chord_genre,drift_unit,par[P_DRIFT],pg_on,pg_bars[pg_i]);
}
static void harmony_pad_retarget(void)
{
    fm1_harm_state_t s;fm1_harm_config_t c;fm1_harm_targets_t targets;float from[4];unsigned i;
    for(i=0;i<4;i++)from[i]=pad[i].target;
    harmony_read(&s,&c);
    if(!fm1_rust_harm_targets(&s,&c,from,&targets))return;
    harmony_write(&s);
    for(i=0;i<4;i++){en_chord[i]=(uint8_t)targets.display[i];pad[i].target=targets.target[i];}
    en_chord_root=(uint8_t)chord_pcs[0];en_chord_n=(uint8_t)chord_nt;
}
static int harmony_preview(int degree,int kind,int *pcs)
{
    fm1_harm_state_t s;fm1_harm_config_t c;harmony_read(&s,&c);
    return fm1_rust_harm_preview(&s,&c,degree,kind,pcs);
}
static void harmony_capture(uint8_t *bytes)
{
    fm1_harm_state_t s;fm1_harm_config_t c;harmony_read(&s,&c);
    (void)fm1_rust_harm_capture(&s,&c,bytes);
}
