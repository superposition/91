/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile the actual physical admission header against an explicit mailbox /
 * confirmed-stop model. Zero online/ready flags never invent physical stop. */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#define EN_RUST_HAL 1
#define EN_CORE2 1
#define FM1_STORE_ADMISSION_TEST 1
#include "../../firmware/src/app/plat.h"
static struct {uint32_t request,complete,rejected,ready;} aw;
static struct {uint32_t count;} fm1_core1_fault;
static uint32_t aw_online,held,unknown,active,depth,irq_word,observations;
static uint32_t aw_load(uint32_t *p){observations++;return *p;}
static uint32_t irq_save(void){uint32_t old=irq_word;irq_word&=~0x100u;return old;}
static void irq_restore(uint32_t old){irq_word=old;}
int en_flash_begin(int stopped){assert(!(irq_word&0x100u));if((unknown&&!stopped)||active)return 0;depth++;return 1;}
void en_flash_end(void){assert(depth && !(irq_word&0x100u));depth--;}
int en_flash_window_ready(int stopped){return depth && (!unknown||stopped) && !active;}
#define FM1_FLASH_CORE1_STOPPED() (held)
#include "fm1_flash_admission.h"
int main(void){
    unsigned cases=0;
    for(unsigned mask=0;mask<256;mask++){
        aw_online=mask&1;aw.ready=!!(mask&2);fm1_core1_fault.count=!!(mask&4);
        aw.rejected=!!(mask&8);aw.request=91;aw.complete=91+!!(mask&16);
        held=!!(mask&32);unknown=!!(mask&64);active=!!(mask&128);
        irq_word=0x83490100u;depth=observations=0;
        const int backend=held || (aw_online&&aw.ready&&!fm1_core1_fault.count&&!aw.rejected&&aw.request==aw.complete);
        const int admitted=backend&&(!unknown||held)&&!active;
        assert(plat_store_begin()==(admitted?0:PLAT_STORE_BUSY));
        assert(depth==(unsigned)admitted && irq_word==0x83490100u);
        assert(observations<=4); /* bounded loads, no join/spin */
        if(admitted){assert(fm1_flash_window_ready());plat_store_end();}
        assert(depth==0 && irq_word==0x83490100u);cases++;
    }
    printf("%u admission cases: live/unknown/boot/Safe Mode/held-core states, exact IRQ restore and bounded mailbox reads\n",cases);
    return 0;
}
