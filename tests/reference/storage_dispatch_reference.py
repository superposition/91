#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compile actual foreground dispatch with fake admission/USB terminal actions."""
from pathlib import Path
import hashlib,json,re,sys
repo=Path(__file__).resolve().parents[2]
out=Path(sys.argv[1]).resolve();out.mkdir(parents=True,exist_ok=True)
main=(repo/'firmware/src/app/main_fm1.c').read_text()
host=(repo/'host/amb_host.c').read_text()
def function(signature):
    start=main.index(signature)
    # Preprocessor alternatives may each open the same lexical block. Every
    # owned top-level function ends at column zero; nested blocks are indented.
    end=re.search(r'(?m)^}',main[start:])
    assert end,signature
    return main[start:start+end.end()]
# Make only the existing Jam include late, so the real protocol can use the
# fault-injected load/save binding. Other app/engine/platform code is unchanged.
needle='#include "../firmware/src/app/jam_usb.h"'
assert host.count(needle)==1
host=host.replace(needle,'static void ju_song_service(uint8_t *,int);')
host=re.sub(r'#include "([^"]+)"',lambda m:'#include "'+(repo/'host'/m[1]).resolve().as_posix()+'"',host)
(out/'host.inc').write_text(host)
start=main.index('static uint8_t bk_buf[BK_MAX]')
end=main.index('/* THE PLAYER\'S DX7 VOICES.',start)
globals=main[start:end]
parts=[globals,function('static void dx_service('),function('static void px_service('),function('static void enter_uboot(')]
ota_start=main.rindex('        if (usb.ota_req) {')
brace=main.index('{',ota_start);end=brace+1;depth=1
while depth:
    depth+=(main[end]=='{')-(main[end]=='}');end+=1
