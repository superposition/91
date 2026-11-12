/* SPDX-License-Identifier: GPL-3.0-only */
/* Both actual A/B storage consumers, with Busy injected into every read phase
 * and write boundary. Ordinary I/O errors keep their frozen legacy behavior. */
#define main unused_persistence_main
#include "../reference/persistence_reference.c"
#undef main
#include <assert.h>
#if EN_RUST_STORAGE
extern uint32_t fm1_rust_jam_stat(uint32_t);
void fm1_hal_audio_stop_fence(void) {}
uint32_t fm1_hal_audio_stop_confirm(void) {return 0;}
#endif
static void prepare(void){
    unsigned i;memset(nor,255,sizeof nor);reset_faults();
    for(i=0;i<sizeof a;i++){a[i]=(uint8_t)(i*37+5);b[i]=(uint8_t)(i*53+9);}
    assert(st_save(0,a,3840)==0);assert(st_save(0,b,3840)==0);reset_faults();
}
int main(void){
    unsigned cut,cases=0;
    for(cut=0;cut<3;cut++){
        prepare();const uint32_t disk=hash(2166136261u,nor,sizeof nor);
        memset(got,0x69,sizeof got);fail_read=(int)cut;fail_code=PLAT_STORE_BUSY;
#if EN_RUST_STORAGE
        const uint32_t missing=fm1_rust_jam_stat(68),damaged=fm1_rust_jam_stat(70);
#endif
        assert(st_load(0,got+1,3840)==PLAT_STORE_BUSY);
        for(unsigned j=0;j<sizeof got;j++)assert(got[j]==0x69);
        assert(hash(2166136261u,nor,sizeof nor)==disk && programs==0);
#if EN_RUST_STORAGE
        assert(fm1_rust_jam_stat(68)==missing && fm1_rust_jam_stat(70)==damaged);
#endif
        reset_faults();assert(st_load(0,got+1,3840)==3840 && !memcmp(got+1,b,3840));cases++;
        prepare();const uint32_t before=hash(2166136261u,nor,sizeof nor);
        fail_read=(int)cut;fail_code=PLAT_STORE_BUSY;
        assert(st_save(0,a,3840)==PLAT_STORE_BUSY && programs==0);
        assert(hash(2166136261u,nor,sizeof nor)==before);cases++;
    }
    for(cut=3;cut<5;cut++){
        prepare();fail_read=(int)cut;fail_code=PLAT_STORE_BUSY;
#if EN_RUST_STORAGE
        const uint32_t damaged=fm1_rust_jam_stat(70);
#endif
        assert(st_save(0,a,3840)==PLAT_STORE_BUSY);
#if EN_RUST_STORAGE
        assert(fm1_rust_jam_stat(70)==damaged);
#endif
        reset_faults();assert(st_load(0,got+1,3840)==3840 && !memcmp(got+1,a,3840));cases++;
    }
    for(cut=0;cut<17;cut++){
        prepare();fail_code=PLAT_STORE_BUSY;
        if(cut==16)fail_erase=1;else fail_write=(int)cut;
        assert(st_save(0,a,3840)==PLAT_STORE_BUSY);
        reset_faults();assert(st_load(0,got+1,3840)==3840 && !memcmp(got+1,b,3840));cases++;
    }
    printf("%u Busy read/erase/program/verify boundaries preserve a valid record; denied loads leave output/health unchanged\n",cases);
    return 0;
}
