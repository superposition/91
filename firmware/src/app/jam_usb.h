/* SPDX-License-Identifier: GPL-3.0-only */
/* Request: F0 7D 50 60 op token payload checksum F7.
 * Response: F0 7D 50 61 op token status [raw payload as nibbles] checksum F7.
 * checksum covers op through last data byte, seven-bit two's complement.
 * 1 caps, 2 read(slot,index), 3 begin(len14,crc32 nibbles),
 * 4 chunk(index,len14,nibbles), 5 validate, 6 audition, 7 save(slot),
 * 8 status, 9 abort, 10 control(field,value14). No flash writes while playing.
 * Uses the backup staging buffer; callers must serialize legacy backups. */
#pragma once
#include "../backup_proto.h"
enum { JU_OK, JU_BAD, JU_CRC, JU_BUSY, JU_STOP, JU_STORE, JU_VERSION };
typedef struct {
    uint32_t expected, touched;
    uint16_t len, mask;
    uint8_t token, valid;
#if !EN_RUST_HANDOFF
    uint8_t owner; /* 1 receiving, 2 awaiting activation, 3 read snapshot */
#endif
} ju_state_t;
static ju_state_t ju;
#if EN_RUST_HANDOFF
extern uint32_t fm1_rust_staging_owner(void);
extern int32_t fm1_rust_staging_transition(uint32_t,uint32_t);
static unsigned ju_owner(void) {return fm1_rust_staging_owner();}
static int ju_owner_set(unsigned next)
{
    unsigned current=ju_owner();
    return current==next || fm1_rust_staging_transition(current,next);
}
#else
static unsigned ju_owner(void) {return ju.owner;}
static int ju_owner_set(unsigned next) {ju.owner=(uint8_t)next;return 1;}
#endif
static unsigned ju_reply(uint8_t *f,unsigned op,unsigned token,unsigned status,const uint8_t *raw,unsigned n)
{
    unsigned i,o=0;
    f[o++]=0xf0; f[o++]=0x7d; f[o++]=0x50; f[o++]=0x61;
    f[o++]=(uint8_t)op; f[o++]=(uint8_t)token; f[o++]=(uint8_t)status;
    for(i=0;i<n;i++) {f[o++]=raw[i]>>4;f[o++]=raw[i]&15;}
    f[o]=(uint8_t)bk_sum(f+4,o-4);o++;f[o++]=0xf7;
    return o;
}
static void ju_poll(void)
{
    if(ju_owner()==2 && !en_jam_busy()) ju_owner_set(0);
    if(ju_owner() && ju_owner()!=2 && plat_ms()-ju.touched>30000u) {ju_owner_set(0);ju.mask=0;ju.valid=0;}
}
static int ju_busy(void) {ju_poll();return ju_owner()!=0 || en_jam_busy();}
static void ju_song_service(uint8_t *buf,int legacy_busy)
{
    static int8_t seen=-1;static uint32_t since;
    int song=en_jam_load;unsigned len;const uint8_t *data;
    ju_poll();
    if(song<0){seen=-1;return;}
    if(song!=seen){seen=(int8_t)song;since=plat_ms();return;}
    if(plat_ms()-since<150u || ju_busy() || legacy_busy)return;
    if(song<3)data=en_jam_builtin(song,&len);
    else {
        int got;if(!ju_owner_set(1))return;
        got=plat_store_load(OBJ_JAM0+song-3,buf,JP_MAX);
        if(got<=0){ju_owner_set(0);en_jam_load=-1;return;}len=(unsigned)got;data=buf;
    }
    if(en_jam_upload_from(data,len,song)){if(song>=3)ju_owner_set(2);en_jam_load=-1;}
    else if(song>=3)ju_owner_set(0);
}

