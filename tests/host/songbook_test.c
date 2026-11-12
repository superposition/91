/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <string.h>
#include <stdio.h>
#include "../../firmware/src/dsp/amb.c"
static unsigned notes[16],clocks;
void en_midi_out(uint32_t st,uint32_t n,uint32_t v){(void)n;if(st==0xf8)clocks++;if((st&0xf0)==0x90 && v)notes[st&15]++;}
int main(void)
{
    unsigned song,key,minor,bars,i,j,n;uint32_t seed=4;uint8_t bad[JP_MAX];int32_t out[512];
    for(song=0;song<3;song++) {
        const uint8_t *data=en_jam_builtin(song,&n);assert(en_jam_valid(data,n));
        memcpy(bad,data,n);assert(!en_jam_valid(bad,1120));bad[JP2_CHART]=255;assert(!en_jam_valid(bad,n));
        for(key=0;key<12;key++)for(minor=0;minor<2;minor++)for(bars=8;bars<=16;bars+=8) {
            jp_settings_t s={1,key,minor,bars,0,3,60,0,0,0};jp_phrase_t a,b,c;
            jp_plan(data,&s,0,0,&a,&seed);jp_plan(data,&s,1,&a,&b,&seed);jp_plan(data,&s,3,&b,&c,&seed);
            assert(a.bars==bars && c.beat[bars*4-1].node==data[minor?12:9]);
            for(i=0;i<bars*4;i++) {
                const uint8_t *node=jp_node(data,a.beat[i].node),*scale=data+JP_SCALE+node[2]*8;
                for(j=0;j<4;j++) {
                    unsigned pc=(a.beat[i].note[j]+24-key-node[1])%12,k,found=0;
                    assert(a.beat[i].note[j]>=48 && a.beat[i].note[j]<=72);
                    for(k=0;k<8;k++)if(scale[k]==pc)found=1;
                    assert(found);
                    if(j)assert(a.beat[i].note[j]>a.beat[i].note[j-1]);
                }
                if(a.beat[i].passing&16)assert(i%4==3 && !(a.beat[i+1].passing&16));
            }
            for(i=0;i<bars;i++)for(j=0;j<16;j++){
                unsigned v,g,pat=(a.motif+(i==bars-1u))%data[6];
                int note=jp_bass_note(data,&s,&a,&b,i,j,&v,&g),iv=(int8_t)data[JP_BASS+pat*36+j];
                const uint8_t *node=jp_node(data,a.beat[i*4+j/4].node);
                assert(note<0 || (note>=28 && note<=52));
                if(note>=0 && (iv==0 || iv==12))assert(note%12==(int)((key+node[0])%12));
                if(note>=0 && iv==-125 && node[3]==7)assert(note%12==(int)((key+node[0]+8)%12));
            }
        }
    }
    en_init();{const uint8_t *p=en_jam_builtin(0,&n);assert(en_jam_upload(p,n));}
    en_jam_set(JAM_ON,1);en_play(1);
    for(i=0;i<6000;i++)en_render(out,256,2048);
    assert(jp_dx() && notes[0]>10 && notes[2]>10 && notes[4]>0);
    assert(memcmp(fm_ed,fm_bass_ed,FP_SIZE));
    for(i=0;i<FM_NV;i++)if(fmx[i].used)assert((i==4)==(fmx[i].part==1));
    en_jam_set(JAM_TOUCH,20);en_jam_set(JAM_KEYS_PROGRAM,1);en_jam_set(JAM_SWELL,45);
    for(i=0;i<500;i++)en_render(out,256,2048);
    assert(en_jam_option(JAM_TOUCH)==20 && en_jam_option(JAM_KEYS_PROGRAM)==1 && en_jam_option(JAM_SWELL)==45);
    assert(!jp_patch_dirty && fm_ed[FP_KVS]==1);
    assert(!memcmp(fm_ed+FP_NAME,"Bark EP",7));
    /* A bass attack must not reset the keys' independent LFO. */
    fm_lfo.dly=321;fm_bass_lfo.dly=987;fmx[4].gate=0;
    fm_jam_note(1,36,.7f,4000);
    assert(fm_lfo.dly==321 && fm_bass_lfo.dly==0);
    {uint32_t age=fmx[4].age;
     for(i=0;i<12;i++)fm_jam_note(0,48+(int)i,.7f,4000);
     assert(fmx[4].age==age && fmx[4].part==1);}
    assert(en_jam_valid(en_jam_data(&n),n));
    en_play(0);en_render(out,256,2048);assert(!en_playing && jp_swell_phase==1.0f);
    /* Coalescing intermediate MIDI clocks must retain tempo and every musical
     * sixteenth, including irregular render sizes at the maximum tempo. */
    en_init();clocks=0;en_set(P_TEMPO,190);en_jam_set(JAM_ON,1);en_play(1);
    {unsigned total=0,target=30u*EN_SR;const unsigned sizes[]={256,91,128,253};
     while(total<target){unsigned size=sizes[total%4];if(size>target-total)size=target-total;en_render(out,size,2048);total+=size;}
     {unsigned expected=1u+(unsigned)((uint64_t)total*190u*24u/(60u*EN_SR));assert(clocks+1>=expected && clocks<=expected+1);}
     assert(jp_pos==((clocks+5u)/6u)%256u);}
    puts("songbook: 144 key/mode/length cases, authored cadence, borrowed notes, register, independent DX parts, protected bass, swells and live profiles OK");
    return 0;
}
