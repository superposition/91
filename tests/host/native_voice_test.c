/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "../../firmware/src/dsp/amb.c"
#include "songbook_v2_fixtures.inc"
void en_midi_out(uint32_t status,uint32_t note,uint32_t velocity)
{(void)status;(void)note;(void)velocity;}

static void fixture(uint8_t *p)
{
    static const uint8_t sensitivities[2][6]={{0,1,2,3,6,7},{7,6,3,2,1,0}};
    unsigned role,op;
    memcpy(p,JP_LEGACY_SONG_0,sizeof JP_LEGACY_SONG_0);
    p[3]='3';p[4]=3;p[JP2_HEAD+3]='3';p[JP2_HEAD+5]=0;p[JP2_HEAD+6]=1;
    p[JP2_HEAD+16]=65;p[JP2_HEAD+18]=50;p[JP2_HEAD+19]=50;p[JP2_HEAD+20]=0;
    for(role=0;role<2;role++) {
        uint8_t *v=p+JP2_VOICE+128*role;
        v[114]=(uint8_t)(role?47:33);
        for(op=0;op<6;op++) {
            v[op*17+13]=(uint8_t)((v[op*17+13]&3u)|(sensitivities[role][op]<<2));
            v[op*17+3]=(uint8_t)(20+op);
        }
    }
    assert(jp_valid(p,sizeof JP_LEGACY_SONG_0));
}
static void original(const uint8_t *p,unsigned role,uint8_t *out)
{fm6_unpack(p+JP2_VOICE+128*role,out);}
static uint8_t *loaded(unsigned role){return role?fm_bass_ed:fm_ed;}
static void neutral_and_flag_independence(void)
{
    uint8_t p[JP_MAX],want[FP_SIZE+1];unsigned flags,role,op;
    fixture(p);
    for(flags=0;flags<4;flags++)for(role=0;role<2;role++) {
        p[JP2_HEAD+22]=(uint8_t)flags;original(p,role,want);
        if(!(flags&2))want[FP_LPMD]=0;
        if(!role && !(flags&1))for(op=0;op<6;op++)want[op*FP_OP+FP_KVS]=3;
        fm_jam_program(p,role);
        /* At flags3 the entire sanitized voice, not just KVS/PMD, survives. */
        assert(!memcmp(want,loaded(role),sizeof want));
    }
}
static void relative_touch(void)
{
    static const unsigned touch[]={0,42,43,49,50,51,57,58,100};
    static const int delta[]={-7,-1,0,0,0,0,0,1,7};
    uint8_t p[JP_MAX],want[FP_SIZE+1];unsigned i,op;
    fixture(p);p[JP2_HEAD+22]=3;
    for(i=0;i<sizeof touch/sizeof touch[0];i++) {
        p[JP2_HEAD+18]=(uint8_t)touch[i];original(p,0,want);
        for(op=0;op<6;op++) {
            int v=(int)want[op*FP_OP+FP_KVS]+delta[i];
            want[op*FP_OP+FP_KVS]=(uint8_t)(v<0?0:v>7?7:v);
        }
        fm_jam_program(p,0);assert(!memcmp(want,fm_ed,sizeof want));
        original(p,1,want);fm_jam_program(p,1);assert(!memcmp(want,fm_bass_ed,sizeof want));
    }
}
static void legacy_profile(void)
{
    uint8_t p[JP_MAX],want[FP_SIZE+1];unsigned version,role,op;
    fixture(p);
    for(version=2;version<=3;version++)for(role=0;role<2;role++) {
        p[3]=(uint8_t)('0'+version);p[4]=(uint8_t)version;p[JP2_HEAD+3]=p[3];
        p[JP2_HEAD+22]=(uint8_t)(version==2?3:0); /* v2 must ignore flags */
        p[JP2_HEAD+18]=28;p[JP2_HEAD+16]=48;p[JP2_HEAD+19]=44;p[JP2_HEAD+20]=37;
        original(p,role,want);want[FP_LPMD]=0;
        for(op=0;op<6;op++) {
            if(role)want[op*FP_OP+FP_R1]=77;
            else {
                want[op*FP_OP+FP_KVS]=1;
                want[op*FP_OP+FP_R1+3]+=8;
                if(!fm6_carrier(want[FP_ALG],6u-op) && want[op*FP_OP+FP_OL])want[op*FP_OP+FP_OL]--;
            }
        }
        assert(jp_valid(p,sizeof JP_LEGACY_SONG_0));
        fm_jam_program(p,role);assert(!memcmp(want,loaded(role),sizeof want));
    }
}
static void validation(void)
{
    uint8_t p[JP_MAX];unsigned flags;fixture(p);
    for(flags=0;flags<256;flags++) {
        p[JP2_HEAD+22]=(uint8_t)flags;
        assert(jp_valid(p,sizeof JP_LEGACY_SONG_0)==(flags<=3));
    }
    p[3]='2';p[4]=2;p[JP2_HEAD+3]='2';
    assert(jp_valid(p,sizeof JP_LEGACY_SONG_0)); /* legacy reserved bytes */
}
int main(void)
{
    neutral_and_flag_independence();relative_touch();legacy_profile();validation();
    puts("Native voice: complete neutral VMEM preservation, independent flags, per-operator relative touch/truncation/extremes, both roles, unchanged legacy profile and flags validation OK");
    return 0;
}
