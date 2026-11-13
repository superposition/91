/* SPDX-License-Identifier: GPL-3.0-only */
/* Jam Pack v1: byte-addressed, endian-independent musical data, never code. */
#pragma once
#include <stdint.h>
#define JP_MAX 3840
#define JP_VERSION 1
#define JP_NODES 16
#define JP_FORMS 4
#define JP_PATTERNS 8
#define JP_NODE 64                 /* 16 x 8: bass, sixth root, scale, label, mode, reserved */
#define JP_EDGE 192                /* 16 x 16 transition weights */
#define JP_FORM 448                /* 4 x 32: name[16], length, section roles[8], reserved */
#define JP_BASS 576                /* 8 x 36: relative notes[16], velocity[16], gate, reserved */
#define JP_DRUM 864                /* 8 x 16: six LE16 lane masks, comp LE16, reserved */
#define JP_SOUND 992               /* 32 x 4: parameter LE16, signed value LE16 */
#define JP_META 1120               /* optional UTF-8 source metadata */
#define JP_MIN JP_META
/* header: JMP1, version, node count, bass count, drum count, sound count,
 * major tonic/ii/V nodes, minor tonic/ii/V nodes, reserved, name[24],
 * two editable eight-note scales at offsets 40 and 48. */
#define JP_SCALE 40
#define JP2_HEAD 1120
#define JP2_EXTRA 1152
#define JP2_CHART 1280
#define JP2_VOICE 1536
#define JP2_META 2560
#define JP3_QUALITIES 17
/* v3 shares the Songbook storage, voices and performance profiles. */
static inline int jp_v2(const uint8_t *p) {return p[4]==2 || p[4]==3;}
static inline int jp_v3(const uint8_t *p) {return p[4]==3;}
static inline int jp_synth(const uint8_t *p) {return jp_v3(p) && p[JP2_HEAD+25]==1;}
static inline int jp_octave(const uint8_t *p,unsigned part) {return jp_v3(p)?(int8_t)p[JP2_HEAD+23+part]:0;}
static inline int jp_play_note(const uint8_t *p,unsigned part,int note) {return note+12*jp_octave(p,part);}
static inline const uint8_t *jp_node(const uint8_t *p,unsigned n) {return p+(n<16 ? JP_NODE+n*8:JP2_EXTRA+(n-16)*8);}
static inline unsigned jp_u16(const uint8_t *p) { return p[0] | (unsigned)p[1] << 8; }
static inline uint32_t jp_crc(const uint8_t *p, unsigned n)
{
#if EN_RUST_FORMATS
    extern uint32_t fm1_rust_formats_crc32(const uint8_t*,uint32_t);
    return fm1_rust_formats_crc32(p,n);
#else
    uint32_t c = ~0u; unsigned i;
    while (n--) { c ^= *p++; for (i=0;i<8;i++) c = (c >> 1) ^ (0xedb88320u & (0u-(c&1u))); }
    return ~c;
#endif
}
#if EN_RUST_JAM
extern int32_t fm1_rust_jam_validate(const uint8_t *, uint32_t);
extern uint32_t fm1_rust_jam_stat(uint32_t);
#define jp_valid jp_valid_c_reference
#endif
static inline int jp_valid(const uint8_t *p, unsigned n)
{
    unsigned i,j;
    if (!p || n<JP_MIN || n>JP_MAX || p[0]!='J' || p[1]!='M' || p[2]!='P' || p[4]<1 || p[4]>3 || (p[3]-'0')!=p[4] ||
        !p[5] || p[5]>(jp_v2(p)?32:16) || !p[6] || p[6]>8 || !p[7] || p[7]>8 || p[8]>32 || p[39]) return 0;
    if(jp_v2(p) && n<JP2_META)return 0;
    for(i=9;i<15;i++) if(p[i]>=p[5] || (!jp_v3(p) && jp_node(p,p[i])[4]!=(i>=12))) return 0;
    for(i=0;i<2;i++) for(j=0;j<8;j++)
        if(p[JP_SCALE+i*8+j]>11 || (!j && p[JP_SCALE+i*8+j]) || (j && p[JP_SCALE+i*8+j]<=p[JP_SCALE+i*8+j-1])) return 0;
    for(i=0;i<p[5];i++) {
        const uint8_t *a=jp_node(p,i); unsigned sum=0;
        if(a[0]>11 || a[1]>11 || a[2]>1 || a[3]>(jp_v2(p)?7:5) || a[4]>1 || a[5]>15 || (jp_v2(p) && a[6]>15)) return 0;
        if(jp_v3(p) && a[7]>=JP3_QUALITIES)return 0;
        if(jp_v2(p)) continue;
        for(j=0;j<16;j++) { unsigned w=p[JP_EDGE+i*16+j];
            if(w && (j>=p[5] || p[JP_NODE+j*8+4]!=a[4])) return 0;
            sum+=w;
        }
        if(!sum) return 0;
    }
    for(i=0;i<4;i++) {
        const uint8_t *f=p+JP_FORM+i*32;
        if(f[15] || !f[16] || f[16]>8) return 0;
        for(j=0;j<f[16];j++) if(f[17+j]>3) return 0;
    }
    for(i=0;i<p[6];i++) {
        const uint8_t *b=p+JP_BASS+i*36;
        if(!b[32] || b[32]>100) return 0;
        for(j=0;j<16;j++) { int v=(int8_t)b[j];
            if((v < -12 || v>12) && v!=-128 && v!=-127 && v!=-126 && v!=-125) return 0;
            if(b[16+j]>127) return 0;
        }
    }
    for(i=0;i<p[7];i++) if(p[JP_DRUM+i*16+14]>100) return 0;
    if(jp_v2(p)) {
        if(n<JP2_META || p[JP2_HEAD]!='S' || p[JP2_HEAD+1]!='N' || p[JP2_HEAD+2]!='G' || p[JP2_HEAD+3]!=p[3]) return 0;
        if(jp_v3(p) && (p[JP2_HEAD+21]>1 || p[JP2_HEAD+22]>3 || p[JP2_HEAD+25]>1))return 0;
        if(jp_v3(p) && (jp_octave(p,0)<-2 || jp_octave(p,0)>2 || jp_octave(p,1)<-2 || jp_octave(p,1)>2))return 0;
        if(!p[JP2_HEAD+4] || p[JP2_HEAD+4]>8 || p[JP2_HEAD+5]>=p[JP2_HEAD+4] || p[JP2_HEAD+6]>=p[JP2_HEAD+4] || p[JP2_HEAD+7]>=24) return 0;
        for(i=18;i<=20;i++) if(p[JP2_HEAD+i]>100)return 0;
        for(i=8;i<=10;i++) if(p[JP2_HEAD+i]>100) return 0;
        if(p[JP2_HEAD+11]>3 || p[JP2_HEAD+12]>2 || !p[JP2_HEAD+16] || p[JP2_HEAD+16]>100 || !p[JP2_HEAD+17] || p[JP2_HEAD+17]>100) return 0;
        for(i=0;i<256;i++) {unsigned node=p[JP2_CHART+i];if(node>=p[5] || (!jp_v3(p) && jp_node(p,node)[4]!=(i/128))) return 0;}
        for(i=0;i<p[JP2_HEAD+4]*128u;i++) if(p[JP2_VOICE+i]>127) return 0;
    }
    return 1;
}
#if EN_RUST_JAM
#undef jp_valid
static inline int jp_valid(const uint8_t *p, unsigned n) {return fm1_rust_jam_validate(p,n);}
#endif

