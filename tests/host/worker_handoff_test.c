/* SPDX-License-Identifier: GPL-3.0-only */
/* Real instrument ownership tests. Faults stop cooperatively at a stage, then
 * join the pthread before reporting stopped. Never abandon a live writer. */
#include <assert.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "amb.c"
static pthread_t test_main_thread,test_thread;
static int test_mode,test_thread_live,test_retired;
static atomic_uint test_request,test_finished,test_quit,test_paused,test_resume,test_stop;
static void(*test_fn)(void *);static void *test_context;
static void(*test_old_fn)(void *);static void *test_old_context;
static unsigned test_submits,test_joins,test_unknown_answers,test_frame,test_midi,test_unknown_frames,test_serial_after_retire;
static uint32_t test_hash=2166136261u,test_mix=91;
static FILE *test_trace;
static uint64_t test_ns(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return(uint64_t)t.tv_sec*1000000000u+(uint64_t)t.tv_nsec;}
static void test_record(const void *p,size_t n){const uint8_t *s=p;size_t i;for(i=0;i<n;i++)test_hash=(test_hash^s[i])*16777619u;if(test_trace)assert(fwrite(p,1,n,test_trace)==n);}
void en_midi_out(uint32_t status,uint32_t a,uint32_t b){const uint32_t row[]={0x4d494449u,test_frame,status,a,b};assert(pthread_equal(pthread_self(),test_main_thread));test_midi++;test_record(row,sizeof row);}
static void *test_worker(void *unused){unsigned seen=0;(void)unused;while(!atomic_load_explicit(&test_quit,memory_order_acquire)){unsigned request=atomic_load_explicit(&test_request,memory_order_acquire);if(request==seen)continue;test_fn(test_context);seen=request;atomic_store_explicit(&test_finished,request,memory_order_release);}return 0;}
static unsigned test_fault_stage(void){return test_mode==4?1u:test_mode==6?4u:2u;}
int en_worker_test_stage(uint32_t stage){
    if(test_retired){test_serial_after_retire++;return 1;}
    if(test_mode<4||test_mode==11||atomic_load_explicit(&test_request,memory_order_relaxed)!=40u||stage!=test_fault_stage())return 1;
    if(test_mode==10)return 2; /* no stage flag: full completed join must recover */
    atomic_store_explicit(&test_paused,1,memory_order_release);
    while(!atomic_load_explicit(&test_resume,memory_order_acquire)&&!atomic_load_explicit(&test_stop,memory_order_acquire));
    atomic_store_explicit(&test_paused,0,memory_order_release);
    return atomic_load_explicit(&test_stop,memory_order_acquire)?0:1;
}
static int test_submit(void(*fn)(void *),void *context){
    assert(!test_retired);test_submits++;
    if(test_mode==3){test_mix=test_mix*1103515245u+12345u;if((test_mix>>16)%3u==0)return 0;}
    if(test_mode==11&&atomic_load_explicit(&test_request,memory_order_relaxed)==39u){assert(test_old_fn);test_old_fn(test_old_context);}
    test_fn=fn;test_context=context;if(!test_old_fn){test_old_fn=fn;test_old_context=context;}
    atomic_fetch_add_explicit(&test_request,1,memory_order_release);return 1;
}
static void test_fence(void){atomic_thread_fence(memory_order_seq_cst);}
static int test_alive(void){
    if(test_mode==1&&atomic_load_explicit(&test_finished,memory_order_acquire)!=atomic_load_explicit(&test_request,memory_order_relaxed)){test_fn(test_context);atomic_store_explicit(&test_finished,atomic_load_explicit(&test_request,memory_order_relaxed),memory_order_release);}
    return !atomic_load_explicit(&test_paused,memory_order_acquire);
}
static int test_join(void){
    const uint64_t deadline=test_ns()+2000000000u;test_joins++;
    for(;;){
        if(atomic_load_explicit(&test_finished,memory_order_acquire)==atomic_load_explicit(&test_request,memory_order_relaxed))return 1;
        assert(test_ns()<deadline);
        if(atomic_load_explicit(&test_paused,memory_order_acquire)){
            if((test_mode==7||test_mode==8||test_mode==9)&&test_unknown_answers<3u){test_unknown_answers++;return test_mode==9&&test_unknown_answers==1u?1:-1;}
            if(test_mode==7||test_mode==9){atomic_store_explicit(&test_resume,1,memory_order_release);continue;}
            atomic_store_explicit(&test_stop,1,memory_order_release);
            while(atomic_load_explicit(&test_finished,memory_order_acquire)!=atomic_load_explicit(&test_request,memory_order_relaxed))assert(test_ns()<deadline);
            atomic_store_explicit(&test_quit,1,memory_order_release);assert(!pthread_join(test_thread,0));test_thread_live=0;test_retired=1;return 0;
        }
    }
}
static void test_meters(void){unsigned i;test_record((const void *)&en_pad_level,sizeof(float));test_record((const void *)&en_bass_level,sizeof(float));test_record((const void *)&en_acid_level,sizeof(float));for(i=0;i<EN_NPAD;i++){test_record((const void *)&en_pad_note[i],1);test_record((const void *)&en_pad_amp[i],4);}}
#if EN_RUST_WORKER
static uint32_t test_protected_fingerprint(void){
    uint32_t saved=test_hash,result;FILE *saved_trace=test_trace;unsigned i;test_hash=2166136261u;test_trace=0;
    test_record(pl_buf,sizeof pl_buf);test_record(pad_ring,sizeof pad_ring);test_record(fm_fifo,sizeof fm_fifo);test_record(fm_blk,sizeof fm_blk);test_record(fm6_sum,sizeof fm6_sum);
    test_record(&prng,sizeof prng);test_record(&fm_age,sizeof fm_age);test_record(&fm_pos,sizeof fm_pos);test_record(par,sizeof par);test_record(&rv_job.genre,sizeof rv_job.genre);test_record(&rv_job.float_path,sizeof rv_job.float_path);
    for(i=0;i<EN_NPAD;i++){test_record(pad[i].ph,sizeof pad[i].ph);test_record(&pad[i].amp,sizeof pad[i].amp);test_record(&pad[i].t,sizeof pad[i].t);}
    for(i=0;i<FM_NV;i++){test_record(fmx[i].s.ph,sizeof fmx[i].s.ph);test_record(&fmx[i].gate_left,sizeof fmx[i].gate_left);test_record(&fmx[i].used,1);}
    test_meters();result=test_hash;test_hash=saved;test_trace=saved_trace;return result;
}
#endif
int main(int argc,char **argv){
    static const unsigned sizes[]={256,173,40,256,99,256,256,211};static int32_t out[512];static int16_t usb[514];
    unsigned i,j,frames=0,songbook=argc>3?(unsigned)atoi(argv[3]):0;uint32_t protected_hash=0;int posted=0;float peak=0;
    test_mode=argc>1?atoi(argv[1]):0;test_main_thread=pthread_self();if(argc>2){test_trace=fopen(argv[2],"wb");assert(test_trace);}
    en_init();en_play(1);en_set(P_REVERB,100);en_set(P_CM_LVL,70);en_set(P_CM_SND,15);en_set(P_CM_DENS,90);en_set(P_PL_CHO,70);en_set(P_ECHO,100);en_set(P_SIZE,100);en_set(P_RV_PRE,100);en_balance_force(argc>4?atoi(argv[4]):2);
    en_usb_out=usb+1;
    if(songbook){unsigned n;const uint8_t *p=en_jam_builtin(0,&n);assert(en_jam_upload(p,n));en_jam_set(JAM_ON,1);}
    if(test_mode){en_core2_attach(test_submit,test_join,test_fence,test_alive);if(test_mode!=1){assert(!pthread_create(&test_thread,0,test_worker,0));test_thread_live=1;}}
    for(i=0;i<1200;i++){
        unsigned n=sizes[i%8];const uint32_t header[]={0x50434d31u,i,n};test_frame=i;usb[0]=12345;usb[513]=-12345;for(j=0;j<512;j++){usb[1+j]=0x5555;out[j]=0x12345678;}en_render(out,n,2048);
        assert(usb[0]==12345&&usb[513]==-12345);for(j=2*n;j<512;j++){assert(usb[1+j]==0x5555);assert(out[j]==0x12345678);}
#if EN_RUST_WORKER
        if(worker_unknown){
            assert(atomic_load_explicit(&test_paused,memory_order_acquire));test_unknown_frames++;
            for(j=0;j<2*n;j++)assert(out[j]==0&&usb[1+j]==0);
            if(!posted){protected_hash=test_protected_fingerprint();en_set(P_TEMPO,173);en_genre(5);en_fm_float_path(1);en_test_reseed(17);assert(protected_hash==test_protected_fingerprint());posted=1;}
            else assert(protected_hash==test_protected_fingerprint());
            {int(*submit_before)(void(*)(void *),void *)=c2_submit;en_core2_attach(0,0,0,0);assert(c2_submit==submit_before);en_init();assert(protected_hash==test_protected_fingerprint());}
        }
#endif
        test_record(header,sizeof header);test_record(out,n*2*sizeof(int32_t));test_record(usb+1,n*2*sizeof(int16_t));test_meters();frames+=n;
        for(j=0;j<2*n;j++){assert(out[j]>=-8388608&&out[j]<=8388607);float v=fm_fabsf((float)out[j]*(1.0f/8388608.0f));peak=fm_maxf(peak,v);}
    }
    if(test_thread_live){atomic_store_explicit(&test_quit,1,memory_order_release);assert(!pthread_join(test_thread,0));}
    assert(peak>.001f);if(test_mode>=4&&test_mode<=6)assert(test_retired&&en_c2_lost==1);if(test_mode==8)assert(test_retired&&test_unknown_frames==3&&en_c2_lost==1);if(test_mode==7||test_mode==9)assert(!test_retired&&test_unknown_frames==3&&en_c2_lost==1);if(posted)assert(par[P_TEMPO]==173&&chord_genre==gx(5)&&fm6_float_path==1);if(test_retired)assert(test_serial_after_retire>1000);
#if EN_RUST_WORKER
    if(test_mode==11)assert(fm1_rust_jam_stat(62)>0);
    if(test_mode>=4&&test_mode<=9)assert(fm1_rust_worker_phase()==(test_retired?FM1_WORKER_RETIRED:FM1_WORKER_IDLE));
#endif
    printf("{\"mode\":%d,\"frames\":%u,\"trace_hash\":\"%08x\",\"midi_events\":%u,\"peak\":%.9g,\"submits\":%u,\"joins\":%u,\"unknown_renders\":%u,\"retired\":%d}\n",test_mode,frames,test_hash,test_midi,(double)peak,test_submits,test_joins,test_unknown_frames,test_retired);if(test_trace)fclose(test_trace);return 0;
}
