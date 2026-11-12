/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdio.h>
#include <stdint.h>
#include "amb.c"
void compiler_effects_state(FILE *file,FILE *names){size_t offset=0;
#define record(p,n) do{size_t size_=(n);if(names)fprintf(names,"%zu %zu %s\n",offset,size_,#p);if(file&&fwrite((p),1,size_,file)!=size_)__builtin_trap();offset+=size_;}while(0)
#include "effects_state.inc"
    record(&cp_ka,4);record(&cp_kr,4);record(&cp_env,4);record(&cp_g,4);record(&cp_thr,4);record(&cp_slope,4);record(&cp_make,4);record(&lk_ceil,4);record(&lk_cur,4);record(&lk_sum,4);record(&lk_rs,4);record(&rng,4);
#undef record
}
