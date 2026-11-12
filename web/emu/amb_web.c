/* SPDX-License-Identifier: GPL-3.0-only */
/* AMB-1 in the browser: the host simulator (host/amb_host.c: the whole app, UI and instrument,
 * against a simulated FM-1) compiled to WebAssembly and run in an AudioWorklet (from X0X's). The worklet asks
 * for audio; producing it advances the device's clock one millisecond at a time, which runs the
 * engine every 256 samples (the I2S half buffer) and the UI every 16 ms, as on the FM-1. The page
 * sends input, and reads the screen, the lights and the saved objects back through these exports.
 *
 *   web/emu/build.sh   ->  build/emu/amb.wasm (+ the page) */
#include <stdint.h>
#ifndef EN_RUST_WEB
#define EN_RUST_WEB 0
#endif

static void web_audio(const int32_t *blk, uint32_t n);
#include "../../host/amb_host.c"

#undef __attribute__
#define WEB_EXPORT(name) __attribute__((used, visibility("default"), export_name(#name)))

/* Transitional counters: validation, planning, rejections, C fallbacks, bass events. */
WEB_EXPORT(web_rust_stat) uint32_t web_rust_stat(uint32_t index)
{
#if EN_RUST_JAM
    return fm1_rust_jam_stat(index);
#else
    (void)index; return 0;
#endif
}

/* output ring, stereo float, filled by run_ms, drained by web_render */
#define RING 4096u
#if EN_RUST_WEB
extern uint32_t fm1_rust_web_available(void);
extern int32_t fm1_rust_web_push(const int32_t *,uint32_t);
extern int32_t fm1_rust_web_drain(uint32_t);
extern const float *fm1_rust_web_out_l(void),*fm1_rust_web_out_r(void);
extern const float *fm1_rust_web_bg(uint32_t,uint32_t,uint32_t,uint32_t);
static void web_audio(const int32_t *blk,uint32_t n) { (void)fm1_rust_web_push(blk,n); }
#define out_l (fm1_rust_web_out_l())
#define out_r (fm1_rust_web_out_r())
#else
static float ring[RING * 2];
static uint32_t ring_w, ring_r;
static void web_audio(const int32_t *blk, uint32_t n)
{
    uint32_t k;
    for (k = 0; k < n; k++) {
        uint32_t i = (ring_w++ % RING) * 2u;
        ring[i] = (float)blk[2 * k] / 8388608.0f;
        ring[i + 1] = (float)blk[2 * k + 1] / 8388608.0f;
    }
}

static float out_l[1024], out_r[1024];
#endif
WEB_EXPORT(web_out_l) const float *web_out_l(void) { return out_l; }
WEB_EXPORT(web_out_r) const float *web_out_r(void) { return out_r; }

/* n frames (<= 1024) into out_l / out_r: the device runs until they exist */
WEB_EXPORT(web_render) void web_render(uint32_t n)
{
    if (n > 1024u)
        n = 1024u;
#if EN_RUST_WEB
    while (fm1_rust_web_available() < n)
        run_ms(1);
    (void)fm1_rust_web_drain(n);
#else
    uint32_t k;
    while (ring_w - ring_r < n)
        run_ms(1);
    for (k = 0; k < n; k++) {
        uint32_t i = (ring_r++ % RING) * 2u;
        out_l[k] = ring[i];
        out_r[k] = ring[i + 1];
    }
#endif
}

WEB_EXPORT(web_boot) void web_boot(void)
{
    ui_splash_ms = 3200u;                        /* the boot cards, as on the FM-1 */
    boot();
}
WEB_EXPORT(web_seed) void web_seed(uint32_t s) { host_seed = s; }   /* before web_boot */
WEB_EXPORT(web_buttons) void web_buttons(uint32_t m) { held_btn = m; }
WEB_EXPORT(web_keys) void web_keys(uint32_t m) { held_keys = m; }
WEB_EXPORT(web_enc) void web_enc(int role, int32_t n)
{
    if (role >= 0 && role < NE)
        enc_acc[role] += n;
}
WEB_EXPORT(web_master) void web_master(uint32_t v) { master = v > 4096u ? 4096u : v; }
WEB_EXPORT(web_lit_buttons) uint32_t web_lit_buttons(void) { return lit_btn; }
WEB_EXPORT(web_lit_keys) uint32_t web_lit_keys(void) { return lit_keys; }
WEB_EXPORT(web_fb) uint16_t *web_fb(void) { return fb; }
WEB_EXPORT(web_blits) uint32_t web_blits(void) { return blits; }
WEB_EXPORT(web_now) uint32_t web_now(void) { return now_ms; }
WEB_EXPORT(web_cpu) uint32_t web_cpu(void) { return cpu_pct; }

/* saved objects: the page keeps them (localStorage) and gives them back before boot */
WEB_EXPORT(web_store) uint8_t *web_store(uint32_t obj) { return obj < OBJ_NOBJ ? store[obj] : 0; }
WEB_EXPORT(web_store_len) int web_store_len(uint32_t obj) { return obj < OBJ_NOBJ ? store_len[obj] : -1; }
WEB_EXPORT(web_store_set_len) void web_store_set_len(uint32_t obj, int n)
{
    if (obj < OBJ_NOBJ)
        store_len[obj] = n < 0 ? -1 : n > (int)PLAT_STORE_MAX ? (int)PLAT_STORE_MAX : n;
}
WEB_EXPORT(web_store_writes) uint32_t web_store_writes(void) { return store_writes; }
WEB_EXPORT(web_nobj) uint32_t web_nobj(void) { return OBJ_NOBJ; }
WEB_EXPORT(web_store_max) uint32_t web_store_max(void) { return PLAT_STORE_MAX; }

#if !EN_RUST_WEB
static float bg_out[12];                         /* for the stream view's background: what the screen shows */
static void bg_rgb(int at, uint16_t c)
{
    bg_out[at] = (float)(c >> 11) / 31.0f;
    bg_out[at + 1] = (float)((c >> 5) & 63u) / 63.0f;
    bg_out[at + 2] = (float)(c & 31u) / 31.0f;
}
#endif
/* the stream view's background: the screen theme's primary, accent and ground */
WEB_EXPORT(web_bg) const float *web_bg(void)
{
#if EN_RUST_WEB
    return fm1_rust_web_bg(th->pad,th->acc,th->bg,th->acid);
#else
    bg_rgb(0, th->pad);
    bg_rgb(3, th->acc);
    bg_rgb(6, th->bg);
    bg_rgb(9, th->acid);
    return bg_out;
#endif
}
