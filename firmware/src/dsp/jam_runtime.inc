/* SPDX-License-Identifier: GPL-3.0-only */
/* Included after stab_fire/bl_note. All mutations here run on the audio core. */
static void apply(int p, int v);
static void pool_set(uint32_t p);
static void jp_performance_reset(void);
uint8_t *en_jam_boot_buffer(void) { return jp_active; }
static void jp_key_apply(void)
{
    apply(P_ROOT,jp_want.key); apply(P_MODE,jp_want.minor?MODE_AEOLIAN:MODE_IONIAN);
    pool_set(EN_MODE_POOL[jp_want.minor?MODE_AEOLIAN:MODE_IONIAN]);
    naiwan_pending=-1; naiwan_rotate=0; change_pending=0;
}
static void jp_sound_apply(void)
{
    /* Packs are complete patches over a documented dry baseline. Never inherit
     * a previous piece's chorus, shimmer, pitch/filter LFOs or drum tuning. */
    static const int16_t dry[][2]={
        {P_ACID,0},{P_CM_LVL,0},{P_NOISE,0},{P_PADTONE,0},{P_DR_INST,23},
        {P_PL_DEPTH,0},{P_DL_DEPTH,0},{P_MOTION,0},{P_DETUNE,0},
        {P_ECHO,0},{P_REVERB,0},{P_SIZE,12},{P_RV_TYPE,0},{P_RV_DAMP,75},
        {P_RV_PRE,20},{P_PL_CHO,0},{P_DRIVE,0},{P_DR_RES,8},{P_DE_AMT,0},
        {P_BL_MODE,3},{P_BL_GLIDE,0},{P_PAD_COMP,3},{P_LINKS,0}
    };
    int16_t target[P_NPARAMS];unsigned p,i;
    /* Build the same complete patch in linear time. Nested scans here caused
     * a deadline spike when a live song changed at its section boundary. */
    for(p=0;p<P_NPARAMS;p++)target[p]=PARAMS[p].def;
    for(i=0;i<sizeof dry/sizeof dry[0];i++)target[dry[i][0]]=dry[i][1];
    for(i=0;i<jp_data[8];i++) {
        const uint8_t *a=jp_data+JP_SOUND+i*4;target[jp_u16(a)]=(int16_t)jp_u16(a+2);
    }
    for(p=0;p<P_NPARAMS;p++)
        if(p!=P_TEMPO && p!=P_ROOT && p!=P_MODE && !en_par_global((int)p) && par[p]!=target[p])apply((int)p,target[p]);
    jp_performance_reset();
    jp_key_apply();
    if(jp_dx()) {fm_jam_load(jp_data);jp_swell_amount=jp_data[JP2_HEAD+10];jp_variation=jp_data[JP2_HEAD+11];jp_swell_phase=1.0f;}
}
int en_jam_valid(const uint8_t *p, unsigned n)
{
    unsigned i;
    if(!jp_valid(p,n)) return 0;
    for(i=0;i<p[8];i++) {
        const uint8_t *a=p+JP_SOUND+i*4; unsigned ix=jp_u16(a); int v=(int16_t)jp_u16(a+2);
        if(ix>=P_NPARAMS || ix==P_TEMPO || ix==P_ROOT || ix==P_MODE || en_par_global((int)ix) || v<PARAMS[ix].lo || v>PARAMS[ix].hi) return 0;
    }
    return 1;
}
/* The caller owns the immutable staging bytes until busy becomes false.
 * Publication is one pointer; no partially copied data can reach playback. */
int en_jam_source(void) {return jp_song;}
const char *en_jam_pending_title(void)
{
    unsigned len;
    if(jp_upload)return (const char *)(jp_upload+16);
    if(en_jam_load>=0 && en_jam_load<3)return (const char *)(en_jam_builtin(en_jam_load,&len)+16);
    return "USER SONG QUEUED";
}

