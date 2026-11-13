/* SPDX-License-Identifier: GPL-3.0-only */
/* Load the actual saved-slot binaries and render their first eight-bar chart.
 * This is a level/activity check, not a subjective assessment of the music. */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#include "../../firmware/src/dsp/amb.c"
static unsigned notes[16];
void en_midi_out(uint32_t status,uint32_t note,uint32_t velocity)
{
    (void)note;if((status&0xf0)==0x90 && velocity)notes[status&15]++;
}
int main(void)
{
    static const char *const paths[]={"packs/04-blue-bossa-deep.jmp","packs/05-leaves-drawbars.jmp",
        "packs/06-satin-muted-guitar.jmp","packs/07-major-minor-lab.jmp"};
    static const unsigned keys[]={0,4,0,0};
    unsigned song;
    for(song=0;song<4;song++) {
        uint8_t data[JP_MAX+1];int32_t out[512];FILE *file=fopen(paths[song],"rb");
        unsigned n,i,j,maxbar=0,frames=0,limit=(unsigned)((uint64_t)8*4*60*EN_SR/94+EN_SR);
        uint64_t nonzero=0,fullscale=0,near_ceiling=0;int64_t peak=0;
        double energy=0,max_comp=0,max_lim=0;
        assert(file);n=(unsigned)fread(data,1,sizeof data,file);assert(!ferror(file));fclose(file);
        assert(jp_v3(data) && en_jam_valid(data,n));
        if(en_playing)play_stop();
        en_init();memset(notes,0,sizeof notes);
        assert(en_jam_upload(data,n));en_jam_set(JAM_KEY,(int)keys[song]);
        en_jam_set(JAM_BARS,8);en_jam_set(JAM_FORM,0);en_jam_set(JAM_MOVE,0);
        en_set(P_TEMPO,94);en_jam_set(JAM_ON,1);en_play(1);
        for(i=0;i<(limit+255)/256;i++) {
            en_render(out,256,2048);frames+=256;
            if(en_jam_bar>maxbar)maxbar=en_jam_bar;
            if(en_comp_gr>max_comp)max_comp=en_comp_gr;
            if(en_lim_gr>max_lim)max_lim=en_lim_gr;
            for(j=0;j<512;j++) {
                int64_t v=out[j];if(v<0)v=-v;
                if(v>peak)peak=v;
                if(v)nonzero++;
                if(v>=8388607)fullscale++;
                if(v>=8217000)near_ceiling++;
                energy+=(double)v*(double)v;
            }
            if(jp_section>=1)break;
        }
        assert(jp_dx() && jp_len==n && !memcmp(jp_data,data,n));
        assert(notes[0]>0 && notes[2]>0 && notes[9]>0 && nonzero>1000 && maxbar==7 && jp_section>=1 && !fullscale);
        printf("{\"file\":\"%s\",\"name\":\"%s\",\"bars\":8,\"bpm\":94,\"frames\":%u,\"keys\":%u,\"bass\":%u,\"drums\":%u,\"peak_dbfs\":%.3f,\"rms_dbfs\":%.3f,\"nonzero_samples\":%llu,\"fullscale_samples\":%llu,\"near_soft_ceiling_samples\":%llu,\"max_compressor_reduction_db\":%.3f,\"max_limiter_reduction_db\":%.3f}\n",
            paths[song],data+16,frames,notes[0],notes[2],notes[9],20*log10((double)peak/8388608.0),
            20*log10(sqrt(energy/(frames*2.0))/8388608.0),(unsigned long long)nonzero,(unsigned long long)fullscale,
            (unsigned long long)near_ceiling,max_comp,max_lim);
    }
    return 0;
}
