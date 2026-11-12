/* SPDX-License-Identifier: GPL-3.0-only */
/* Keep a REAL quarantined worker generation while its pthread is positively
 * stopped/joined. Flash admission must not reclaim any retained DSP span. */
#define main unused_worker_test_main
#include "worker_handoff_test.c"
#undef main
#define FM1_STORE_ADMISSION_TEST 1
#include "../../firmware/src/app/plat.h"
static uint32_t test_flash_held,irq_word=0x84190100u;
static uint32_t irq_save(void){const uint32_t old=irq_word;irq_word&=~0x100u;return old;}
static void irq_restore(uint32_t old){irq_word=old;}
#define FM1_FLASH_CORE1_STOPPED() (test_flash_held)
#define FM1_FLASH_BACKEND_IDLE() (test_flash_held)
#include "fm1_flash_admission.h"
void fm1_hal_audio_stop_fence(void) {}
uint32_t fm1_hal_audio_stop_confirm(void) {return 0;}
int main(void){
    static int32_t out[512];unsigned i;
    test_mode=8;test_main_thread=pthread_self();
    en_init();en_play(1);en_balance_force(2);en_set(P_REVERB,100);
    en_core2_attach(test_submit,test_join,test_fence,test_alive);
    assert(!pthread_create(&test_thread,0,test_worker,0));test_thread_live=1;
    for(i=0;i<200&&!worker_unknown;i++)en_render(out,256,2048);
    assert(worker_unknown && atomic_load_explicit(&test_paused,memory_order_acquire));
    assert(plat_store_begin()==PLAT_STORE_BUSY);
    atomic_store_explicit(&test_stop,1,memory_order_release);
    const uint64_t deadline=test_ns()+2000000000u;
    while(atomic_load_explicit(&test_finished,memory_order_acquire)!=atomic_load_explicit(&test_request,memory_order_relaxed))assert(test_ns()<deadline);
    atomic_store_explicit(&test_quit,1,memory_order_release);
    assert(!pthread_join(test_thread,0));test_thread_live=0;test_flash_held=1;
    const uint32_t phase=fm1_rust_worker_phase(),generation=worker_generation,submits=test_submits;
    const uint32_t before=test_protected_fingerprint();assert(phase==FM1_WORKER_QUARANTINED);
    assert(plat_store_begin()==0 && fm1_flash_window_ready());
    assert(worker_unknown && worker_generation==generation && fm1_rust_worker_phase()==phase);
    assert(before==test_protected_fingerprint() && test_submits==submits);
    plat_store_end();test_flash_held=0;
    assert(plat_store_begin()==PLAT_STORE_BUSY && irq_word==0x84190100u);
    assert(worker_unknown && fm1_rust_worker_phase()==phase && before==test_protected_fingerprint());
    puts("confirmed stopped pthread admits flash-only recovery; real worker generation/state/spans remain quarantined, unknown stop denies");
    return 0;
}