typedef struct { uint8_t note[4], node, passing; } jp_beat_t;
typedef struct { uint8_t bars, role, motif, section; jp_beat_t beat[64]; } jp_phrase_t;
typedef struct { uint8_t on, key, minor, bars, form, movement, energy, hold, bass, slot; } jp_settings_t;
#if EN_RUST_JAM
_Static_assert(sizeof(jp_phrase_t)==388 && sizeof(jp_settings_t)==10, "Rust Jam bridge ABI");
#endif
static inline uint32_t jp_rand(uint32_t *r) { *r=*r*1664525u+1013904223u; return *r; }
static inline unsigned jp_choose(const uint8_t *p, unsigned node, uint32_t *seed, unsigned role)
{
    unsigned i,sum=0,r;
    for(i=0;i<p[5];i++) sum+=p[JP_EDGE+node*16+i] * (1u+(p[JP_NODE+i*8+5]>>role&1u));
    r=(jp_rand(seed)>>8)%sum;
    for(i=0;i<p[5];i++) { unsigned w=p[JP_EDGE+node*16+i]*(1u+(p[JP_NODE+i*8+5]>>role&1u));
        if(r<w) return i;
        r-=w;
    }
    return node;
}
/* A bounded phrase planner. Structural destinations last one/two bars;
 * adjacent degrees of the local eight-note scale alternate sixth/dim7.
 * The eight-note domain never leaks into the legacy seven-degree engine. */
