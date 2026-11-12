/* SPDX-License-Identifier: GPL-3.0-only */
/* Included after the worker callback; the platform owns actual stop mechanics.
 * join:1 returned,0 permanently stopped,negative ownership still unknown. */
static void plate_init(void);
static void worker_silence(int32_t *out,int16_t *usb,uint32_t n)
{
    uint32_t i;for(i=0;i<n;i++){out[2*i]=out[2*i+1]=0;if(usb)usb[2*i]=usb[2*i+1]=0;}
}
static int worker_join_status(void)
{
    const int status=c2_join?c2_join():-1;
    if(status==1&&fm1_rust_worker_completed(worker_generation))return 1;
    if(status==0&&fm1_rust_worker_retire(worker_generation))return 0;
    if(!worker_unknown){(void)fm1_rust_worker_quarantine(worker_generation);worker_unknown=1;}
    return -1;
}
static int worker_wait_stage(uint32_t stage)
{
    uint32_t polls;
    /* A missing/lying liveness callback cannot produce an infinite C spin.
     * Backend join itself must be bounded; its negative answer keeps leases. */
    for(polls=0;polls<65536u;polls++){
        if(fm1_rust_worker_ready(worker_generation,stage))return 1;
        if(c2_alive&&!c2_alive())break;
    }
    return worker_join_status();
}
static void worker_drop_outputs(unsigned stage,uint32_t n)
{
    uint32_t j;
    /* Called only after confirmed completion/stop, never on mere timeout. */
    pad_ring_clear(rv_job.pad_w,n);
    if(stage<=FM1_WORKER_BASS)for(j=0;j<n;j++)bass_o[j]=acid_o[j]=0.0f;
    if(stage<=FM1_WORKER_TANK)for(j=0;j<n;j++)rv_wl[j]=rv_wr[j]=0.0f;
    plate_init();fm_reset();for(j=0;j<FM_R;j++)fm_fifo[j]=0.0f;
    pq_n=0; /* no unfinished job's partial MIDI list escapes retirement */
    midi_all_off(); /* discarded note-offs must not leave external notes held */
}
static int worker_poll_quarantine(void)
{
    int status;
    if(!worker_unknown)return 1;
    status=worker_join_status();
    if(status<0)return 0;
    if(status==1&&!fm1_rust_worker_reclaim(worker_generation))return 0;
    worker_drop_outputs(FM1_WORKER_BASS,(uint32_t)rv_job.n);
    en_c2_lost++;worker_unknown=0;worker_generation=0;return 1;
}
static int worker_dispatch(void)
{
    uint32_t prepared;
    worker_meters_capture();
    if(fm1_rust_worker_phase()==FM1_WORKER_RETIRED){worker_generation=0;chunk_job_body(&rv_job);return 0;}
    prepared=fm1_rust_worker_prepare(&rv_job);
    if(!prepared){
        if(fm1_rust_worker_phase()==FM1_WORKER_RETIRED){worker_generation=0;chunk_job_body(&rv_job);return 0;}
        worker_unknown=1;(void)fm1_rust_worker_quarantine(worker_generation);return -1;
    }
    worker_generation=prepared;
    if(c2_submit&&c2_join&&c2_submit(chunk_job,(void *)(uintptr_t)prepared)>0){en_c2_jobs++;return 1;}
    fm1_rust_worker_refused();chunk_job((void *)(uintptr_t)prepared);
    if(!fm1_rust_worker_completed(prepared)){worker_unknown=1;(void)fm1_rust_worker_quarantine(prepared);return -1;}
    return 0;
}
