/* SPDX-License-Identifier: GPL-3.0-only */
/* Authored destinations, then voice-led sixth-family movement. Flags: low
 * nibble borrowed voices, bit 4 diminished neighbour. No stateful RNG in Hold. */
static inline int jp_abs(int x){return x<0?-x:x;}
#include "chord_plan.inc"
static inline void jp_plan(const uint8_t *p,const jp_settings_t *s,unsigned section,
                           const jp_phrase_t *prev,jp_phrase_t *out,uint32_t *seed)
{
    const uint8_t *form;unsigned bar,b,i;int last[4]={52,55,59,62};
    if(jp_v3(p)){jp_plan_chords(p,s,section,prev,out,seed);return;}
    if(!jp_v2(p)) {jp_plan_v1(p,s,section,prev,out,seed);return;}
    form=p+JP_FORM+s->form*32;
    out->bars=s->bars;out->section=(uint8_t)(section%form[16]);out->role=form[17+out->section];
    out->motif=(uint8_t)((p[JP2_HEAD+12]+(p[JP2_HEAD+11] && section%4u==2u?1u:0u))%p[6]);
    if(p[JP2_HEAD+11]>=2 && section%4u==3u)out->motif=(uint8_t)((jp_rand(seed)>>16)%p[6]);
    if(prev && prev->bars>=8 && prev->bars<=16)for(i=0;i<4;i++)last[i]=prev->beat[prev->bars*4-1].note[i];
    for(bar=0;bar<s->bars;bar++)for(b=0;b<4;b++) {
        unsigned part=(out->role==1 || out->role==2),half=b/2;
        unsigned offset=((s->minor*2u+(s->bars==16))*2u+part)*32u+bar*2u+half;
        unsigned node=p[JP2_CHART+offset],borrow=0,dim=0;
        const uint8_t *a;const uint8_t *scale;int inv,oct,best=100000,chosen[4]={48,52,55,57};
        jp_beat_t *e=&out->beat[bar*4+b];
        /* A2 and the closing A answer the opening dominant with a tonic. */
        if(!part && out->section && bar==s->bars-1u && b>=2)node=p[s->minor?12:9];
        a=jp_node(p,node);scale=p+JP_SCALE+a[2]*8;
        if(s->movement && (b==1 || (s->movement>=2 && b==3))) {
            /* Borrow from the diminished partner, returning to the unborrowed
             * destination on the next strong beat. Major seventh/ninth colour. */
            borrow=a[6]&((bar&1)?1u:8u);
            if(p[JP2_HEAD+11]==0)borrow=0;
            if(s->movement==3 && (bar%4u)==3u && b==3 && bar+1u<s->bars) {dim=1;borrow=0;}
        }
        e->node=(uint8_t)node;e->passing=(uint8_t)(borrow | dim<<4);
        for(inv=0;inv<4;inv++) {
            int shape[4],low,high;
            for(i=0;i<4;i++) {
                unsigned voice=((unsigned)inv+i)&3u,d=voice*2u+(dim?1u:((borrow>>voice)&1u));
                shape[i]=s->key+a[1]+scale[d]+12*(int)(((unsigned)inv+i)/4);
            }
            /* Only score octaves that actually fit the keyboard register.
             * The old planner built twelve candidates per beat, most invalid.
             * Keep its cost and tie order, reducing phrase-boundary IRQ work. */
            low=(48-shape[0]+11)/12;high=(72-shape[3])/12;
            if(low<3)low=3;
            if(high>5)high=5;
            for(oct=low;oct<=high;oct++) {
                int notes[4],cost=0;
                for(i=0;i<4;i++){notes[i]=shape[i]+12*oct;cost+=jp_abs(notes[i]-last[i])*3;}
                cost+=jp_abs(notes[3]-(int)(64+s->key%5+(part?2:0)));
                if(cost<best){best=cost;for(i=0;i<4;i++)chosen[i]=notes[i];}
            }
        }
        for(i=0;i<4;i++)last[i]=e->note[i]=(uint8_t)chosen[i];
    }
}
