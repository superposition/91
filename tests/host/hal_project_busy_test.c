/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual C0/Rust project/session consumers. Admission denial must precede every
 * mutation, not merely prevent a flash write after defaults have been loaded. */
#define EN_WEB 1
#define FM1_STORE_ADMISSION_TEST 1
#include <assert.h>
#include <stdint.h>
static void web_audio(const int32_t *b,uint32_t n){(void)b;(void)n;}
#include "host/amb_host.c"
#undef __attribute__
static unsigned denied,depth,enters,exits;
int plat_store_begin(void){enters++;if(denied)return PLAT_STORE_BUSY;depth++;return 0;}
void plat_store_end(void){assert(depth);depth--;exits++;}
static uint32_t busy_hash(uint32_t h,const void *p,unsigned n){const uint8_t *b=p;while(n--)h=(h^*b++)*16777619u;return h;}
static uint32_t witness(void){
    uint32_t h=2166136261u;
#define H(v) h=busy_hash(h,&(v),sizeof(v))
    H(proj);H(image);H(saved);H(pending);H(pending_glob);H(glob);H(glob_from_old);
    H(px);H(pxl);H(pending_px);H(bs_dirty);H(store);H(store_len);H(store_lock);H(store_writes);
    H(jam_boot_on);
#undef H
    h=busy_hash(h,(const void *)en_pats,sizeof en_pats);return busy_hash(h,(const void *)en_mod,sizeof en_mod);
}
int main(void){
    unsigned i;
    for(i=0;i<OBJ_NOBJ;i++)store_len[i]=-1;
    boot();en_play(0);
    proj.par[P_PAD]=37;project_piece_save();project_globals();
    assert(project_flush()==0);
    proj.par[P_PAD]=41;project_piece_save();project_globals();
    const uint32_t before=witness();const unsigned balanced=enters-exits;
    denied=1;
    project_start(123);assert(witness()==before);
    image_load(0);assert(witness()==before);
    px_load(0);assert(witness()==before);
    bs_load(0);assert(witness()==before);
    glob_load();assert(witness()==before);
    assert(px_save()==PLAT_STORE_BUSY && witness()==before);
    assert(bs_save()==PLAT_STORE_BUSY && witness()==before);
    assert(project_try_genre(7)==PLAT_STORE_BUSY && witness()==before);
    project_set_genre(9);assert(witness()==before);
    assert(project_flush()==PLAT_STORE_BUSY && witness()==before);
    assert(project_flush_globals()==PLAT_STORE_BUSY && witness()==before);
    jam_session_load();assert(witness()==before);
    assert(jam_session_save_slot(2)==PLAT_STORE_BUSY && witness()==before);
    assert(depth==0 && enters-exits==balanced+13);
    denied=0;
    assert(project_flush()==0 && !project_pending());
    assert(project_try_genre(7)==0 && proj.genre==7);
    assert(depth==0);
    puts("13 denied C/Rust project/session operations preserve all state and saved bytes; retry succeeds");
    return 0;
}
