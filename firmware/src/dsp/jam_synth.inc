/* SPDX-License-Identifier: GPL-3.0-only */
/* Synth Jam held-key ownership. Audio-core only; owner occupies existing slot
 * padding, so five manual keys and the protected bass need no extra RAM. */
static unsigned jp_held_count(unsigned pitch)
{
#if EN_RUST_FM6
    return fm1_rust_fm6_pool((fm1_fm6_slot_t *)fmx,6,pitch);
#else
    unsigned k,n=0;for(k=0;k<FM_NV;k++)n+=fmx[k].owner && fmx[k].s.note==pitch;return n;
#endif
}
static void jp_held_release(unsigned owner)
{
    unsigned result=0;
#if EN_RUST_FM6
    result=fm1_rust_fm6_pool((fm1_fm6_slot_t *)fmx,5,owner);
#else
    unsigned k;for(k=0;k<FM_NV;k++)if(fmx[k].owner==owner && owner) {
        result=0x100u|fmx[k].s.note;fmx[k].owner=0;fmx[k].gate=0;fmx[k].kill_left=0x7fffffff;break;
    }
#endif
    if(result && !jp_held_count(result&127u))midi(0x80,result&127u,0);
}
static void jp_held_flush(void)
{
    unsigned k;for(k=0;k<FM_NV;k++)if(fmx[k].owner)jp_held_release(fmx[k].owner);
}
static void jp_keys_stop(void)
{
    unsigned k;
    for(k=0;k<EN_NPAD;k++) {
        if(pad[k].midi_note)midi(0x80,pad[k].midi_note,0);
        pad[k].midi_note=0;
        if(pad[k].pnote)midi(0x84,pad[k].pnote,0);
        pad[k].pnote=0;
    }
    jp_pad_gate=0;jp_swell_phase=1.0f;
    for(k=0;k<FM_NV;k++)if(!fmx[k].part){fmx[k].gate=0;fmx[k].fade=0.9999f;}
}
static void jp_backing_stop(void)
{
    unsigned k;
    if(bass.midi_note)midi(0x82,bass.midi_note,0);
    bass.midi_note=0;
    for(k=0;k<FM_NV;k++)if(fmx[k].part){fmx[k].gate=0;fmx[k].fade=0.9999f;}
}
static void jp_held_key(unsigned owner,int raw_note,int velocity)
{
    uint32_t allocation;unsigned slot,previous,pitch;
    if(!owner && !velocity){jp_held_flush();return;}
    if(!((owner>=1 && owner<=27) || (owner>=128 && owner<=255)))return;
    jp_held_release(owner);
    if(!velocity || !jp_synth_on())return;
    pitch=(unsigned)clamp(raw_note+(owner<128?12*jp_octave(jp_data,0):0),0,127);
    allocation=fm_jam_note(0,(int)pitch,(float)velocity/127.0f,0x7fffffff);
    slot=allocation&255u;if(slot>=FM_NV)return;
    previous=allocation>>24;
    if((allocation>>16&255u) && !jp_held_count(previous))midi(0x80,previous,0);
    /* Multiple input owners of one pitch share its outgoing MIDI lifetime. */
    if(!jp_held_count(pitch))midi(0x90,pitch,(unsigned)velocity);
#if EN_RUST_FM6
    (void)fm1_rust_fm6_pool((fm1_fm6_slot_t *)fmx,4,slot|(owner<<8));
#else
    fmx[slot].owner=(uint8_t)owner;
#endif
}
