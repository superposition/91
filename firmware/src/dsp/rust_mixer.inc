/* SPDX-License-Identifier: GPL-3.0-only */
/* Foreground working spans only. Never expose the whole shared pad/predelay
 * allocation as a Rust borrow while the staged worker is running. */
static void rust_mix_pointers(float **p,uint32_t at)
{
    p[0]=mixl+at;p[1]=mixr+at;p[2]=rsend+at;p[3]=esend+at;p[4]=busl+at;p[5]=busr+at;
}
static void rust_mix_pad_fm(uint32_t read,uint32_t n,const fm1_mix_merge_t *config)
{
    uint32_t at=0;
    while(at<n){
        const uint32_t q=(read+at)&(PAD_R-1);
        const uint32_t count=n-at<PAD_R-q?n-at:PAD_R-q;
        float *out[6];
        const float *input[7]={pad_ring[0]+q,pad_ring[1]+q,pad_ring[2]+q,fm_ol+at,fm_or+at,fm_rs+at,fm_es+at};
        rust_mix_pointers(out,at);
        fm1_rust_mix_pad_fm(config,out,input,count);
        at+=count;
    }
}
static void rust_mix_echoes(uint32_t n,float *duck,float rg,float eg)
{
    fm1_fx_echo_t s={tape_w,tape_ph,tape_acc,tape_lp,tape_hp,tape_p1,tape_p2,tape_c1,tape_c2,plex_w,plex_lp,plex_lp2,echo_len,head2};
    const fm1_fx_echo_config_t c={echo_len_t,head2_v,motion,mod[5].v,lfo_echo,fb_lean,echo_fbx,{hg[0],hg[1],hg[2],hg[3]},(uint32_t)shed3};
    const fm1_mix_echo_sends_t sends={rg,eg,rev_send,throw_amt,tdx_m,fxs_v[EN_FXS_TD_SEND],wash_e};
    float *out[6];uint32_t j;
    rust_mix_pointers(out,0);
    fm1_rust_mix_echo_block(&s,&c,tape,plex,&sends,out,duck,n);
    tape_w=s.tape_write;tape_ph=s.phase;tape_acc=s.accumulator;tape_lp=s.low;tape_hp=s.high;
    tape_p1=s.previous1;tape_p2=s.previous2;tape_c1=s.current1;tape_c2=s.current2;
    plex_w=s.plex_write;plex_lp=s.plex_low;plex_lp2=s.plex_low2;echo_len=s.length;head2=s.head2;
    /* Each consumed rsend slot holds an exactly representable quantized i16
     * value in f32. Only storage/cursor plumbing remains here. Commit in sample
     * order; these write slots are disjoint
     * from the worker's old reads (256 <= rv_pre <= 1322; PRE_LP = 1579). */
    for(j=0;j<n;j++){
        dl_t *d=&dl[L_PRE];d->b[d->i]=(int16_t)rsend[j];
        if(++d->i>=d->n)d->i=0;
    }
}
