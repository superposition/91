/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#define main host_script_main
#include "../../host/amb_host.c"
#undef main
#include "../../firmware/src/app/jam_usb.h"
static uint8_t scratch[JP_MAX],reply[148];
static uint32_t reply_hash=0x811c9dc5u;
static unsigned request(unsigned op,const uint8_t *payload,unsigned n,int busy)
{
    uint8_t m[540]={0x7d,0x50,0x60,0,42};unsigned i,got;
    m[3]=(uint8_t)op;for(i=0;i<n;i++)m[5+i]=payload[i];m[5+n]=(uint8_t)bk_sum(m+3,n+2);
    got=ju_request(m,n+6,scratch,busy,reply);assert(got>=9);
    for(i=0;i<got;i++)reply_hash=(reply_hash^reply[i])*16777619u;
    return reply[6];
}
static void begin(const uint8_t *data,unsigned len)
{
    uint32_t crc=jp_crc(data,len);uint8_t b[10];unsigned i;
    b[0]=(uint8_t)(len>>7);b[1]=len&127;for(i=0;i<8;i++)b[2+i]=(uint8_t)(crc>>(28-4*i)&15);
    assert(request(3,b,10,0)==JU_OK);
}
static void chunk(const uint8_t *data,unsigned len,unsigned ix)
{
    uint8_t b[515];unsigned off=ix*256,n=len-off>256?256:len-off,i;
    b[0]=(uint8_t)ix;b[1]=(uint8_t)(n>>7);b[2]=n&127;
    for(i=0;i<n;i++){b[3+i*2]=data[off+i]>>4;b[4+i*2]=data[off+i]&15;}
    assert(request(4,b,3+n*2,0)==JU_OK);
}
#if EN_RUST_HANDOFF
static void snapshot_matches_live(void)
{
    en_jam_snapshot_t s;jp_settings_t state;char name[32];unsigned n,i;
    assert(en_jam_snapshot(&s));en_jam_get(&state);
    assert(!memcmp(&s.settings,&state,sizeof state));
    assert(s.on==en_jam_on && s.bar==en_jam_bar && s.beat==en_jam_beat);
    assert(s.section==en_jam_section && s.pending==en_jam_pending && s.length==en_jam_length);
    assert(s.busy==en_jam_busy() && s.load==en_jam_load && s.source==en_jam_source());
    assert(s.playing==en_playing && s.revision==en_jam_revision);
    assert(s.version==en_jam_data(&n)[4]);
    assert(!strcmp(s.title,(const char *)en_jam_data(&n)+16));
    assert(!strcmp(s.form,en_jam_form(1)) && !strcmp(s.pending_title,en_jam_pending_title()));
    en_jam_name(0,name);assert(!strcmp(s.chord,name));
    en_jam_name(1,name);assert(!strcmp(s.next,name));
    for(i=0;i<10;i++)assert(s.options[i]==en_jam_option(JAM_SWELL+(int)i));
    assert(memchr(s.title,0,32) && memchr(s.form,0,32) && memchr(s.pending_title,0,32));
    assert(memchr(s.chord,0,32) && memchr(s.next,0,32));
    assert(s.keys_name[10]==0 && s.bass_name[10]==0);
    assert(s.reserved==0 && (int8_t)s.reserved_tail[0]==en_jam_option(JAM_KEYS_OCTAVE) &&
           (int8_t)s.reserved_tail[1]==en_jam_option(JAM_BASS_OCTAVE));
}
#endif
int main(void)
{
    unsigned n,i;uint8_t copy[JP_MAX],p[2];const uint8_t *data;
    ui_splash_ms=0;boot();run_ms(100);data=en_jam_data(&n);memcpy(copy,data,n);
#if EN_RUST_HANDOFF
    snapshot_matches_live();assert(fm1_rust_jam_stat(13)>0 && fm1_rust_jam_stat(14)>0);
#endif
    /* Saved performance automation must never steer Jam's key or effects. */
    ui.dj_on=1;ui.dj_piece=1;ui.dj_len=0;ui.key_set=4;
    ui.fxs[EN_FXS_AF_AMT]=3;ui.fxs[EN_FXS_AF_EVERY]=0;
    ui.sec_last=EN_SEC_BUILD;ui.key_on=0;
    run_ms(42000);
    assert(en_par_now(P_ROOT)==0 && en_sec==EN_SEC_GROOVE && !ui.af_live && !ui.key_on);
    assert(!ui.piece_q && !ui.sl_q && en_par_now(P_REVERB)==10);
    {
        int old=proj.par[P_BRIGHT];
        /* The legacy piece and active Jam patch intentionally differ. */
        assert(en_par_now(P_BRIGHT)!=old);
        ui_param_set(P_BRIGHT,old);run_ms(40);assert(en_par_now(P_BRIGHT)==old);
        ui_param_set(P_BRIGHT,old+1);run_ms(40);
        assert(en_par_now(P_BRIGHT)==old+1 && proj.par[P_BRIGHT]==old);
        ui_param_set(P_ROOT,7);run_ms(40);ui_param_set(P_ROOT,8);run_ms(40);
        assert(ui_param_get(P_ROOT)==8 && en_par_now(P_ROOT)==0);
        ui_param_set(P_MODE,MODE_AEOLIAN);run_ms(40);
        assert(ui_param_get(P_MODE)==MODE_AEOLIAN);
        run_ms(36000);
        assert(en_par_now(P_ROOT)==8 && en_par_now(P_MODE)==MODE_AEOLIAN);
        assert(en_par_now(P_BRIGHT)==old+1);
    }
    assert(request(1,0,0,0)==JU_OK);
    assert(request(7,(uint8_t[]){1},1,0)==JU_STOP);
    begin(copy,n);chunk(copy,n,0);chunk(copy,n,0);assert(request(5,0,0,0)==JU_BAD);
    assert(request(2,(uint8_t[]){0,0},2,0)==JU_BUSY);
    assert(request(9,0,0,0)==JU_OK);assert(!en_jam_busy());
    begin(copy,n);for(i=0;i<(n+255)/256;i++)chunk(copy,n,i);
    scratch[40]^=1;assert(request(5,0,0,0)==JU_CRC);assert(request(6,0,0,0)==JU_BAD);
    assert(request(9,0,0,0)==JU_OK);
    begin(copy,n);now_ms+=30001;ju_poll();assert(!ju_busy());
    assert(request(3,(uint8_t[10]){0},10,1)==JU_BUSY);
    copy[16]='U';begin(copy,n);for(i=0;i<(n+255)/256;i++)chunk(copy,n,i);
    assert(request(5,0,0,0)==JU_OK);assert(request(6,0,0,0)==JU_OK);
    assert(ju_busy());assert(en_jam_data(&i)[16]!='U');
    assert(ju_owner()==2);
    assert(request(9,0,0,0)==JU_BUSY);
    assert(request(2,(uint8_t[]){0,0},2,0)==JU_BUSY);
    now_ms+=30001;ju_poll();assert(ju_owner()==2 && en_jam_busy());
    assert(!memcmp(scratch,copy,n)); /* activation cannot reuse the staged bytes */
    en_play(0);run_ms(50);ju_poll();assert(!ju_busy());assert(en_jam_data(&i)[16]=='U');
    assert(request(7,(uint8_t[]){2},1,0)==JU_OK);run_ms(50);
    assert(store_len[OBJ_JAM1]==(int)n && !memcmp(store[OBJ_JAM1],copy,n));
    assert(jam_session_valid(store[OBJ_JSESSION],(unsigned)store_len[OBJ_JSESSION]));
    assert(store[OBJ_JSESSION][13]==2);
    for(i=0;i<(n+63)/64;i++){
        p[0]=2;p[1]=(uint8_t)i;assert(request(2,p,2,0)==JU_OK);
        if((i+1)*64<n) {
            assert(ju_owner()==3);
            assert(request(3,(uint8_t[10]){0},10,0)==JU_BUSY);
        }
    }
    assert(!ju_busy());boot();run_ms(50);assert(en_jam_data(&i)[16]=='U');
    /* Invalid and missing saved packs must release a loader's staging lease. */
    store_len[OBJ_JAM3]=1;store[OBJ_JAM3][0]=0;
    en_jam_set(JAM_SONG,6);run_ms(250);assert(ju_owner()==0 && !en_jam_busy());
    store_len[OBJ_JAM3]=-1;run_ms(200);assert(en_jam_load<0 && ju_owner()==0);
    printf("jam USB response hash: %08x\n",reply_hash);
#if EN_RUST_HANDOFF
    {
        en_jam_snapshot_t before,after;
        snapshot_matches_live();assert(en_jam_snapshot(&before));
        en_play(0);run_ms(50);copy[16]='Q';
        begin(copy,n);for(i=0;i<(n+255)/256;i++)chunk(copy,n,i);
        assert(request(5,0,0,0)==JU_OK && request(6,0,0,0)==JU_OK);
        run_ms(50);ju_poll();assert(ju_owner()==0);
        snapshot_matches_live();assert(en_jam_snapshot(&after));
        assert(before.title[0]=='U' && after.title[0]=='Q');
        assert(jam_view.title[0]=='Q'); /* actual UI uses the copied generation */
        assert(fm1_rust_jam_stat(15)>0 && fm1_rust_jam_stat(16)>0);
        assert(fm1_rust_jam_stat(17)==0);
    }
#endif
    puts("jam USB: checksums, incomplete/duplicate chunks, timeout, busy interlock, audition, save/readback and reboot OK");
    return 0;
}
