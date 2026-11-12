/* SPDX-License-Identifier: GPL-3.0-only */
/* Separate session, so all 176 legacy parameters and piece records stay stable. */
#define JS_LEN 20
static uint8_t jam_boot_on;
#if EN_RUST_JAM
extern int32_t fm1_rust_session_validate(const uint8_t *,uint32_t);
extern int32_t fm1_rust_session_encode(const jp_settings_t *,uint32_t,uint32_t,uint8_t *,uint32_t);
#endif
int jam_session_valid(const uint8_t *p,unsigned n)
{
#if EN_RUST_JAM
    return fm1_rust_session_validate(p,n);
#else
    return n==JS_LEN && p[0]=='J' && p[1]=='S' && p[2]=='M' && p[3]==1 && p[4]<=1 &&
        p[5]<12 && p[6]<2 && (p[7]==8 || p[7]==16) && p[8]<4 && p[9]<4 && p[10]<=100 &&
        p[11]<2 && p[12]<8 && p[13]<=4 && jp_u16(p+14)>=70 && jp_u16(p+14)<=190;
#endif
}
int jam_session_save_slot(int slot)
{
    jp_settings_t s; uint8_t b[JS_LEN]={ 'J','S','M',1 }; unsigned i;
    if(en_playing || en_jam_busy() || plat_store_locked()) return -1;
    en_jam_get(&s);
    if(slot>=0 && slot<=4) {s.slot=(uint8_t)slot;en_jam_set(JAM_SLOT,slot);}
#if EN_RUST_JAM
    if(fm1_rust_session_encode(&s,(uint32_t)proj.par[P_TEMPO],(uint32_t)en_jam_source(),b,JS_LEN)==JS_LEN)
        return plat_store_save(OBJ_JSESSION,b,JS_LEN);
    fm1_rust_jam_fallback();
#endif
    for(i=0;i<sizeof s;i++) b[4+i]=((const uint8_t *)&s)[i];
    b[14]=(uint8_t)proj.par[P_TEMPO]; b[15]=0;
    b[16]=(uint8_t)en_jam_source();b[17]='S';
    return plat_store_save(OBJ_JSESSION,b,JS_LEN);
}
/* Boot-time scratch only; use the existing project image buffer, before audio starts. */
static void jam_session_apply(const uint8_t *b)
{
    en_jam_set(JAM_KEY,b[5]+12*b[6]); en_jam_set(JAM_BARS,b[7]); en_jam_set(JAM_FORM,b[8]);
    en_jam_set(JAM_MOVE,b[9]); en_jam_set(JAM_ENERGY,b[10]); en_jam_set(JAM_HOLD,b[11]);
    en_jam_set(JAM_BASS,b[12]); en_jam_set(JAM_SLOT,b[13]);
    proj.par[P_TEMPO]=(int16_t)jp_u16(b+14); en_set(P_TEMPO,proj.par[P_TEMPO]);
    en_jam_set(JAM_ON,b[4]);
}
void jam_session_load(void)
{
    uint8_t b[JS_LEN]={'J','S','M',1,1,0,0,16,0,3,60,0,0,0,100,0};
    uint8_t saved[JS_LEN];
    int n=plat_store_load(OBJ_JSESSION,saved,JS_LEN);
    unsigned i;
    if(jam_session_valid(saved,(unsigned)(n>0?n:0)))
        for(i=0;i<JS_LEN;i++) b[i]=saved[i];
    if(b[13]) {
        uint8_t *p=en_jam_boot_buffer();
        n=plat_store_load(OBJ_JAM0+b[13]-1,p,JP_MAX);
        if(n<=0 || !en_jam_upload_from(p,(unsigned)n,b[13]+2)) b[13]=0;
    }
    if(!b[13] && b[17]=='S' && b[16]<3) {
        unsigned len;const uint8_t *p=en_jam_builtin(b[16],&len);en_jam_upload_from(p,len,b[16]);
    }
    jam_boot_on=b[4];
    jam_session_apply(b);
}

int jam_session_save(void) {return jam_session_save_slot(-1);}