const uint8_t *en_jam_builtin(int song,unsigned *n)
{
    if(song==1){*n=sizeof JP_SONG_1;return JP_SONG_1;}
    if(song==2){*n=sizeof JP_SONG_2;return JP_SONG_2;}
    *n=sizeof JP_SONG_0;return JP_SONG_0;
}
int en_jam_upload_from(const uint8_t *p,unsigned n,int source)
{
    if(jp_upload || !en_jam_valid(p,n)) return 0;
    jp_upload_source=(uint8_t)source;
    jp_upload_n=(uint16_t)n; jp_upload_prepared=0; BARRIER(); jp_upload=p; return 1;
}
int en_jam_upload(const uint8_t *p,unsigned n) {return en_jam_upload_from(p,n,255);}
int en_jam_busy(void) { return jp_upload!=0; }
const uint8_t *en_jam_data(unsigned *n) { *n=jp_len; return jp_data; }
void en_jam_get(jp_settings_t *s) { *s=jp_want; }
void en_jam_set(int field,int value) { post(C_JAM,field,value); }
void en_jam_name(int next,char *b)
{
    static const char *const suffix[]={"6","m7","m7b5","9","m6","dim7","maj9","7alt"};
    const jp_phrase_t *ph=&jp_phrase[jp_current]; unsigned beat=en_jam_beat, i=0,j=0;
    const jp_beat_t *e; const uint8_t *data=jp_data;unsigned key=jp_now.key;
    if(next && ++beat>=ph->bars*4u) {
        ph=&jp_phrase[jp_current^((jp_now.hold && !en_jam_pending && !jp_upload)?0:1)];beat=0;
        if(jp_upload && jp_upload_prepared) data=jp_upload;
        key=jp_want.key;
    }
    e=&ph->beat[beat];
    { const uint8_t *a=jp_node(data,e->node);
      int dim=jp_v2(data)?(e->passing&16)!=0:e->passing;
      const char *root=EN_NOTE_NAME[(key+(dim ? a[1]+2:a[0]))%12];
      const char *kind=dim?"dim7":suffix[a[3]];
      if(jp_v2(data) && !dim && e->passing) {
          if(a[3]==0)kind=e->passing&8?"maj7":"6/9";
          if(a[3]==1)kind=e->passing&8?"m9":"m11";
          if(a[3]==3)kind="13";
          if(a[3]==4)kind="m6/9";
      }
      while(root[i]) {b[i]=root[i];i++;} while(kind[j]) b[i++]=kind[j++]; b[i]=0;
    }
}
const char *en_jam_form(int pending) { return (const char *)(jp_data+JP_FORM+(pending?jp_want.form:jp_now.form)*32); }
static void jp_prepare(int fresh)
{
    jp_plan_wait=0;
    if(fresh) {
        jp_current=0; jp_section=0;
        jp_plan(jp_data,&jp_now,0,0,&jp_phrase[0],&jp_seed);
    }
    jp_plan(jp_upload ? jp_upload:jp_data,&jp_want,jp_upload ? 0:jp_section+1,&jp_phrase[jp_current],&jp_phrase[jp_current^1],&jp_seed);
}
static void jp_copy_upload(void)
{
    unsigned i;
    if(jp_upload) {
        const uint8_t *src=jp_upload;
        for(i=0;i<jp_upload_n;i++) jp_active[i]=src[i];
        jp_len=jp_upload_n; jp_data=jp_active; jp_want.slot=jp_upload_source>=3 && jp_upload_source<7 ? jp_upload_source-2:0;
        if(jp_upload_source<7)jp_song=jp_upload_source;
        BARRIER(); jp_upload=0; en_jam_revision++;
    }
    jp_upload_prepared=0;
}
static void jp_activate(int reset_sound)
{
    if(jp_upload) reset_sound=1;
    jp_copy_upload();
    jp_now=jp_want; en_jam_on=jp_now.on;
    if(reset_sound) jp_sound_apply(); else jp_key_apply();
    jp_prepare(1); en_jam_pending=0;
    en_jam_bar=en_jam_beat=0; en_jam_length=jp_now.bars; jp_pos=0;
}
static int jp_option_offset(int field)
{
    switch(field){case JAM_KEYS_PROGRAM:return 5;case JAM_BASS_PROGRAM:return 6;case JAM_SWELL:return 10;
    case JAM_VARIATION:return 11;case JAM_RELEASE:return 16;case JAM_BASS_GATE:return 17;case JAM_TOUCH:return 18;
    case JAM_BRIGHTNESS:return 19;case JAM_BASS_ATTACK:return 20;case JAM_SWELL_VOICE:return 7;default:return -1;}
}
int en_jam_option(int field)
{
    int off=jp_option_offset(field);return jp_dx() && off>=0?jp_data[JP2_HEAD+off]:0;
}
#if EN_RUST_HANDOFF
extern int32_t fm1_rust_snapshot_copy(const uint8_t *,uint8_t *,uint32_t);
static void jp_snapshot_text(char *dst,const char *src,unsigned capacity)
{
    unsigned i=0;while(i+1<capacity && src[i]){dst[i]=src[i];i++;}dst[i]=0;
}
int en_jam_snapshot(en_jam_snapshot_t *out)
{
    en_jam_snapshot_t s;unsigned i;
    /* Source exclusion is provided by the emulator's single execution context,
     * not by volatile or by the destination's synchronization gate. */
    for(i=0;i<sizeof s;i++)((uint8_t *)&s)[i]=0;
    s.settings=jp_want;s.on=en_jam_on;s.bar=en_jam_bar;s.beat=en_jam_beat;
    s.section=en_jam_section;s.pending=en_jam_pending;s.length=en_jam_length;
    s.busy=(uint8_t)en_jam_busy();s.load=en_jam_load;s.source=(uint8_t)en_jam_source();
    s.playing=en_playing;s.version=jp_data[4];s.revision=en_jam_revision;
    for(i=0;i<10;i++)s.options[i]=(uint8_t)en_jam_option(JAM_SWELL+(int)i);
    jp_snapshot_text(s.title,(const char *)jp_data+16,sizeof s.title);
    jp_snapshot_text(s.form,en_jam_form(1),sizeof s.form);
    jp_snapshot_text(s.pending_title,en_jam_pending_title(),sizeof s.pending_title);
    if(en_jam_on){en_jam_name(0,s.chord);en_jam_name(1,s.next);}
    for(i=0;i<2;i++) {
        char *name=i?s.bass_name:s.keys_name;
        unsigned value=s.options[(i?JAM_BASS_PROGRAM:JAM_KEYS_PROGRAM)-JAM_SWELL],k;
        if(s.version==2 && value<jp_data[JP2_HEAD+4]) {
            for(k=0;k<10;k++)name[k]=(char)jp_data[JP2_VOICE+value*128+118+k];
            name[10]=0;
        } else {name[0]='-';name[1]=0;}
    }
    return fm1_rust_snapshot_copy((const uint8_t *)&s,(uint8_t *)out,sizeof s);
}
#endif
static void jp_editable(void)
{
    unsigned i;if(jp_data!=jp_active){for(i=0;i<jp_len;i++)jp_active[i]=jp_data[i];jp_data=jp_active;}
    jp_want.slot=0;en_jam_revision++;
}
static void jp_control(int field,int value)
{
    if(field==JAM_SONG) {
        jp_song=(uint8_t)((value%7+7)%7);
        if(jp_upload && jp_upload_source!=255) {jp_upload=0;jp_upload_prepared=0;jp_prepare(0);}
        en_jam_load=(int8_t)jp_song;return;
    }
    if(jp_dx() && field>=JAM_SWELL && field<=JAM_BRIGHTNESS) {
        int off=jp_option_offset(field),max=100;
        if(field==JAM_KEYS_PROGRAM || field==JAM_BASS_PROGRAM)max=jp_data[JP2_HEAD+4]-1;
        if(field==JAM_VARIATION)max=3;
                if(field==JAM_SWELL_VOICE)max=23;
        if(off>=0){jp_editable();jp_active[JP2_HEAD+off]=(uint8_t)(value<(field==JAM_RELEASE || field==JAM_BASS_GATE?1:0)?(field==JAM_RELEASE || field==JAM_BASS_GATE?1:0):value>max?max:value);}
        jp_swell_amount=jp_data[JP2_HEAD+10];jp_variation=jp_data[JP2_HEAD+11];
        if(field==JAM_KEYS_PROGRAM || field==JAM_TOUCH || field==JAM_BRIGHTNESS || field==JAM_RELEASE)jp_patch_dirty|=1;
        if(field==JAM_BASS_PROGRAM || field==JAM_BASS_ATTACK)jp_patch_dirty|=2;
        if(field==JAM_VARIATION){jp_seed+=7919u;jp_prepare(0);}return;
    }
    if(field==JAM_ON) {jp_want.on=(uint8_t)(value!=0);en_jam_on=jp_want.on;
        if(en_jam_on) jp_activate(1); else {jp_now.on=0;pad_retarget();}
        return;
    }
    switch(field) {
    case JAM_KEY: jp_want.key=(uint8_t)((value%12+12)%12); jp_want.minor=(uint8_t)(value>=12); break;
    case JAM_BARS: jp_want.bars=(uint8_t)(value==8?8:16); break;
    case JAM_FORM: jp_want.form=(uint8_t)((value%4+4)%4); break;
    case JAM_MOVE: jp_want.movement=(uint8_t)(value<0?0:value>3?3:value); break;
    case JAM_ENERGY: jp_want.energy=jp_now.energy=(uint8_t)(value<0?0:value>100?100:value); return;
    case JAM_HOLD: jp_want.hold=jp_now.hold=(uint8_t)(value!=0); return;
    case JAM_BASS: jp_want.bass=jp_now.bass=(uint8_t)((value%8+8)%8); return;
    case JAM_IDEA: jp_want.hold=jp_now.hold=0; jp_seed+=7919u; break;
    case JAM_SLOT: if(value>=1 && value<=4)jp_song=(uint8_t)(value+2);jp_want.slot=(uint8_t)(value>=0 && value<=4?value:0); return;
    default:return;
    }
    en_jam_pending=1;
    if(!en_playing && en_jam_on) jp_activate(0);
    else jp_prepare(0);
    if(field==JAM_IDEA && jp_dx())jp_phrase[jp_current^1].motif=(uint8_t)((jp_phrase[jp_current].motif+1u)%jp_data[6]);
}
static void jp_tick(int st)
{
    jp_phrase_t *ph; unsigned i,bar,beat,vel,gate,pat; int n;
    if(!en_jam_on) return;
#if EN_RUST_SCHEDULER
    const uint32_t boundary=fm1_rust_seq_boundary(jp_pos,jp_phrase[jp_current].bars,(uint32_t)(jp_upload!=0),jp_now.hold,en_jam_pending);
    if(boundary) {
        if(boundary==1) {
#else
    if(jp_pos>=jp_phrase[jp_current].bars*16u) {
        if(jp_upload) {
#endif
            jp_current^=1; jp_section=0; jp_copy_upload(); jp_now=jp_want;
            jp_sound_apply();
            if(jp_dx())jp_plan_wait=1024;else jp_prepare(0);
            en_jam_pending=0;
        }
#if EN_RUST_SCHEDULER
        else if(boundary==2) {
#else
        else if(!jp_now.hold || en_jam_pending) {
#endif
            jp_current^=1; jp_section++; jp_now=jp_want;
            if(en_jam_pending) jp_key_apply();
            if(jp_dx())jp_plan_wait=1024;else jp_prepare(0);
            en_jam_pending=0;
        }
        jp_pos=0;
    }
    en_jam_length=jp_now.bars;
    ph=&jp_phrase[jp_current]; bar=jp_pos/16; beat=jp_pos/4;
    en_jam_bar=(uint8_t)bar; en_jam_beat=(uint8_t)beat; en_jam_section=ph->role;
    if(!(st&3)) {
        const jp_beat_t *e=&ph->beat[beat];
        chord_nt=4;
        for(i=0;i<4;i++) { pad[i].target=(float)e->note[i]; en_chord[i]=e->note[i]; chord_pcs[i]=e->note[i]%12; }
        en_chord_root=(uint8_t)((jp_now.key+jp_node(jp_data,e->node)[0])%12); en_chord_n=4;
    }
    pat=(ph->role+(bar==ph->bars-1u))%jp_data[7];
    if(jp_u16(jp_data+JP_DRUM+pat*16+12)>>st&1u) {
        unsigned g=jp_data[JP_DRUM+pat*16+14];
        if(jp_dx())g=(ph->beat[beat].passing&16)?35:jp_data[JP2_HEAD+16];
        if(jp_dx()) {
            const jp_beat_t *e=&ph->beat[beat];
            uint32_t old_age=fm_age;
            for(i=0;i<4;i++) {
                if(pad[i].midi_note)midi(0x80,pad[i].midi_note,0);
                pad[i].midi_note=e->note[i];midi(0x90,e->note[i],(unsigned)(65+jp_now.energy/3));
                fm_jam_note(0,e->note[i],0.45f+0.004f*(float)jp_now.energy,(int32_t)(step_len*0.04f*g));
            }
            /* One short overlap from the previous voicing, rather than a
             * fifth sustained keyboard note blurring the next harmony. */
            for(i=0;i<FM_NV;i++)if(fmx[i].used && !fmx[i].part && fmx[i].age<=old_age)fmx[i].fade=0.9999f;
        } else stab_fire(0.55f+0.004f*(float)jp_now.energy);
        jp_pad_gate=(uint32_t)(step_len*0.04f*(float)(g?g:80));
    }
    if(jp_dx() && (bar%4u)==3u && st==8 && jp_swell_amount) {
        const jp_beat_t *e=&ph->beat[beat];jp_swell_phase=0.0f;
        for(i=0;i<2;i++){pad[i].note=pad[i].target=(float)e->note[i];pad[i].voices=PAD_SOUND[jp_data[JP2_HEAD+7]].osc;pad[i].hand=0;pad[i].hf=1.0f;
            if(pad[i].pnote)midi(0x84,pad[i].pnote,0);
            pad[i].pnote=e->note[i];midi(0x94,pad[i].pnote,jp_swell_amount);}
    }
    n=jp_bass_note(jp_data,&jp_now,ph,jp_now.hold?ph:&jp_phrase[jp_current^1],bar,(unsigned)st,&vel,&gate);
    if(n>=0 && !bass.play && !par[P_DR_MAN]) {
        midi(0x82,bass.midi_note,0); bass.midi_note=(uint8_t)n;
        vel=vel*(60u+jp_now.energy)/110u; if(vel>127) vel=127;
        midi(0x92,(unsigned)n,vel); bass.target=bass.note=(float)n;
        if(jp_dx())gate=jp_data[JP2_HEAD+17];
        if(jp_dx())fm_jam_note(1,n,(float)vel/127.0f,(int32_t)((float)gate*0.04f*step_len));
        bass.vd=(float)vel/127.0f*0.2f; bass.env_up=1; bl_up=1; bl_slide=0;
        bl_gate=(float)gate*0.04f*step_len; jp_midi_gate=(uint32_t)bl_gate;
    }
    jp_pos++;
}