static inline void jp_plan_v1(const uint8_t *p, const jp_settings_t *s, unsigned section,
                           const jp_phrase_t *prev, jp_phrase_t *out, uint32_t *seed)
{
    unsigned bar,b,i,base=s->minor?12:9,node=p[base],rung=0;
    const uint8_t *f=p+JP_FORM+s->form*32;
    int last[4]={48,52,55,57};
    out->bars=s->bars; out->section=(uint8_t)(section%f[16]); out->role=f[17+out->section];
    out->motif=prev ? prev->motif%p[6] : (uint8_t)((jp_rand(seed)>>16)%p[6]);
    if(prev && prev->bars>=8 && prev->bars<=16) for(i=0;i<4;i++) last[i]=prev->beat[prev->bars*4-1].note[i];
    for(bar=0;bar<s->bars;bar++) {
        if(bar==s->bars-3u) node=p[base+1];
        else if(bar==s->bars-2u) node=p[base+2];
        else if(bar==s->bars-1u || !bar) node=p[base];
        else if(s->bars==8 || !(bar&1)) node=jp_choose(p,node,seed,out->role);
        /* Preserve the phrase's opening gesture; vary its answering half. */
        for(b=0;b<4;b++) {
            jp_beat_t *e=&out->beat[bar*4+b]; const uint8_t *a=p+JP_NODE+node*8;
            const uint8_t *scale=p+JP_SCALE+a[2]*8;
            unsigned move=s->movement==3 || (s->movement==2 && !(b&1)) || (s->movement==1 && b==3);
            int best=10000,bnotes[4]={48,52,55,57},inv,oct;
            if(!b) rung=0;
            else if(move) rung=(rung+(bar<s->bars/2 || out->role!=2 ? 1:7))%8;
            e->node=(uint8_t)node; e->passing=(uint8_t)(rung&1);
            for(inv=0;inv<4;inv++) for(oct=3;oct<6;oct++) {
                int notes[4],cost=0;
                for(i=0;i<4;i++) {
                    unsigned d=(rung&1)+2u*(unsigned)inv+2*i;
                    int v=(int)(s->key+a[1])+12*oct+scale[d%8]+12*(int)(d/8);
                    notes[i]=v; cost+=v>last[i]?v-last[i]:last[i]-v;
                    if(v<48 || v>76) cost+=1000;
                }
                if((unsigned)inv != rung/2) cost+=9;
                if(cost<best) {best=cost;for(i=0;i<4;i++) bnotes[i]=notes[i];}
            }
            for(i=0;i<4;i++) last[i]=e->note[i]=(uint8_t)bnotes[i];
        }
    }
}
#if EN_RUST_JAM
#define jp_plan jp_plan_c_reference
#endif
#include "song_plan.inc"
#if EN_RUST_JAM
#undef jp_plan
extern int32_t fm1_rust_jam_plan(const uint8_t *,uint32_t,const jp_settings_t *,uint32_t,const jp_phrase_t *,jp_phrase_t *,uint32_t *);
extern void fm1_rust_jam_fallback(void);
static inline void jp_plan(const uint8_t *p,const jp_settings_t *s,unsigned section,
                          const jp_phrase_t *prev,jp_phrase_t *out,uint32_t *seed)
{
    if(fm1_rust_jam_plan(p,jp_v2(p)?JP2_META:JP_MIN,s,section,prev,out,seed))return;
    fm1_rust_jam_fallback(); /* visible in emulator diagnostics; parity checks require zero */
    jp_plan_c_reference(p,s,section,prev,out,seed);
}
#endif
#if EN_RUST_JAM
#define jp_bass_note jp_bass_note_c_reference
#endif
static inline int jp_bass_note(const uint8_t *p, const jp_settings_t *s, const jp_phrase_t *ph,
                               const jp_phrase_t *next, unsigned bar, unsigned step, unsigned *velocity, unsigned *gate)
{
    unsigned pat=(ph->motif+s->bass+(bar==ph->bars-1u))%p[6],node=ph->beat[bar*4+(jp_v2(p)?step/4:0)].node;
    const uint8_t *a=jp_node(p,node),*b=p+JP_BASS+pat*36;
    int iv=(int8_t)b[step],root=(s->key+a[0])%12,n=36+root;
    *velocity=b[16+step]; *gate=b[32];
    if(iv==-128 || !*velocity) return -1;
    if(iv==-127) {
        unsigned target=jp_v2(p) && step<8 ? ph->beat[bar*4+2].node : bar+1<ph->bars ? ph->beat[(bar+1)*4].node : next->beat[0].node;
        n=36+(s->key+jp_node(p,target)[0])%12-1;
        while(n-(36+root)>6) n-=12;
        while((36+root)-n>6) n+=12;
    } else if(iv==-126) n+=jp_v3(p)?jp_chord_bass(a[7],0):(a[3]==1 || a[3]==2 || a[3]==4 || a[3]==5 ? 3:4);
    else if(iv==-125) n+=jp_v3(p)?jp_chord_bass(a[7],1):(a[3]==2 || a[3]==5 ? 6:jp_v2(p) && a[3]==7 ? 8:7);
    else n+=iv;
    while(n<(jp_v2(p)?28:31)) n+=12;
    while(n>(jp_v2(p)?52:55)) n-=12;
    return n;
}
#if EN_RUST_JAM
#undef jp_bass_note
typedef struct { int16_t note; uint8_t velocity, gate; } jp_rust_bass_t;
_Static_assert(sizeof(jp_rust_bass_t)==4, "Rust bass bridge ABI");
extern int32_t fm1_rust_jam_bass(const uint8_t *,uint32_t,const jp_settings_t *,const jp_phrase_t *,const jp_phrase_t *,uint32_t,uint32_t,jp_rust_bass_t *);
static inline int jp_bass_note(const uint8_t *p,const jp_settings_t *s,const jp_phrase_t *ph,
                               const jp_phrase_t *next,unsigned bar,unsigned step,unsigned *velocity,unsigned *gate)
{
    jp_rust_bass_t event;
    if(fm1_rust_jam_bass(p,jp_v2(p)?JP2_META:JP_MIN,s,ph,next,bar,step,&event)) {
        *velocity=event.velocity; *gate=event.gate; return event.note;
    }
    fm1_rust_jam_fallback();
    return jp_bass_note_c_reference(p,s,ph,next,bar,step,velocity,gate);
}
#endif
