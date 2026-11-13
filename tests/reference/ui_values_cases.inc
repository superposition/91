/* SPDX-License-Identifier: GPL-3.0-only */
/* Every plain parameter, pseudo-control and all 35x4 FX controls go through
 * their real C/Rust consumer. Raw packed metadata is restored before audio. */
static void values_case(unsigned n,uint32_t*random)
{
    unsigned i,slot=n&15;uint32_t r=random_word(random);
    const int code=n%356<176?(int)(n%356):200+(int)(n%356-176);
    int lo=12345,hi=-12345;uint8_t bass=en_bs[slot],mod=en_mod[n%EN_NLANE][slot];
    held_keys&=~(1u<<11); /* frozen simultaneous tap division-by-zero is separate */
    ui.lane=(uint8_t)(n%6);ui.mix_trk=(uint8_t)n;ui.slsl=(uint8_t)(n>>1);
    ui.pgsl=(uint8_t)(n>>2);ui.bstp=ui.pstp=ui.stp=ui.astp=(uint8_t)n;
    ui.mood=(uint8_t)n;ui.gr_all=(uint8_t)(n%101);
    glob.sl[ui.slsl%SL_N]=(uint8_t)(n%191);glob.sl_on=(uint8_t)(n&1);
    /* Public foreground metadata, no retained private voice/sample pointers. */
    en_bs[slot]=(uint8_t)n;en_mod[ui.lane][slot]=(uint8_t)(n>>2);
    for(i=0;i<3;i++)word((uint32_t)knob_get(K_BSTY+(int)i));
    word((uint32_t)knob_get(K_STR));word((uint32_t)knob_get(K_STP));word((uint32_t)knob_get(K_STV));
    en_bs[slot]=bass;en_mod[ui.lane][slot]=mod;
    ui.as_t=now_ms-(n%4==0?399u:n%4==1?400u:n%4==2?401u:0u);
    ui.as_gate=(uint16_t)r;ui.as_acc=(uint16_t)(r>>8);ui.as_slide=(uint16_t)(r>>16);
    for(i=0;i<16;i++){ui.as_off[i]=(uint8_t)((r+i)&15);ui.as_oct[i]=(uint8_t)((r>>i)&3);}
    word((uint32_t)knob_par(code));word((uint32_t)knob_get(code));
    knob_range(code,&lo,&hi);word((uint32_t)lo);word((uint32_t)hi);
    for(i=0;i<3;i++)word((uint32_t)knob_get(K_ASTY+(int)i));
    for(i=0;i<16;i++)word((uint32_t)as_kind((int)i));
    for(i=0;i<P_NPARAMS;i++){
        const en_param_t*pi=en_param_info((int)i);
        const int value=n%3==0?pi->lo:n%3==1?pi->hi:(pi->lo+pi->hi)/2;
        word((uint32_t)mood_val((int)i,value));
    }
    if(n%64==31)mood_apply();
}
