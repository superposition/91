/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/amb.c"
#include "songbook_v2_fixtures.inc"
static unsigned checking,active[3][128],key_ons,bass_ons;
static int wanted_keys,wanted_bass;
void en_midi_out(uint32_t status,uint32_t note,uint32_t velocity)
{
    unsigned channel=status&15u,kind=status&0xf0u;
    if(!checking || (channel!=0 && channel!=2) || note>=128)return;
    if(kind==0xb0 && note==123){memset(active[channel],0,sizeof active[channel]);return;}
    if(kind==0x80 || (kind==0x90 && !velocity)){active[channel][note]=0;return;}
    if(kind!=0x90)return;
    assert(!active[channel][note]);active[channel][note]=1;
    if(channel==0) {
        const jp_beat_t *e=&jp_phrase[jp_current].beat[en_jam_beat];
        assert((int)note==(int)e->note[key_ons%4]+12*wanted_keys);key_ons++;
    } else {
        unsigned velocity2,gate;const jp_phrase_t *ph=&jp_phrase[jp_current];
        int expected=jp_bass_note(jp_data,&jp_now,ph,jp_now.hold?ph:&jp_phrase[jp_current^1],
                                 en_jam_bar,jp_pos%16,&velocity2,&gate);
        assert(expected>=0 && (int)note==expected+12*wanted_bass);bass_ons++;
    }
}
int main(void)
{
    uint8_t pack[JP_MAX],programs[1024],keys_profile[155],bass_profile[155];
    unsigned n,i,k;int key,bass_oct;int32_t audio[512];jp_phrase_t phrase;
    const uint8_t *source=en_jam_builtin(0,&n);
    memcpy(pack,JP_LEGACY_SONG_0,sizeof JP_LEGACY_SONG_0);
    pack[JP2_HEAD+23]=2;pack[JP2_HEAD+24]=254;
    assert(jp_valid(pack,sizeof JP_LEGACY_SONG_0));
    assert(jp_play_note(pack,0,60)==60 && jp_play_note(pack,1,36)==36);
    assert(jp_v3(source));memcpy(pack,source,n);
    pack[JP2_HEAD+23]=253;assert(!jp_valid(pack,n));pack[JP2_HEAD+23]=3;assert(!jp_valid(pack,n));
    memcpy(pack,source,n);pack[JP2_HEAD+24]=253;assert(!jp_valid(pack,n));
    memcpy(pack,source,n);pack[JP2_HEAD+23]=0;pack[JP2_HEAD+24]=0;assert(jp_valid(pack,n));
    for(i=0;i<pack[7];i++){pack[JP_DRUM+i*16+12]=1;pack[JP_DRUM+i*16+13]=1;}
    en_init();assert(en_jam_upload(pack,n));en_jam_set(JAM_ON,1);en_play(1);en_render(audio,256,2048);
    memcpy(programs,jp_data+JP2_VOICE,sizeof programs);
    memcpy(keys_profile,fm_ed,sizeof keys_profile);memcpy(bass_profile,fm_bass_ed,sizeof bass_profile);
    phrase=jp_phrase[jp_current];checking=1;
    for(key=-2;key<=2;key++)for(bass_oct=-2;bass_oct<=2;bass_oct++) {
        unsigned before_keys=key_ons,before_bass=bass_ons;uint32_t age=fm_age;
        wanted_keys=key;wanted_bass=bass_oct;
        en_jam_set(JAM_KEYS_OCTAVE,key);en_jam_set(JAM_BASS_OCTAVE,bass_oct);
        en_render(audio,256,2048);
        assert(en_jam_option(JAM_KEYS_OCTAVE)==key && en_jam_option(JAM_BASS_OCTAVE)==bass_oct);
        assert(!memcmp(&phrase,&jp_phrase[jp_current],sizeof phrase));
        jp_pos=0;jp_tick(0);
        assert(key_ons==before_keys+4 && bass_ons==before_bass+1);
        {unsigned keys_active=0,bass_active=0;
         for(i=0;i<128;i++){keys_active+=active[0][i];bass_active+=active[2][i];}
         assert(keys_active==4 && bass_active==1);}
        for(i=0;i<FM_NV;i++)if(fmx[i].used && fmx[i].age>age) {
            int note=fmx[i].s.note;assert(note>=0 && note<128);
            assert(active[fmx[i].part?2:0][note]);
        }
        assert(!memcmp(programs,jp_data+JP2_VOICE,sizeof programs));
        assert(!memcmp(keys_profile,fm_ed,sizeof keys_profile) && !memcmp(bass_profile,fm_bass_ed,sizeof bass_profile));
    }
    en_jam_set(JAM_KEYS_OCTAVE,-100);en_jam_set(JAM_BASS_OCTAVE,100);en_render(audio,256,2048);
    assert(en_jam_option(JAM_KEYS_OCTAVE)==-2 && en_jam_option(JAM_BASS_OCTAVE)==2);
    play_stop();for(k=0;k<3;k++)for(i=0;i<128;i++)assert(!active[k][i]);
    puts("Octaves: all 25 independent registers, exact audio/MIDI notes, live note-off pairing, unchanged planner/VMEM/profiles, validation and bounds OK");
    return 0;
}
