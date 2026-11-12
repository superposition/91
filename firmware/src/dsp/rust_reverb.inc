/* SPDX-License-Identifier: GPL-3.0-only */
/* Capture/copyback only. Never borrow the shared predelay allocation as a whole. */
static void rust_reverb_read(fm1_fx_reverb_t *s)
{
    int i;for(i=1;i<L_N;i++)s->positions[i-1]=dl[i].i;
    s->gate_envelope=rg_env;s->gate_gain=rg_g;s->gate_hold=rg_hold;s->shimmer_feedback=rv_shim_fb;
    s->shimmer_dc=sh_dc;s->shimmer_phase=sh_ph;s->bandwidth=pl_bw;s->damp_left=pl_damp_l;s->damp_right=pl_damp_r;
    s->sine=lfo_s;s->cosine=lfo_c;s->shimmer_write=sh_w;
}
static void rust_reverb_write(const fm1_fx_reverb_t *s)
{
    int i;for(i=1;i<L_N;i++)dl[i].i=s->positions[i-1];
    rg_env=s->gate_envelope;rg_g=s->gate_gain;rg_hold=s->gate_hold;rv_shim_fb=s->shimmer_feedback;
    sh_dc=s->shimmer_dc;sh_ph=s->shimmer_phase;pl_bw=s->bandwidth;pl_damp_l=s->damp_left;pl_damp_r=s->damp_right;
    lfo_s=s->sine;lfo_c=s->cosine;sh_w=s->shimmer_write;
}
