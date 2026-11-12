/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual complete UI consumers against denied admission and transient IO. */
#define EN_WEB 1
#define FM1_STORE_ADMISSION_TEST 1
#include <assert.h>
#include <stdint.h>
static void web_audio(const int32_t*b,uint32_t n){(void)b;(void)n;}
#include "ui_busy_host.c"
#undef __attribute__
#include "dsp/rust_ui_state.h"
static uint32_t denied,load_mask,save_mask,restore_mask,depth,successes,ends;
int plat_store_begin(void){if(denied)return PLAT_STORE_BUSY;depth++;successes++;return 0;}
void plat_store_end(void){assert(depth);depth--;ends++;}
int plat_store_load(uint32_t obj,void*dst,uint32_t max){if(load_mask&(1u<<obj))return PLAT_STORE_BUSY;return base_plat_store_load(obj,dst,max);}
int plat_store_save(uint32_t obj,const void*src,uint32_t len){if(save_mask&(1u<<obj))return PLAT_STORE_BUSY;return base_plat_store_save(obj,src,len);}
int plat_store_restore(uint32_t obj,const void*src,uint32_t len){if(restore_mask&(1u<<obj))return PLAT_STORE_BUSY;return base_plat_store_restore(obj,src,len);}
static uint32_t ui_busy_hash(uint32_t h,const void*p,uint32_t n){const uint8_t*b=p;while(n--)h=(h^*b++)*16777619u;return h;}
static uint32_t ui_hash(void){uint32_t h=2166136261u;
#define H(name) h=ui_busy_hash(h,&ui.name,sizeof ui.name);
 FM1_UI_FIELDS(H)
#undef H
 return h;
}
static uint32_t saved_hash(void){uint32_t h=ui_busy_hash(2166136261u,store,sizeof store);h=ui_busy_hash(h,store_len,sizeof store_len);h=ui_busy_hash(h,&store_lock,sizeof store_lock);return ui_busy_hash(h,&store_writes,sizeof store_writes);}
static uint32_t voices_hash(void){int i,n=en_user_count();uint32_t h=ui_busy_hash(2166136261u,&n,sizeof n);for(i=0;i<n;i++)h=ui_busy_hash(h,en_user_get(i),128);return h;}
static void drain(void){static int32_t block[512];en_render(block,256,2048);}
int main(void){unsigned i;uint32_t state,saved,voices,cache,screen;
 for(i=0;i<OBJ_NOBJ;i++)store_len[i]=-1;
 ui_splash_ms=0;boot();en_jam_set(JAM_ON,0);en_play(0);drain();
 {uint8_t voice[128]={0};assert(en_user_voice(0,voice));drain();assert(en_user_count()==1);}
 voices_save();ui.vox_dirty=0;settings_save();ui.set_dirty=ui.set_lazy=0;
 {mmap_t record={0};record.magic=MM_MAGIC;assert(base_plat_store_save(OBJ_MMAP,&record,sizeof record)==0);}
 assert(depth==0);

 /* Denied init must not clear UI, messages, remote state or pixels first. */
 ui.msg[0][0]='X';ui.mood=73;rm.on=1;rs_state=3;
 state=ui_hash();saved=saved_hash();screen=ui_busy_hash(2166136261u,fb,sizeof fb);
 denied=1;ui_init();assert(ui_hash()==state&&saved_hash()==saved&&rm.on==1&&rs_state==3);
 assert(ui_busy_hash(2166136261u,fb,sizeof fb)==screen&&ui_io_pending==UI_IO_INIT&&ui_flush_pending());
 denied=0;ui_io_retry();assert(!ui_io_pending&&!rm.on);en_play(0);drain();

 /* Busy reads never turn into defaults/missing/corrupt or discard current data. */
 state=ui_hash();saved=saved_hash();cache=ui_busy_hash(2166136261u,&set_disk,sizeof set_disk);
 load_mask=1u<<OBJ_SET;assert(settings_load()==PLAT_STORE_BUSY);
 assert(ui_hash()==state&&saved_hash()==saved&&ui_busy_hash(2166136261u,&set_disk,sizeof set_disk)==cache);
 settings_save();assert(ui.set_dirty&&saved_hash()==saved&&ui_storage_loading());
 held_btn=1u<<B_HOME;enc_acc[EN_K1]=1;state=ui_hash();ui_input_only();assert(ui_hash()==state&&enc_acc[EN_K1]==1);
 held_btn=0;enc_acc[EN_K1]=0;load_mask=0;ui_io_retry();assert(!(ui_io_pending&UI_IO_SET));
 state=ui_hash();voices=voices_hash();saved=saved_hash();load_mask=1u<<OBJ_VOICES;
 voices_load();assert(ui_hash()==state&&voices_hash()==voices&&saved_hash()==saved);
 voices_save();assert(ui.vox_dirty&&saved_hash()==saved);
 load_mask=0;ui_io_retry();drain();assert(!(ui_io_pending&UI_IO_VOICES)&&voices_hash()==voices);ui.vox_dirty=0;
 memset(&mm,0x6a,sizeof mm);mm.n=1;cache=ui_busy_hash(2166136261u,&mm,sizeof mm);load_mask=1u<<OBJ_MMAP;
 mm_load();assert(ui_busy_hash(2166136261u,&mm,sizeof mm)==cache&&(ui_io_pending&UI_IO_MAP));
 load_mask=0;ui_io_retry();assert(!ui_io_pending&&mm.n==0);

 /* Admission and backend Busy both preserve pending RAM writes and old disk cache. */
 ui.fxs[0]=7;ui.set_dirty=0;cache=ui_busy_hash(2166136261u,&set_disk,sizeof set_disk);saved=saved_hash();
 denied=1;settings_save();assert(ui.set_dirty&&saved_hash()==saved&&ui_busy_hash(2166136261u,&set_disk,sizeof set_disk)==cache);
 denied=0;ui_flush();assert(!ui.set_dirty);
 ui.fxs[0]=9;ui.set_dirty=0;save_mask=1u<<OBJ_SET;cache=ui_busy_hash(2166136261u,&set_disk,sizeof set_disk);saved=saved_hash();
 settings_save();assert(ui.set_dirty&&saved_hash()==saved&&ui_busy_hash(2166136261u,&set_disk,sizeof set_disk)==cache);
 save_mask=0;ui_flush();assert(!ui.set_dirty);
 ui.vox_dirty=0;saved=saved_hash();denied=1;voices_save();assert(ui.vox_dirty&&saved_hash()==saved);
 denied=0;ui_flush();assert(!ui.vox_dirty);
 ui.vox_dirty=0;saved=saved_hash();save_mask=1u<<OBJ_VOICES;voices_save();assert(ui.vox_dirty&&saved_hash()==saved);
 save_mask=0;ui_flush();assert(!ui.vox_dirty&&depth==0);

 /* Restore keeps the same source lease and waiting state until it commits. */
 memcpy(host_jam_scratch,store[OBJ_MMAP],sizeof mm);saved=saved_hash();restore_mask=1u<<OBJ_MMAP;host_seamless=1;
 ui_restore_start(OBJ_MMAP,host_jam_scratch,sizeof mm);
 assert(rs_state==1&&ui.store_wait&&ui.store_kind==4&&rs_buf==host_jam_scratch&&saved_hash()==saved&&!store_lock);
 ui_flush();assert(rs_state==1&&ui.store_wait&&saved_hash()==saved&&!store_lock);
 restore_mask=0;store_tick(1);assert(rs_state==2&&!ui.store_wait&&store_lock);
 assert(!memcmp(store[OBJ_MMAP],host_jam_scratch,sizeof mm));store_lock=0;

 /* No half genre/set-list action may run before admission; retry includes its piece. */
 ui.store_wait=0;ui_genre_pending=-1;state=ui_hash();saved=saved_hash();i=(unsigned)proj.genre;
 denied=1;genre_step(1);assert(proj.genre==i&&ui_hash()==state&&saved_hash()==saved&&ui_genre_pending==(int)((i+1)%EN_NGENRE));
 denied=0;ui_io_retry();assert(proj.genre==(int)((i+1)%EN_NGENRE)&&ui_genre_pending<0);
 {unsigned g=((unsigned)proj.genre+2)%EN_NGENRE;int entry=(int)(g*NPIECES+5);
  glob.sl[3]=(uint8_t)entry;ui.sl_pos=2;state=ui_hash();saved=saved_hash();i=(unsigned)proj.genre;
  denied=1;sl_go(3);assert(proj.genre==i&&ui.sl_pos==2&&ui_hash()==state&&saved_hash()==saved);
  assert((uint16_t)ui_genre_pending==(UI_SWITCH_SETLIST|3u<<8|(uint32_t)entry));
  glob.sl[3]=1;denied=0;ui_io_retry();assert(proj.genre==(int)g&&proj.piece==4&&ui.sl_pos==3&&ui_genre_pending<0);
 }
 assert(depth==0&&successes==ends);
 puts("UI denied init/reads/saves/restore/complete genre+setlist retries preserve state, bytes and balanced admission");
 return 0;
}
