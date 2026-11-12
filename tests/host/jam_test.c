/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/amb.c"
static unsigned ons[16],offs[16];
void en_midi_out(uint32_t st,uint32_t n,uint32_t v) {(void)n;if((st&0xf0)==0x90 && v) ons[st&15]++;if((st&0xf0)==0x80)offs[st&15]++;}
static void render_steps(unsigned count)
{
    int32_t out[512];unsigned target=en_bars+count;
    while(en_bars<target) en_render(out,256,2048);
}
static void clean_patch_test(void)
{
    int32_t out[512];int16_t expected[P_NPARAMS];unsigned i;
    en_init();en_jam_set(JAM_ON,1);en_render(out,256,2048);
    memcpy(expected,par,sizeof expected);
    en_init();
    /* Reproduce switching from a bright, pitch-modulated, shimmering piece. */
    apply(P_PL_DEPTH,100);apply(P_PL_DEST,3);apply(P_DL_DEPTH,100);apply(P_DL_DEST,1);
    apply(P_PL_CHO,100);apply(P_RV_TYPE,1);apply(P_REVERB,90);apply(P_SIZE,100);
    apply(P_BRIGHT,100);apply(P_DETUNE,100);apply(P_LANE_TUNE(0),40);
    apply(P_TEMPO,123);apply(P_TUNE,7);fx_req=FXB(FX_TAPE);fxk_v[0]=50;
    intro_len=60*EN_SR;
    en_jam_set(JAM_ON,1);en_play(1);render_steps(8);
    for(i=0;i<P_NPARAMS;i++) if(i!=P_TEMPO && !en_par_global((int)i)) assert(par[i]==expected[i]);
    assert(par[P_TEMPO]==123 && par[P_TUNE]==7);
    assert(!fx_req && !fx_on && !fxk_v[0] && en_intro_k==1.0f);
    assert(pad_pitch==0.0f && pad_filt==0.0f && par[P_DL_DEPTH]==0);
    assert(par[P_REVERB]==10 && par[P_SIZE]==12 && !par[P_ECHO] && !par[P_PL_CHO]);
    assert(DRONE_SOUND[par[P_DR_INST]].osc==DO_SINE);
    /* A queued form/key change must preserve live sound edits and tuning. */
    en_set(P_BRIGHT,23);en_jam_set(JAM_KEY,5);en_jam_set(JAM_FORM,2);
    while(en_jam_pending || jp_now.key!=5)render_steps(1);
    assert(par[P_BRIGHT]==23 && par[P_TUNE]==7 && par[P_ROOT]==5);
    render_steps(160);assert(jp_now.key==5 && par[P_ROOT]==5);
    en_play(0);en_render(out,256,2048);
    en_jam_set(JAM_FORM,3);en_render(out,256,2048);
    assert(jp_now.form==3 && par[P_BRIGHT]==23 && par[P_ROOT]==5);
    memset(ons,0,sizeof ons);memset(offs,0,sizeof offs);
}
int main(void)
{
    unsigned key,minor,bars,i,j;uint32_t seed=91;uint8_t data[JP_MAX];jp_phrase_t a,b;
    assert(en_jam_valid(JP_FACTORY,sizeof JP_FACTORY));
    clean_patch_test();
    for(key=0;key<12;key++)for(minor=0;minor<2;minor++)for(bars=8;bars<=16;bars+=8) {
        jp_settings_t s={1,(uint8_t)key,(uint8_t)minor,(uint8_t)bars,1,3,60,0,0,0};
        jp_plan(JP_FACTORY,&s,0,0,&a,&seed);jp_plan(JP_FACTORY,&s,1,&a,&b,&seed);
        assert(a.bars==bars && b.motif==a.motif && b.role==1);
        for(i=0;i<bars*4;i++) {
            const uint8_t *node=JP_FACTORY+JP_NODE+a.beat[i].node*8;
            const uint8_t *scale=JP_FACTORY+JP_SCALE+node[2]*8;
            for(j=0;j<4;j++) {
                unsigned pc=(a.beat[i].note[j]+24-key-node[1])%12,k,found=0;
                assert(a.beat[i].note[j]>=48 && a.beat[i].note[j]<=76);
                for(k=a.beat[i].passing;k<8;k+=2) if(scale[k]==pc)found=1;
                assert(found);
                if(j) assert(a.beat[i].note[j]>a.beat[i].note[j-1]);
            }
        }
        assert(a.beat[(bars-3)*4].node==JP_FACTORY[(minor?12:9)+1]);
        assert(a.beat[(bars-2)*4].node==JP_FACTORY[(minor?12:9)+2]);
        assert(a.beat[(bars-1)*4].node==JP_FACTORY[minor?12:9]);
        for(i=0;i<bars;i++)for(j=0;j<16;j++){unsigned v,g;int n=jp_bass_note(JP_FACTORY,&s,&a,&b,i,j,&v,&g);assert(n==-1 || (n>=31 && n<=55));}
    }
    memcpy(data,JP_FACTORY,sizeof JP_FACTORY);data[5]=17;assert(!en_jam_valid(data,sizeof JP_FACTORY));
    memcpy(data,JP_FACTORY,sizeof JP_FACTORY);data[JP_EDGE]=0;memset(data+JP_EDGE,0,16);assert(!en_jam_valid(data,sizeof JP_FACTORY));
    en_init();assert(en_jam_upload(JP_FACTORY,sizeof JP_FACTORY));en_jam_set(JAM_ON,1);en_set(P_TEMPO,190);en_play(1);render_steps(1);
    a=jp_phrase[jp_current];en_jam_set(JAM_HOLD,1);render_steps(256);assert(!memcmp(&a,&jp_phrase[jp_current],sizeof a));
    en_jam_set(JAM_FORM,2);en_jam_set(JAM_BARS,8);render_steps(16);assert(jp_now.bars==16 && en_jam_pending);
    while(jp_now.bars==16)render_steps(1);
    assert(jp_now.form==2 && !en_jam_pending && en_jam_bar==0);
    memcpy(data,JP_FACTORY,sizeof JP_FACTORY);data[16]='X';
    assert(en_jam_upload(data,sizeof JP_FACTORY));render_steps(1);assert(en_jam_busy());
    while(en_jam_busy())render_steps(1);
    assert(jp_data[16]=='X' && en_jam_bar==0);
    assert(ons[0]>100 && ons[2]>40 && ons[9]>40 && ons[3]==0 && ons[1]==0);
    en_play(0);{int32_t out[512];en_render(out,256,2048);}assert(!en_playing);
    puts("jam: clean patch isolation, stable key, 48 transpositions, sixth/diminished membership, cadence, bass range, hold, queued changes and MIDI OK");
    return 0;
}
