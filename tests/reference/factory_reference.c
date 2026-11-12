/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile against the independently verified R01 engine, not maintained Rust. */
#include "amb.c"
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <limits.h>
#if EN_RUST_FACTORY
extern uint32_t fm1_rust_jam_stat(uint32_t index);
#endif
void en_midi_out(uint32_t a, uint32_t b, uint32_t c) { (void)a;(void)b;(void)c; }
static void str(const char *s) {
    putchar('"'); if(s) for(;*s;s++) { if(*s=='"'||*s=='\\')putchar('\\'); putchar(*s); } putchar('"');
}
static void ints(const int16_t *p,int n) { int i;putchar('[');for(i=0;i<n;i++)printf("%s%d",i?",":"",p[i]);putchar(']'); }
static void words(const uint16_t *p,int n) { int i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",p[i]);putchar(']'); }
static void bytes(const uint8_t *p,int n) { int i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",p[i]);putchar(']'); }
static int first=1;
static void names(const char *key,const char *const *p,int n) { int i;printf("%s",first?"":",");first=0;str(key);printf(":[");for(i=0;i<n;i++){if(i)putchar(',');str(p[i]);}putchar(']'); }
#define NAMES(n) names(#n,n,sizeof(n)/sizeof(n[0]))
int main(int argc,char **argv) {
    int g,i,k; int16_t par[P_NPARAMS]; uint16_t pool,pat[6][6]; uint8_t mods[6][16];
    en_init();
    if(argc>1&&!strcmp(argv[1],"--bridge-check")) {
        int seed,slot;uint8_t packed[18];
        for(seed=0;seed<512;seed++){
            en_acid_gate=en_acid_acc=en_acid_slide=0;
            for(slot=0;slot<16;slot++){unsigned v=(seed+37*slot)&511;en_acid_off[slot]=v&15;en_acid_oct[slot]=v>>4&3;en_acid_gate|=(v>>6&1)<<slot;en_acid_acc|=(v>>7&1)<<slot;en_acid_slide|=(v>>8&1)<<slot;}
            assert(en_acid_pack(packed)==18);q_w=q_r=0;en_acid_unpack(packed);assert(q_w==16);
            for(slot=0;slot<16;slot++)assert(q[slot]==((uint32_t)C_ACSTEP<<24|(uint32_t)slot<<16|(uint32_t)((seed+37*slot)&511)));
        }
        for(seed=0;seed<64;seed++){
            for(slot=0;slot<16;slot++){unsigned v=(seed+13*slot)&63;en_cm_rung[slot]=v&15;en_cm_lvl[slot]=v>>4&3;}
            assert(en_pluck_pack(packed));q_w=q_r=0;en_pluck_unpack(packed);assert(q_w==16);
            for(slot=0;slot<16;slot++)assert(q[slot]==((uint32_t)C_CMSTEP<<24|(uint32_t)slot<<16|(uint32_t)((seed+13*slot)&63)));
        }
        {const int invalid[]={-1,19,INT_MIN,INT_MAX};int16_t expected[P_NPARAMS];uint16_t ep;en_piece_g(0,0,expected,&ep);
         for(seed=0;seed<4;seed++){en_piece_g(invalid[seed],-1,par,&pool);assert(!memcmp(expected,par,sizeof par)&&pool==ep);assert(!strcmp(en_genre_name(invalid[seed]),en_genre_name(0)));}}
#if EN_RUST_FACTORY
        fprintf(stderr,"{\"factory_bridge\":\"pass\",\"table_accesses\":%u,\"codec_calls\":%u,\"rejections\":%u,\"copied_bytes\":%u}\n",fm1_rust_jam_stat(32),fm1_rust_jam_stat(33),fm1_rust_jam_stat(34),fm1_rust_jam_stat(35));
#else
        fprintf(stderr,"{\"frozen_c_bridge_commands\":\"pass\"}\n");
#endif
        return 0;
    }
    if(argc>1&&!strcmp(argv[1],"--lifecycle-check")) {
        uint8_t voice[128]={0}; const char *dy,*user;
        memcpy(voice+118,"Reset Test",10);
        assert(en_user_voice(0,voice)); fm_user_take(0);
        assert(en_user_count()==1);
        user=EN_CM_SOUND[12+EN_NFMFACT];
        assert(!strcmp(user,"DX Reset Test"));
        en_fm_set_dy(1); dy=EN_CM_SOUND[12];
        en_init();
        assert(EN_CM_SOUND[12]==dy);
        assert(EN_CM_SOUND[12+EN_NFMFACT]==user);
        assert(en_user_count()==1);
        assert(!memcmp(en_user_get(0),voice,128));
        fprintf(stderr,"{\"factory_reset\":\"pass\",\"dy_and_user_voice_labels_retained\":true}\n");
        return 0;
    }
    if(argc>1&&!strcmp(argv[1],"--tables")) {
        printf("{\"base\":[");for(g=0;g<15;g++){int n=0;while(XBASE[g][n]>=0)n+=2;if(g)putchar(',');ints(XBASE[g],n+1);}
        printf("],\"overrides\":[");for(g=0;g<19;g++){int n=0;while(GEN_OVR[g][n]>=0)n+=2;if(g)putchar(',');ints(GEN_OVR[g],n+1);}
        printf("],\"rows\":");bytes(&XROW[0][0][0],15*10*7);
        printf(",\"patterns\":");words(&XPAT[0][0][0],15*36);
        printf(",\"psy_down\":");words(&PSY_DOWN[0][0],36);
        printf(",\"tempo_key_mode\":");bytes(&GEN_TR[0][0][0],19*10*3);
        printf(",\"pluck\":");bytes(&GEN_PLUCK[0][0],19*5);
        printf(",\"palettes\":");bytes((const uint8_t*)PALETTE,sizeof PALETTE);
        printf(",\"pluck_rate\":");bytes(PLUCK_RATE,19);
        printf(",\"pad_shift\":");bytes(PAD_K,19);
        printf("}\n");return 0;
    }
    printf("{\"parameters\":[");
    for(i=0;i<P_NPARAMS;i++){const en_param_t *p=en_param_info(i);if(i)putchar(',');printf("[");str(p->name);printf(",%d,%d,%d]",p->lo,p->hi,p->def);}
    printf("],\"presets\":[");
    for(g=0;g<EN_NGENRE;g++)for(i=0;i<EN_NPIECES;i++) {
        if(g||i)putchar(',');en_piece_g(g,i,par,&pool);en_piece_pats_g(g,i,pat);en_piece_mods_g(g,i,mods);
        printf("{\"genre\":%d,\"piece\":%d,\"pool\":%u,\"parameters\":",g,i,pool);ints(par,P_NPARAMS);
        printf(",\"patterns\":");words(&pat[0][0],36);printf(",\"modifiers\":");bytes(&mods[0][0],96);printf("}");
    }
    printf("],\"boot_patterns\":");words(&EN_PIECE_PATS[0][0][0],36);
    printf(",\"mode_pools\":");words(EN_MODE_POOL,EN_NMODES);
    printf(",\"donors\":[");for(g=0;g<EN_NGENRE;g++)printf("%s%d",g?",":"",en_genre_donor(g));
    printf("],\"names\":{");
    NAMES(EN_NOTE_NAME);NAMES(EN_MODE_NAME);NAMES(EN_LANE_NAME);NAMES(BL_FIG_NAME);NAMES(GENRE_NAME);
    NAMES(ETIME_NAME);NAMES(LFO_RATE_NAME);NAMES(LFO_SHAPE_NAME);NAMES(PAD_DEST_NAME);NAMES(DRONE_DEST_NAME);
    NAMES(NOISE_NAME);NAMES(TRIG_NAME);NAMES(THEME_NAME);NAMES(EN_PAD_SOUND);NAMES(EN_DRONE_SOUND);
    NAMES(EN_ACID_SOUND);NAMES(EN_KIT_NAME);NAMES(EN_CHORD_NAME);NAMES(EN_MAP_NAME);NAMES(EN_CM_SOUND);
    NAMES(FM_DX_NAME);NAMES(EN_CM_STYLE);NAMES(EN_CM_RATE);NAMES(EN_PT_NAME);NAMES(EN_FX_NAME);
    names("GEN_PIECE_NAME",&GEN_PIECE_NAME[0][0],EN_NGENRE*EN_NPIECES);
    printf("},\"arcs\":[");
    for(i=0;i<10;i++){uint8_t b[6]={0},kb[3]={0};if(i)putchar(',');for(k=0;k<8;k++){en_prg_put(b,3*k,3,NAIWAN_ARC[i][k]);en_prg_put(kb,3*k,3,NAIWAN_KIND[i][k]);en_prg_put(b,24+2*k,2,NAIWAN_BARS[i]);}en_prg_put(b,40,1,1);printf("[");bytes(b,6);putchar(',');bytes(kb,3);putchar(']');}
    printf("],\"acid\":[");
    for(i=0;i<512;i++){uint8_t b[18];en_acid_gate=en_acid_acc=en_acid_slide=0;for(k=0;k<16;k++){unsigned v=(i+37*k)&511;en_acid_off[k]=v&15;en_acid_oct[k]=v>>4&3;en_acid_gate|=(v>>6&1)<<k;en_acid_acc|=(v>>7&1)<<k;en_acid_slide|=(v>>8&1)<<k;}en_acid_pack(b);if(i)putchar(',');bytes(b,18);}
    printf("],\"pluck\":[");for(i=0;i<64;i++){uint8_t b[12];for(k=0;k<16;k++){unsigned v=(i+13*k)&63;en_cm_rung[k]=v&15;en_cm_lvl[k]=v>>4&3;}en_pluck_pack(b);if(i)putchar(',');bytes(b,12);}
    printf("]}\n");return 0;
}
