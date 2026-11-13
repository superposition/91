/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/jam.h"
#include "../../firmware/src/dsp/jam_factory.inc"

/* A fixed v2 fixture independent of the replaceable factory song bank. */
static void song_fixture(uint8_t *p)
{
    unsigned i;
    memset(p,0,JP_MAX);memcpy(p,JP_FACTORY,JP_MIN);
    p[3]='2';p[4]=2;memcpy(p+JP2_HEAD,"SNG2",4);
    p[JP2_HEAD+4]=1;p[JP2_HEAD+8]=40;p[JP2_HEAD+9]=40;
    p[JP2_HEAD+11]=2;p[JP2_HEAD+16]=65;p[JP2_HEAD+17]=58;
    for(i=0;i<p[5];i++)p[JP_NODE+i*8+6]=9;
    for(i=0;i<256;i++)p[JP2_CHART+i]=(uint8_t)((i/128)*7+(i%7));
    assert(jp_valid(p,JP2_META));
}
static uint32_t hash_bytes(uint32_t h,const void *v,unsigned n)
{
    const uint8_t *p=v;while(n--)h=(h^*p++)*16777619u;return h;
}
static uint32_t legacy_hash(const uint8_t *p)
{
    unsigned k,m,b,f,mv,s,i,j;uint32_t h=2166136261u;
    for(k=0;k<12;k++)for(m=0;m<2;m++)for(b=8;b<=16;b+=8)
    for(f=0;f<4;f++)for(mv=0;mv<4;mv++) {
        jp_settings_t st={1,k,m,b,f,mv,60,0,0,0};
        jp_phrase_t a={0},next={0};uint32_t seed=91;
        for(s=0;s<4;s++) {
            jp_plan(p,&st,s,s?&a:0,&next,&seed);
            h=hash_bytes(h,&next,sizeof next);h=hash_bytes(h,&seed,sizeof seed);
            a=next;
            for(i=0;i<b;i++)for(j=0;j<16;j++) {
                unsigned vel,gate;int note=jp_bass_note(p,&st,&a,&next,i,j,&vel,&gate);
                h=hash_bytes(h,&note,sizeof note);h=hash_bytes(h,&vel,sizeof vel);h=hash_bytes(h,&gate,sizeof gate);
            }
        }
    }
    return h;
}
static void literal_fixture(uint8_t *p)
{
    unsigned i,j;
    song_fixture(p);p[3]='3';p[4]=3;p[5]=17;p[JP2_HEAD+3]='3';
    for(i=0;i<17;i++) {
        uint8_t *a=p+(i<16?JP_NODE+i*8:JP2_EXTRA+(i-16)*8);
        memset(a,0,8);a[0]=(uint8_t)(i%12);a[7]=(uint8_t)i;
    }
    /* Each bank differs. Both modes intentionally share mode-zero nodes. */
    for(i=0;i<8;i++)for(j=0;j<32;j++)p[JP2_CHART+i*32+j]=(uint8_t)((i*3+j)%17);
    assert(jp_valid(p,JP2_META));
}
static const uint8_t expected[17][4]={
    {0,4,7,11},{0,3,7,10},{0,4,7,10},{0,3,6,10},{0,3,6,9},
    {0,4,7,9},{0,3,7,9},{0,5,7,10},{0,3,7,11},{0,4,8,10},
    {4,7,10,13},{4,7,11,14},{3,7,10,14},{4,7,10,14},{4,8,10,13},
    {0,4,7,12},{0,3,7,12}
};
static void check_chord(const jp_beat_t *e,unsigned root,unsigned quality,unsigned drop)
{
    unsigned i,want[12]={0},got[12]={0};
    for(i=0;i<4;i++) {
        want[(root+expected[quality][i])%12]++;
        got[e->note[i]%12]++;
        assert(e->note[i]>=(drop?45:48) && e->note[i]<=76);
        if(i)assert(e->note[i]>e->note[i-1]);
    }
    assert(!memcmp(want,got,sizeof want));assert(!e->passing);
    /* Seventh/extension drop-2 voicings span more than an octave; their
     * closed counterparts fit inside one. Triads explicitly double a root. */
    if(quality<15) {
        if(drop)assert(e->note[3]-e->note[0]>=12);
        else assert(e->note[3]-e->note[0]<12);
    }
}
static void literal_tones(void)
{
    uint8_t p[JP_MAX];unsigned q,key,mv,v,i,root;
    literal_fixture(p);
    for(q=0;q<17;q++)for(key=0;key<12;key++)for(root=0;root<12;root++)
    for(mv=0;mv<4;mv++)for(v=0;v<2;v++) {
        uint8_t *a=p+(q<16?JP_NODE+q*8:JP2_EXTRA+(q-16)*8);
        jp_settings_t s={1,key,0,16,0,mv,60,0,0,0};
        jp_phrase_t ph={0};uint32_t seed=91;
        a[0]=(uint8_t)root;p[JP2_HEAD+21]=(uint8_t)v;memset(p+JP2_CHART,(int)q,256);
        jp_plan(p,&s,3,0,&ph,&seed);
        for(i=0;i<64;i++)check_chord(&ph.beat[i],(root+key)%12,q,mv==2 || (!mv && v));
    }
}
static void literal_charts(void)
{
    uint8_t p[JP_MAX];unsigned key,mode,bars,form,mv,sec,i;
    literal_fixture(p);
    for(key=0;key<12;key++)for(mode=0;mode<2;mode++)for(bars=8;bars<=16;bars+=8)
    for(form=0;form<4;form++)for(mv=0;mv<4;mv++) {
        jp_settings_t s={1,key,mode,bars,form,mv,60,0,0,0};
        jp_phrase_t prev={0},ph={0},repeat={0};uint32_t seed=91,again;
        for(sec=0;sec<8;sec++) {
            const uint8_t *f=p+JP_FORM+form*32;
            unsigned role=f[17+sec%f[16]],part=role==1 || role==2;
            unsigned off=JP2_CHART+((mode*2+(bars==16))*2+part)*32;
            again=seed;jp_plan(p,&s,sec,sec?&prev:0,&ph,&seed);
            jp_plan(p,&s,sec,sec?&prev:0,&repeat,&again);
            assert(seed==again && !memcmp(&ph,&repeat,sizeof ph));
            assert(ph.bars==bars && ph.role==role);
            for(i=0;i<bars*4;i++) {
                unsigned node=p[off+i/2];const uint8_t *a=jp_node(p,node);
                /* Includes the last two beats of returning A sections, where
                 * v2 deliberately rewrote the chart to the tonic. */
                assert(ph.beat[i].node==node);
                check_chord(&ph.beat[i],(a[0]+key)%12,a[7],mv==2);
            }
            prev=ph;
        }
    }
}
static void literal_bass(void)
{
    static const uint8_t third[]={4,3,4,3,3,4,3,5,3,4,4,4,3,4,4,4,3};
    static const uint8_t fifth[]={7,7,7,6,6,7,7,7,7,8,7,7,7,7,8,7,7};
    uint8_t p[JP_MAX];unsigned q,key,step;
    literal_fixture(p);p[6]=1;
    for(step=0;step<16;step++) {p[JP_BASS+step]=(uint8_t)(step&1?-125:-126);p[JP_BASS+16+step]=90;}
    for(q=0;q<17;q++)for(key=0;key<12;key++) {
        jp_settings_t s={1,key,0,8,0,0,60,0,0,0};jp_phrase_t ph={0};uint32_t seed=91;
        const uint8_t *a=jp_node(p,q);memset(p+JP2_CHART,(int)q,256);jp_plan(p,&s,0,0,&ph,&seed);
        for(step=0;step<16;step++) {
            unsigned vel,gate;int n=jp_bass_note(p,&s,&ph,&ph,0,step,&vel,&gate);
            assert(n>=28 && n<=52 && vel==90 && gate==p[JP_BASS+32]);
            assert(n%12==(int)((key+a[0]+(step&1?fifth[q]:third[q]))%12));
        }
    }
}
static void literal_validation(void)
{
    uint8_t p[JP_MAX];literal_fixture(p);
    p[JP_NODE+7]=17;assert(!jp_valid(p,JP2_META));p[JP_NODE+7]=0;
    p[JP2_HEAD+21]=2;assert(!jp_valid(p,JP2_META));p[JP2_HEAD+21]=1;
    p[JP2_HEAD+3]='2';assert(!jp_valid(p,JP2_META));p[JP2_HEAD+3]='3';
    p[JP2_CHART]=17;assert(!jp_valid(p,JP2_META));p[JP2_CHART]=0;
    p[3]='2';assert(!jp_valid(p,JP2_META));p[3]='3';
    assert(!jp_valid(p,JP2_META-1));assert(jp_valid(p,JP2_META));
    assert(sizeof(jp_phrase_t)==388 && sizeof(jp_settings_t)==10);
    assert(!strcmp(jp_chord_suffix(0),"maj7") && !strcmp(jp_chord_suffix(2),"7"));
    assert(!strcmp(jp_chord_suffix(3),"m7b5") && !strcmp(jp_chord_suffix(14),"7alt"));
    assert(!strcmp(jp_chord_movement(2),"DROP2"));
}
int main(void)
{
    uint8_t p[JP_MAX];song_fixture(p);
    assert(legacy_hash(JP_FACTORY)==0x2dacbdb5u);assert(legacy_hash(p)==0x3d113a57u);
    literal_tones();literal_charts();literal_bass();literal_validation();
    puts("JMP3: 17 qualities x 12 roots x 12 keys x 4 movement modes x 2 authored voicings; exact chart replay, drop2, bass, validation, determinism and unchanged v1/v2 golden hashes OK");
    return 0;
}
