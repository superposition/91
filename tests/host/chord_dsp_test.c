/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/amb.c"
static unsigned check_midi,midi_count,midi_notes[4],bass_count;
void en_midi_out(uint32_t status,uint32_t note,uint32_t velocity)
{
    if(!check_midi || !velocity)return;
    if(status==0x90) {
        const jp_beat_t *e=&jp_phrase[jp_current].beat[en_jam_beat];
        assert(note==(unsigned)jp_play_note(jp_data,0,e->note[midi_count%4]));midi_notes[midi_count%4]=note;midi_count++;
    }
    if(status==0x92) {
        unsigned vel,gate;
        const jp_phrase_t *ph=&jp_phrase[jp_current];
        int expected=jp_bass_note(jp_data,&jp_now,ph,jp_now.hold?ph:&jp_phrase[jp_current^1],
                                  en_jam_bar,jp_pos%16,&vel,&gate);
        assert(expected>=0 && note==(unsigned)jp_play_note(jp_data,1,expected));bass_count++;
    }
}
static void attacks_and_labels(void)
{
    uint8_t p[JP_MAX];unsigned n,q,mv,step,i;int32_t out[512];
    const uint8_t *src=en_jam_builtin(0,&n);assert(jp_v3(src));
    for(q=0;q<JP3_QUALITIES;q++)for(mv=0;mv<4;mv++) {
        char name[32],wanted[32];memcpy(p,src,n);
        p[JP_NODE]=1;p[JP_NODE+7]=(uint8_t)q;memset(p+JP2_CHART,0,256);
        for(i=0;i<p[7];i++){p[JP_DRUM+i*16+12]=1;p[JP_DRUM+i*16+13]=1;}
        check_midi=0;play_stop();en_init();assert(en_jam_valid(p,n));assert(en_jam_upload(p,n));
        en_jam_set(JAM_KEY,0);en_jam_set(JAM_MOVE,(int)mv);en_jam_set(JAM_ON,1);
        en_render(out,256,2048);assert(jp_dx() && jp_v3(jp_data));
        jp_pos=0;
        midi_count=0;bass_count=0;check_midi=1;
        for(step=0;step<16;step++) {
            unsigned before=midi_count;uint32_t age=fm_age;
            jp_tick((int)step);
            if(step==0 || step==8) {
                unsigned count=0,seen=0;
                assert(midi_count==before+4);
                for(i=0;i<FM_NV;i++)if(fmx[i].used && !fmx[i].part && fmx[i].age>age) {
                    unsigned j;count++;
                    for(j=0;j<4;j++)if(fmx[i].s.note==midi_notes[j])seen|=1u<<j;
                }
                assert(count==4 && seen==15);
                assert(fmx[4].part==1 || !fmx[4].used);
            } else assert(midi_count==before);
            en_jam_name(0,name);snprintf(wanted,sizeof wanted,"C#%s",jp_chord_suffix(q));
            assert(!strcmp(name,wanted));
        }
        check_midi=0;
    }
}
static void actual_factory_render(void)
{
    unsigned song,n,i,j;int32_t out[512];
    for(song=0;song<3;song++) {
        uint64_t nonzero=0,fullscale=0;unsigned sections=song?2:1;
        unsigned limit=(unsigned)((uint64_t)sections*16*4*60*EN_SR/94+EN_SR);
        const uint8_t *p=en_jam_builtin(song,&n);assert(jp_v3(p) && en_jam_valid(p,n));
        check_midi=0;play_stop();en_init();assert(en_jam_upload(p,n));en_jam_set(JAM_MOVE,0);
        en_set(P_TEMPO,94);
        en_jam_set(JAM_ON,1);en_play(1);midi_count=0;bass_count=0;check_midi=1;
        for(i=0;i<(limit+255)/256;i++) {
            en_render(out,256,2048);
            for(j=0;j<512;j++) {
                int64_t v=out[j];if(v<0)v=-v;
                if(v)nonzero++;
                if(v>=8388607)fullscale++;
            }
            if(jp_section>=sections)break;
        }
        assert(midi_count>=8 && !(midi_count%4) && bass_count>0 && nonzero>1000);
        assert(jp_dx() && jp_v3(jp_data) && jp_section>=sections && !fullscale);
        printf("JMP3 actual DSP factory %u: %u bars at94BPM, %u chord notes, %u bass attacks, nonzero audio, zero fullscale; exact MIDI tones OK\n",song,sections*16,midi_count,bass_count);
        check_midi=0;
    }
}
int main(void)
{
    attacks_and_labels();actual_factory_render();
    puts("JMP3 DSP: all17qualities/all4movement modes retain four keyboard voices, protected bass, exact chord labels; allthree actualfactory renders pass");
    return 0;
}