static unsigned ju_request(const uint8_t *m,unsigned n,uint8_t *buf,int legacy_busy,uint8_t *out)
{
    unsigned op=n>3?m[3]:0,token=n>4?m[4]:0,st=JU_OK,i,count=0;
    uint8_t raw[68]; const uint8_t *a=m+5; unsigned alen=n>=6?n-6:0;
    ju_poll();
    if(n<6 || m[0]!=0x7d || m[1]!=0x50 || m[2]!=0x60 || bk_sum(m+3,n-4)!=m[n-1])
        return ju_reply(out,op,token,JU_CRC,0,0);
    for(i=0;i<n;i++) if(m[i]>127) return ju_reply(out,op,token,JU_BAD,0,0);
    if(op!=1 && op!=8 && legacy_busy) return ju_reply(out,op,token,JU_BUSY,0,0);
    switch(op) {
    case 1:
        if(alen) {st=JU_BAD;break;}
        raw[0]=1;raw[1]=JP_MAX&255;raw[2]=JP_MAX>>8;raw[3]=4;raw[4]=26;raw[5]=30;count=6;break;
    case 2: {
        unsigned slot,index,off,len; const uint8_t *data;
        if(alen!=2 || a[0]>4 || a[1]>59) {st=JU_BAD;break;}
        slot=a[0];index=a[1];off=index*64;
        if(!index) {
            if(ju_busy()) {st=JU_BUSY;break;}
            if(!ju_owner_set(3)) {st=JU_BUSY;break;}
            if(!slot) {data=en_jam_data(&len);for(i=0;i<len;i++)buf[i]=data[i];}
            else {int got=plat_store_load(OBJ_JAM0+slot-1,buf,JP_MAX);len=got>0?(unsigned)got:0;}
            ju.len=(uint16_t)len;ju.token=(uint8_t)token;
        } else if(ju_owner()!=3 || ju.token!=token) {st=JU_BAD;break;}
        len=ju.len;
        if(off>len) {st=JU_BAD;break;}
        count=len-off>64?64:len-off;
        raw[0]=(uint8_t)len;raw[1]=(uint8_t)(len>>8);raw[2]=(uint8_t)off;raw[3]=(uint8_t)(off>>8);
        for(i=0;i<count;i++) raw[4+i]=buf[off+i];
        if(off+count==len) ju_owner_set(0);
        count+=4;break;
    }
    case 3:
        if(alen!=10 || ju_busy()) {st=alen!=10?JU_BAD:JU_BUSY;break;}
        ju.len=(uint16_t)(a[0]<<7|a[1]);
        if(ju.len<JP_MIN || ju.len>JP_MAX) {st=JU_BAD;break;}
        ju.expected=0;
        for(i=0;i<8;i++) {if(a[2+i]>15)st=JU_BAD;ju.expected=(ju.expected<<4)|a[2+i];}
        if(st)break;
        if(!ju_owner_set(1)) {st=JU_BUSY;break;}
        ju.mask=0;ju.valid=0;ju.token=(uint8_t)token;break;
    case 4: {
        unsigned ix,off,len;
        if(alen<3 || ju_owner()!=1 || ju.token!=token || ju.valid) {st=JU_BAD;break;}
        ix=a[0];len=a[1]<<7|a[2];off=ix*256;
        if(ix>=15 || !len || len>256 || alen!=3+len*2 || off+len>ju.len || len!=(off+256<=ju.len?256:ju.len-off)) {st=JU_BAD;break;}
        for(i=0;i<len*2;i++) if(a[3+i]>15) st=JU_BAD;
        if(st)break;
        if(ju.mask>>ix&1u) { /* retries must be byte-identical */
            for(i=0;i<len;i++) if(buf[off+i]!=(uint8_t)(a[3+i*2]<<4|a[4+i*2])) st=JU_BAD;
        } else {
            for(i=0;i<len;i++) buf[off+i]=(uint8_t)(a[3+i*2]<<4|a[4+i*2]);
            ju.mask|=(uint16_t)(1u<<ix);
        }
        raw[0]=(uint8_t)ix;count=1;break;
    }
    case 5:
        if(alen || ju_owner()!=1 || ju.token!=token || ju.mask!=(uint16_t)((1u<<((ju.len+255)/256))-1u)) {st=JU_BAD;break;}
        if(jp_crc(buf,ju.len)!=ju.expected) {st=JU_CRC;break;}
        if(!en_jam_valid(buf,ju.len)) {st=buf[4]!=1 && buf[4]!=2?JU_VERSION:JU_BAD;break;}
        ju.valid=1;break;
    case 6:
        if(alen || ju_owner()!=1 || !ju.valid || ju.token!=token) {st=JU_BAD;break;}
        if(!en_jam_upload(buf,ju.len)) {st=JU_BUSY;break;}
        ju_owner_set(2);
        if(!en_jam_on) en_jam_set(JAM_ON,1);
        break;
    case 7: {
        unsigned len;const uint8_t *data;uint32_t crc;int got;
        if(alen!=1 || a[0]<1 || a[0]>4) {st=JU_BAD;break;}
        if(en_playing) {st=JU_STOP;break;}
        if(ju_busy() || plat_store_locked()) {st=JU_BUSY;break;}
        if(!ju_owner_set(3)) {st=JU_BUSY;break;}
        data=en_jam_data(&len);crc=jp_crc(data,len);
        if(plat_store_save(OBJ_JAM0+a[0]-1,data,len)) {ju_owner_set(0);st=JU_STORE;break;}
        got=plat_store_load(OBJ_JAM0+a[0]-1,buf,JP_MAX);
        if(got!=(int)len || jp_crc(buf,len)!=crc || jam_session_save_slot(a[0])) {ju_owner_set(0);st=JU_STORE;break;}
        ju_owner_set(0);
        for(i=0;i<4;i++)raw[i]=(uint8_t)(crc>>(i*8));
        count=4;break;
    }
    case 8: {
        unsigned len;const uint8_t *data=en_jam_data(&len);uint32_t crc=jp_crc(data,len),x=plat_xruns();jp_settings_t s;
        if(alen) {st=JU_BAD;break;}
        en_jam_get(&s);raw[0]=en_jam_on;raw[1]=en_jam_length;raw[2]=en_jam_bar;raw[3]=en_jam_beat;
        raw[4]=s.hold;raw[5]=(uint8_t)(en_jam_busy() || en_jam_pending);raw[6]=s.slot;
        for(i=0;i<4;i++){raw[7+i]=(uint8_t)(crc>>(i*8));raw[11+i]=(uint8_t)(en_jam_revision>>(i*8));raw[15+i]=(uint8_t)(x>>(i*8));}
        raw[19]=s.key;raw[20]=s.minor;raw[21]=s.form;raw[22]=s.movement;raw[23]=s.energy;raw[24]=en_playing;
        count=25;break;
    }
    case 11:
        if(alen){st=JU_BAD;break;}
        raw[0]=3;raw[1]=8;raw[2]=3;raw[3]=6;count=4;break;
    case 13:
        if(alen!=1 || a[0]>1){st=JU_BAD;break;}
        en_usb_fixed=a[0];raw[0]=en_usb_fixed;count=1;break;
    case 12:
        if(alen){st=JU_BAD;break;}
        raw[0]=(uint8_t)en_jam_source();raw[1]=(uint8_t)(en_jam_data(&i)[4]);
        raw[2]=(uint8_t)(en_jam_load>=0);count=3;break;
    case 9:
        if(alen) {st=JU_BAD;break;}
        if(ju_owner()==2) {st=JU_BUSY;break;}
        ju_owner_set(0);ju.valid=0;ju.mask=0;break;
    case 10:
        if(alen!=3 || a[0]>JAM_BRIGHTNESS) {st=JU_BAD;break;}
        if(a[0]==JAM_SLOT+1) en_play(a[2]!=0);
        else en_jam_set(a[0],a[1]<<7|a[2]);
        break;
    default:st=JU_BAD;break;
    }
    if(st==JU_OK && op>=2 && op<=6) ju.touched=plat_ms();
    return ju_reply(out,op,token,st,raw,count);
}
