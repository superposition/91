/* SPDX-License-Identifier: GPL-3.0-only */
/* JMP3: literal chord qualities. No scale substitution or rewritten cadence.
 * Rows are four keyboard tones, then the bass third (sus fourth) and fifth.
 * Extensions omit the root in the keys; the bass supplies it. All bytes are
 * immutable flash data, with no additional audio-core state or allocation. */
static inline const uint8_t *jp_chord_tones(unsigned quality)
{
    static const uint8_t tones[JP3_QUALITIES][6]={
        {0,4,7,11,4,7}, {0,3,7,10,3,7}, {0,4,7,10,4,7},
        {0,3,6,10,3,6}, {0,3,6,9,3,6}, {0,4,7,9,4,7},
        {0,3,7,9,3,7}, {0,5,7,10,5,7}, {0,3,7,11,3,7},
        {0,4,8,10,4,8}, {4,7,10,13,4,7}, {4,7,11,14,4,7},
        {3,7,10,14,3,7}, {4,7,10,14,4,7}, {4,8,10,13,4,8},
        {0,4,7,12,4,7}, {0,3,7,12,3,7}
    };
    return tones[quality];
}
static inline unsigned jp_chord_bass(unsigned quality,unsigned fifth)
{
    return jp_chord_tones(quality)[4+fifth];
}
static inline const char *jp_chord_suffix(unsigned quality)
{
    static const char *const suffix[JP3_QUALITIES]={
        "maj7","m7","7","m7b5","dim7","6","m6","7sus4","mMaj7",
        "7+5","7b9","maj9","m9","9","7alt","","m"
    };
    return suffix[quality];
}
static inline const char *jp_chord_movement(unsigned movement)
{
    static const char *const names[]={"CHART","CLOSE","DROP2","INVERT"};
    return names[movement];
}
static inline void jp_plan_chords(const uint8_t *p,const jp_settings_t *s,unsigned section,
                                  const jp_phrase_t *prev,jp_phrase_t *out,uint32_t *seed)
{
    const uint8_t *form=p+JP_FORM+s->form*32;
    unsigned bar,b,i,part,drop=s->movement==2 || (!s->movement && p[JP2_HEAD+21]);
    int last[4]={52,55,59,62};
    out->bars=s->bars;out->section=(uint8_t)(section%form[16]);out->role=form[17+out->section];
    part=out->role==1 || out->role==2;
    out->motif=(uint8_t)((p[JP2_HEAD+12]+(p[JP2_HEAD+11] && section%4u==2u?1u:0u))%p[6]);
    if(p[JP2_HEAD+11]>=2 && section%4u==3u)out->motif=(uint8_t)((jp_rand(seed)>>16)%p[6]);
    if(prev && prev->bars>=8 && prev->bars<=16)
        for(i=0;i<4;i++)last[i]=prev->beat[prev->bars*4-1].note[i];
    for(bar=0;bar<s->bars;bar++)for(b=0;b<4;b++) {
        unsigned offset=((s->minor*2u+(s->bars==16))*2u+part)*32u+bar*2u+b/2;
        unsigned node=p[JP2_CHART+offset],inv;
        const uint8_t *a=jp_node(p,node),*tones=jp_chord_tones(a[7]);
        int root=(s->key+a[0])%12,best=100000,chosen[4]={48,52,55,59};
        jp_beat_t *e=&out->beat[bar*4+b];
        e->node=(uint8_t)node;e->passing=0;
        for(inv=0;inv<4;inv++) {
            int shape[4],oct,low,high;
            for(i=0;i<4;i++) {
                shape[i]=root+tones[(inv+i)%4]+12*(int)((inv+i)/4);
                /* A triad doubles its root; keep that doubling in a distinct
                 * octave instead of issuing two identical MIDI pitches. */
                if(i)while(shape[i]<=shape[i-1])shape[i]+=12;
            }
            if(drop) {
                int lowered=shape[2]-12;
                shape[2]=shape[1];shape[1]=shape[0];shape[0]=lowered;
                for(i=1;i<4;i++) {
                    unsigned j=i;int n=shape[i];
                    while(j && shape[j-1]>n){shape[j]=shape[j-1];j--;}
                    shape[j]=n;
                }
                if(shape[0]==shape[1] || shape[1]==shape[2] || shape[2]==shape[3])continue;
            }
            low=((drop?45:48)-shape[0]+11)/12;high=(76-shape[3])/12;
            for(oct=low;oct<=high;oct++) {
                int notes[4],cost=0;
                for(i=0;i<4;i++){notes[i]=shape[i]+12*oct;cost+=jp_abs(notes[i]-last[i])*3;}
                cost+=jp_abs(notes[3]-(int)(67+part*2));
                if(s->movement==3 && inv!=(section+bar*2+b/2)%4)cost+=128;
                if(cost<best){best=cost;for(i=0;i<4;i++)chosen[i]=notes[i];}
            }
        }
        for(i=0;i<4;i++)last[i]=e->note[i]=(uint8_t)chosen[i];
    }
}
