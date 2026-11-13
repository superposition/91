/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/amb.c"
static unsigned checking,active[3][128],key_ons,key_offs,bass_ons,drum_ons;
void en_midi_out(uint32_t st,uint32_t note,uint32_t vel)
{
    unsigned ch=st&15u,kind=st&0xf0u;
    if(!checking || note>=128)return;
    if(kind==0x90 && vel && ch==9)drum_ons++;
    if(ch!=0 && ch!=2)return;
    if(kind==0xb0 && note==123){memset(active[ch],0,sizeof active[ch]);return;}
    if(kind==0x80 || (kind==0x90 && !vel)){
        if(ch==0){assert(active[ch][note]);key_offs++;}active[ch][note]=0;return;
    }
    if(kind!=0x90)return;
    if(ch==0){assert(!active[ch][note]);key_ons++;}else bass_ons++;
    active[ch][note]=1;
}
static int32_t audio[512];
static uint64_t nonzero,fullscale;
static void render(unsigned blocks)
{
    unsigned b,i;for(b=0;b<blocks;b++){
        en_render(audio,256,2048);
        for(i=0;i<512;i++){int64_t x=audio[i];if(x<0)x=-x;nonzero+=x!=0;fullscale+=x>=8388607;}
    }
}
static unsigned held_count(void)
{
    unsigned k,n=0;for(k=0;k<FM_NV;k++)n+=fmx[k].owner!=0;return n;
}
static fm_slot_t *owner(unsigned id)
{
    unsigned k;for(k=0;k<FM_NV;k++)if(fmx[k].owner==id)return &fmx[k];return 0;
}
static void command(int id,int note,int vel){en_jam_key(id,note,vel);render(1);}
int main(void)
{
    uint8_t before[JP_MAX];unsigned n,k,on;uint16_t position;uint32_t seed;uint8_t key,form,program;
    en_init();en_jam_set(JAM_ON,1);render(1);
    {const uint8_t *p=en_jam_data(&n);memcpy(before,p,n);}before[JP2_HEAD+25]=2;assert(!en_jam_valid(before,n));
    key=jp_now.key;form=jp_now.form;program=jp_data[JP2_HEAD+5];position=jp_pos;seed=jp_seed;
    en_jam_set(JAM_SYNTH,1);render(1);assert(jp_synth_on());
    assert(key==jp_now.key && form==jp_now.form && program==jp_data[JP2_HEAD+5] && position==jp_pos && seed==jp_seed);
    checking=1;
    command(1,53,100);assert(owner(1) && owner(1)->s.note==65 && active[0][65]);
    render(600);assert(owner(1) && owner(1)->gate); /* >3 seconds: genuinely held, not a long fixed gate. */
    en_jam_set(JAM_KEYS_OCTAVE,-2);render(1);command(1,53,0);assert(!held_count() && !active[0][65]);
    command(1,53,100);assert(owner(1)->s.note==29);command(1,53,0);
    en_jam_set(JAM_KEYS_OCTAVE,1);render(1);
    /* Independent owners share one outgoing MIDI pitch. */
    on=key_ons;command(1,53,100);command(193,65,96);assert(held_count()==2 && key_ons==on+1);
    command(1,53,0);assert(active[0][65]);command(193,65,0);assert(!active[0][65]);
    /* A naturally retired percussive voice still owns its required MIDI off. */
    command(188,60,100);owner(188)->used=0;owner(188)->gate=0;command(188,60,0);assert(!active[0][60]);
    command(188,60,100);owner(188)->used=0;owner(188)->gate=0;command(189,61,100);
    assert(!active[0][60] && active[0][61]);command(188,60,0);assert(active[0][61]);command(189,61,0);
    /* All panel keys remain chromatic; the sixth attack steals only keys. */
    for(k=0;k<27;k++){command(1+(int)k,53+(int)k,100);assert(owner(1+k)->s.note==65+k);}
    assert(held_count()==5);for(k=0;k<27;k++)command(1+(int)k,53+(int)k,0);assert(!held_count());
    command(1,53,100);command(2,57,100);on=key_ons;
    en_play(1);render(1200);assert(en_playing && held_count()==2 && key_ons==on && bass_ons && drum_ons);
    assert(fmx[4].part==1 && !fmx[4].owner);position=jp_pos;seed=jp_seed;
    en_play(0);render(1);assert(!en_playing && held_count()==2 && active[0][65] && active[0][69]);
    en_play(1);render(1);assert(held_count()==2 && jp_pos>=position && jp_seed==seed);
    en_play(0);render(1);
    /* Program commits flush held old-patch notes before the next attack. */
    en_jam_set(JAM_KEYS_PROGRAM,1);render(1);command(3,60,100);
    assert(held_count()==1 && owner(3) && !active[0][65] && !active[0][69]);
    en_jam_set(JAM_SPACE,67);render(1);assert(par[P_REVERB]==67 && en_jam_option(JAM_SPACE)==67);
    for(k=0;k<jp_data[8];k++)if(jp_u16(jp_data+JP_SOUND+4*k)==P_REVERB)break;
    assert(k<jp_data[8] && jp_u16(jp_data+JP_SOUND+4*k+2)==67);
    position=jp_pos;seed=jp_seed;en_jam_set(JAM_SYNTH,0);render(1);
    assert(!jp_synth_on() && !held_count() && position==jp_pos && seed==jp_seed && key==jp_now.key && form==jp_now.form);
    for(k=0;k<128;k++)assert(!active[0][k]);
    assert(key_ons==key_offs);
    en_jam_set(JAM_SYNTH,1);render(1);command(1,53,100);
    en_jam_set(JAM_ON,0);render(1);assert(!held_count() && !active[0][65]);
    checking=0;en_jam_set(JAM_ON,1);en_jam_set(JAM_SYNTH,1);render(1);checking=1;
    command(1,53,100);fm_set_patch(fm_cur);assert(held_count()==1);
    fm_set_patch(1);assert(!held_count() && !active[0][65]);
    assert(nonzero>1000 && !fullscale);
    printf("Synth Jam: held chromatic keys, raw MIDI, coalesced ownership, stealing/retirement, protected bass, backing transport, program/mode exit, persistent Space; %llu nonzero samples, zero fullscale OK\n",(unsigned long long)nonzero);
    return 0;
}
