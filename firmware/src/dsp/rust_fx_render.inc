/* SPDX-License-Identifier: GPL-3.0-only */
/* Scalar capture once per block; delay/audio payloads stay in their allocations.
 * No UI spectrum reader may enter while this foreground ring lease is live. */
static void fx_render(float *left,float *right,uint32_t n)
{
    fm1_fx_sampler_t s;fm1_fx_sampler_config_t c;
    fm1_fx_tone_t t;fm1_fx_tone_config_t q;
    fx_sampler_read(&s,&c);fx_tone_read(&t,&q);
    fm1_rust_fx_render(&s,&c,&t,&q,rb,&tp_buf[0][0],&fl_buf[0][0],&rng,left,right,n);
    fx_sampler_write(&s);fx_tone_write(&t);
}