ota=main[ota_start:end]
prefix=r'''
#include <assert.h>
#include <stdint.h>
#include <string.h>
#define FM1_STORE_ADMISSION_TEST 1
#define main old_host_main
#include "host.inc"
#undef main
#include "dx_proto.h"
#include "backup_proto.h"
#ifndef RING_PUBLISH
#define RING_PUBLISH() do{}while(0)
#endif
static unsigned deny,depth,enters,ends,load_busy,save_busy,loads,saves;
int plat_store_begin(void){enters++;if(deny)return PLAT_STORE_BUSY;depth++;return 0;}
void plat_store_end(void){assert(depth);depth--;ends++;}
static int fault_load(uint32_t o,void*d,uint32_t n){loads++;if(load_busy)return PLAT_STORE_BUSY;return plat_store_load(o,d,n);}
static int fault_save(uint32_t o,const void*d,uint32_t n){saves++;if(save_busy)return PLAT_STORE_BUSY;return plat_store_save(o,d,n);}
#define plat_store_load fault_load
#define plat_store_save fault_save
static uint32_t px_q[32],px_r,px_w;
#define PX_N 32u
static uint8_t dx_buf[4112],dx_state,dx_kind;static uint32_t dx_pos;
#define fm1_ms now_ms
static unsigned sent,blocked,detaches,reboots,roms,stops,plays,panics,flushes,fake_io_pending,fake_io_loading,inits,otas;
static uint8_t frames[64][148];static uint32_t frame_len[64];
static struct {uint8_t ota_req,uboot_req;}usb;
static struct {uint8_t pending;}bootguard;
static uint8_t safe_mode;static int flash_ok=1;
static int usb_sysex_try(const uint8_t *p,uint32_t n){if(blocked)return -1;assert(sent<64&&n<=148);memcpy(frames[sent],p,n);frame_len[sent++]=n;return 0;}
static void usb_detach(void){detaches++;}
static void fm1_delay_ms(uint32_t n){(void)n;}
static void fm1_reboot(void){reboots++;}
static void fm1_enter_uboot(void){roms++;}
static void fm1_audio_stop(void){stops++;}
static void fake_play(int n){(void)n;plays++;}
static void fake_panic(void){panics++;}
static void fake_flush(void){flushes++;}
static int fake_pending(void){return fake_io_pending;}
static int fake_loading(void){return fake_io_loading;}
static void fake_init(void){inits++;}
static void ota_session(void){otas++;}
static unsigned dispatch_param_calls;static int dispatch_param_id,dispatch_param_value;
static void fake_param_set(int id,int value){dispatch_param_calls++;dispatch_param_id=id;dispatch_param_value=value;ui_param_set(id,value);}
#define en_play fake_play
#define en_panic fake_panic
#define ui_flush fake_flush
#define ui_flush_pending fake_pending
#define ui_storage_loading fake_loading
#define ui_init fake_init
#define ui_param_set fake_param_set
'''
suffix=r'''
#undef en_play
#undef en_panic
#undef ui_flush
#undef ui_flush_pending
#undef ui_storage_loading
#undef ui_init
#undef ui_param_set
#undef plat_store_load
#undef plat_store_save
static unsigned raw_request(unsigned op,const uint8_t *p,unsigned n){uint8_t m[540]={0x7d,0x50,0x60,0,42};m[3]=(uint8_t)op;memcpy(m+5,p,n);m[5+n]=(uint8_t)bk_sum(m+3,n+2);return ju_request(m,n+6,bk_buf,0,ju_ack);}
int main(void){
    uint8_t expected[148],saved[BK_MAX],p[2]={1,0};unsigned n,k,enters0,saves0;
    boot();en_play(0);run_ms(50);ju_poll();now_ms=1000;en_jam_load=-1;
    for(k=0;k<192;k++)store[OBJ_JAM0][k]=(uint8_t)(k*17u);
    store_len[OBJ_JAM0]=192;memcpy(saved,store[OBJ_JAM0],192);
    memset(bk_buf,0xcc,sizeof bk_buf);bk_sess.mask=0x0555;
    deny=1;px_q[px_w++%PX_N]=4u<<28|OBJ_JAM0;px_q[px_w++%PX_N]=7u<<28;
    px_service();assert(px_r==px_w&&bk_rd_obj==OBJ_JAM0&&bk_rd_len==BK_READ_PENDING);
    assert(bk_sess.mask==0x0555&&bk_buf[0]==0xcc&&sent==1&&frames[0][3]==0x45);
    enters0=enters;px_service();assert(enters==enters0+1&&sent==1&&loads==0);
    deny=0;blocked=1;px_service();assert(depth==0&&bk_rd_len==192&&bk_rd_i==0&&bk_sess.mask==0);
    assert(!memcmp(bk_buf,saved,192));store[OBJ_JAM0][0]^=0xff;
    blocked=0;for(k=0;k<3;k++)px_service();assert(bk_rd_obj==0xffffffffu&&sent==4);
    for(k=0;k<3;k++){n=bk_build_frame(expected,OBJ_JAM0,k,192,saved+k*64,64);assert(frame_len[k+1]==n&&!memcmp(frames[k+1],expected,n));}
    /* A queued Jam request stays in dx_buf; neither Busy admission nor a
     * transient backend read can masquerade as an empty successful snapshot. */
    bk_sess.mask=0;memset(ju_ack,0xa5,sizeof ju_ack);deny=1;
    assert(raw_request(2,p,2)==0&&ju_owner()==0&&ju_ack[0]==0xa5);
    deny=0;load_busy=1;assert(raw_request(2,p,2)==0&&depth==0&&ju_owner()==0);
    load_busy=0;n=raw_request(2,p,2);assert(n>9&&ju_ack[6]==JU_OK);
    (void)raw_request(9,p,0);assert(ju_owner()==0);
    dx_kind=6;dx_state=2;dx_buf[0]=0x7d;dx_buf[1]=0x50;dx_buf[2]=0x60;dx_buf[3]=2;dx_buf[4]=42;dx_buf[5]=1;dx_buf[6]=0;dx_buf[7]=(uint8_t)bk_sum(dx_buf+3,4);dx_pos=8;
    deny=1;dx_service();assert(dx_state==2&&ju_ack_n==0);
    deny=0;dx_service();assert(dx_state==0&&ju_ack_n>0);ju_ack_n=0;(void)raw_request(9,p,0);
    /* The internal song selection is also retained until its stored read is admitted. */
    en_jam_load=3;deny=1;ju_song_service(bk_buf,0);now_ms+=200;ju_song_service(bk_buf,0);assert(en_jam_load==3&&ju_owner()==0);
    deny=0;load_busy=1;ju_song_service(bk_buf,0);assert(en_jam_load==3&&ju_owner()==0&&depth==0);
    load_busy=0;store_len[OBJ_JAM0]=-1;ju_song_service(bk_buf,0);assert(en_jam_load==-1&&ju_owner()==0);
    /* A denied save starts no write. Every later error releases its outer lease. */
    p[0]=1;deny=1;saves0=saves;assert(raw_request(7,p,1)==0&&saves==saves0&&ju_owner()==0);
    deny=0;save_busy=1;assert(raw_request(7,p,1)==0&&depth==0&&ju_owner()==0);
    save_busy=0;n=raw_request(7,p,1);assert(n>9&&ju_ack[6]==JU_OK&&depth==0);
    /* Imports and editor changes cannot replace not-yet-read saved state. */
    fake_io_loading=1;dx_kind=4;dx_state=2;dx_service();assert(dx_state==2);
    dx_state=0;
    {int before=ui_param_get(P_TEMPO),value=before==190?70:before+1;
     px_q[px_w++%PX_N]=1u<<28|P_TEMPO<<14|(unsigned)(value+8192);
     k=px_r;n=dispatch_param_calls;px_service();assert(px_r==k&&ui_param_get(P_TEMPO)==before&&dispatch_param_calls==n);
     fake_io_loading=0;px_service();assert(px_r==px_w&&dispatch_param_calls==n+1&&dispatch_param_id==P_TEMPO&&dispatch_param_value==value);}
    /* A valid restore leases the shared snapshot while edits are still pending.
     * The occupied queue head and its following command cannot be consumed. */
    memset(bk_buf,0,sizeof(glob));memcpy(bk_buf,&glob,sizeof(glob));wr32(bk_buf,GLOB_MAGIC);
    bk_sess.obj=OBJ_GLOB;bk_sess.len=sizeof(glob);bk_sess.mask=1;
    rs_state=0;bk_restored=0;bk_wait_obj=0xffffffffu;bk_touched=now_ms;
    memcpy(saved,bk_buf,sizeof(glob));assert(ui_restore_valid(OBJ_GLOB,bk_buf,sizeof(glob)));
    px_q[px_w++%PX_N]=5u<<28|OBJ_GLOB<<14|sizeof(glob);px_q[px_w++%PX_N]=7u<<28;
    fake_io_pending=1;k=px_r;n=sent;px_service();
    assert(px_r==k&&px_w==k+2&&bk_restore_deferred&&bk_sess.mask==1&&sent==n);
    assert(!memcmp(saved,bk_buf,sizeof(glob)));px_service();assert(px_r==k&&bk_restore_deferred);
    fake_io_pending=0;px_service();assert(px_r==px_w&&!bk_restore_deferred&&rs_state==2&&bk_restored);
    /* A subsequent deliberate restore bypasses old RAM dirty bits after the
     * first successful restore locked ordinary writeback. */
    bk_sess.obj=OBJ_GLOB;bk_sess.len=sizeof(glob);bk_sess.mask=1;
    fake_io_pending=1;k=flushes;assert(bk_restore_ready(5u<<28|OBJ_GLOB<<14|sizeof(glob)));
    assert(flushes==k);
    /* Detach/reinit guards do not stop/queue panic again on each Busy retry. */
    bk_restored=0;rs_state=0;
    fake_io_pending=1;safe_mode=0;usb.uboot_req=1;enter_uboot("TEST");assert(usb.uboot_req==2&&!detaches&&!roms&&plays==1&&panics==1);
    enter_uboot("TEST");assert(plays==1&&panics==1&&!roms);fake_io_pending=0;enter_uboot("TEST");assert(roms==1&&detaches==1);
    fake_io_pending=1;usb.ota_req=1;update_service_test();assert(usb.ota_req==2&&!otas&&!inits);
    enters0=plays;update_service_test();assert(plays==enters0&&!otas&&!inits);
    fake_io_pending=0;update_service_test();assert(usb.ota_req==0&&otas==1&&inits==1);
    safe_mode=1;fake_io_pending=1;enter_uboot("SAFE");assert(roms==2&&detaches==2);
    /* After a deliberate successful restore the locked old RAM image must
     * neither be written back nor prevent a requested updater transition. */
    bk_restored=1;rs_state=2;usb.ota_req=1;k=flushes;
    update_service_test();assert(!usb.ota_req&&otas==2&&flushes==k);
    assert(depth==0);puts("Busy backup/Jam reads, retained dispatch, balanced save and pending detach guards passed");return 0;
}
'''
source=prefix+'\n'.join(parts)+'\nstatic void update_service_test(void){\n'+ota+'\n}\n'+suffix
(out/'test.c').write_text(source)
identities={name:hashlib.sha256((repo/name).read_bytes()).hexdigest()for name in['host/amb_host.c','firmware/src/app/main_fm1.c','firmware/src/app/jam_usb.h','firmware/src/app/plat.h']}
(out/'sources.json').write_text(json.dumps(identities,indent=2)+'\n')
