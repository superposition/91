/* SPDX-License-Identifier: GPL-3.0-only */
/* Full engine, real worker publication, real admission header. The backend
 * delays its final RAM-idle acknowledgment after the Rust callback completes. */
#include <stdint.h>
#include <assert.h>
#define FM1_STORE_ADMISSION_TEST 1
static void admission_render(int32_t *,uint32_t,uint32_t);
#define TEST_RENDER admission_render
#define main effects_main
#include "effects_reference.c"
#undef main
#include "../../firmware/src/app/plat.h"
static uint32_t irq_word=0x85a50100u,ram_idle=1,calls,submits,joins,denials,serial_calls;
static uint32_t irq_save(void){const uint32_t old=irq_word;irq_word&=~0x100u;return old;}
static void irq_restore(uint32_t old){irq_word=old;}
#define FM1_FLASH_BACKEND_IDLE() (ram_idle)
#include "fm1_flash_admission.h"
void fm1_hal_audio_stop_fence(void) {}
uint32_t fm1_hal_audio_stop_confirm(void) {return 0;}
static int submit(void (*fn)(void *),void *context){
    submits++;ram_idle=0;fn(context);
    assert(fm1_rust_worker_completed(worker_generation));
    /* Early Rust completion grants no permission to remove XIP. */
    assert(plat_store_begin()==PLAT_STORE_BUSY);denials++;
    assert(irq_word==0x85a50100u);return 1;
}
static int join(void){joins++;ram_idle=1;return 1;}
static void fence(void) {}
static int alive(void){return 1;}
static void admission_render(int32_t *out,uint32_t n,uint32_t gain){
    if(!calls)en_core2_attach(submit,join,fence,alive);
    const uint32_t before=submits;const int serial=calls%17u<5u;
    if(serial){
        assert(plat_store_begin()==0 && plat_store_begin()==0);
        assert(fm1_flash_window_ready());
    }
    en_render(out,n,gain);
    if(serial){
        assert(submits==before);serial_calls++;
        assert(fm1_flash_window_ready());plat_store_end();assert(fm1_flash_window_ready());
        plat_store_end();assert(!fm1_flash_window_ready());
    }
    assert(irq_word==0x85a50100u && ram_idle);
    /* Even without a live Rust phase, final backend state must be admitted. */
    ram_idle=0;assert(plat_store_begin()==PLAT_STORE_BUSY);ram_idle=1;
    worker_unknown=1;assert(plat_store_begin()==PLAT_STORE_BUSY);worker_unknown=0;
    assert(plat_store_begin()==0);plat_store_end();calls++;
}
int main(int argc,char **argv){
    const int rc=effects_main(argc,argv);
    assert(submits && joins && serial_calls && denials==submits);
    fprintf(stderr,"flash admission: calls=%u nested_serial=%u jobs=%u joins=%u early_complete_denials=%u\n",calls,serial_calls,submits,joins,denials);
    return rc;
}
