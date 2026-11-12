/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#define main host_script_main
#include "../../host/amb_host.c"
#undef main
#include "../../firmware/src/app/jam_usb.h"
static uint8_t scratch[JP_MAX],reply[148];
static unsigned request(unsigned op,const uint8_t *payload,unsigned n,int busy)
{
    uint8_t m[540]={0x7d,0x50,0x60,0,42};unsigned i;
    m[3]=(uint8_t)op;for(i=0;i<n;i++)m[5+i]=payload[i];m[5+n]=(uint8_t)bk_sum(m+3,n+2);
    assert(ju_request(m,n+6,scratch,busy,reply)>=9);return reply[6];
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
int main(void)
{
    unsigned n,i;uint8_t copy[JP_MAX],p[2];const uint8_t *data;
    ui_splash_ms=0;boot();run_ms(100);data=en_jam_data(&n);memcpy(copy,data,n);
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
    en_play(0);run_ms(50);ju_poll();assert(!ju_busy());assert(en_jam_data(&i)[16]=='U');
    assert(request(7,(uint8_t[]){2},1,0)==JU_OK);run_ms(50);
    assert(store_len[OBJ_JAM1]==(int)n && !memcmp(store[OBJ_JAM1],copy,n));
    assert(jam_session_valid(store[OBJ_JSESSION],(unsigned)store_len[OBJ_JSESSION]));
    assert(store[OBJ_JSESSION][13]==2);
    for(i=0;i<(n+63)/64;i++){p[0]=2;p[1]=(uint8_t)i;assert(request(2,p,2,0)==JU_OK);}
    assert(!ju_busy());boot();run_ms(50);assert(en_jam_data(&i)[16]=='U');
    puts("jam USB: checksums, incomplete/duplicate chunks, timeout, busy interlock, audition, save/readback and reboot OK");
    return 0;
}
