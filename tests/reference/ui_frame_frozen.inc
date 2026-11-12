/* SPDX-License-Identifier: GPL-3.0-only */
enum { V_TEMPO, V_PAD, V_PADENV, V_PADLFO, V_CHORDS, V_PROG, V_DRONE, V_DRENV, V_DRLFO, V_DRBASS, V_BLSTEP, V_ACID, V_ACENV, V_ACLINE, V_ACANS, V_ACSTEP, V_CMEL, V_CMEL2, V_CMVU, V_CMSTEP,
       V_DRUMS, V_LINKS, V_KIT, V_DDRIFT, V_GROOVE, V_STEPS, V_FEEL, V_PHRASE, V_SPACE, V_REVERB, V_MASTER, V_MIXER, V_GLOBAL, V_SETL, V_JAM, NDIAL, V_SEQ = NDIAL, V_PERF, V_VIZ, V_JAMHELP, V_JAMARR, V_JAMKEYS, V_JAMBASS, V_JAMSWELL, NVIEWS };
static void reference_frame(void)
{
    if (ui.splash) {
        uint32_t held = plat_buttons() | plat_keys();
        if (plat_ms() - ui.boot_t < ui_splash_ms && !held) {
            splash_tick();
            return;
        }
        ui.splash = 0;                           /* the cards' time is up (or skipped): the UI, and the music */
        ui.btn = plat_buttons();                 /* a press that skipped them does nothing more */
        ui.keys = plat_keys();
        ui.btn_used = ui.btn;
        ui.shift_keys = ui.keys;
        lcd_fill(0, 0, 240, 240, K_BG);
        if (!en_playing)
            en_play(1);
    }
    if (ui.theme != (uint8_t)proj.par[P_THEME]) {    /* a new theme: everything again */
        ui.theme = (uint8_t)proj.par[P_THEME];
        th = &THEMES[ui.theme < EN_NTHEME ? ui.theme : 0];
        ui.sig[0] = ui.sig[1] = ui.sig[2] = 0;
        lcd_fill(0, 0, 240, 240, K_BG);
    }
    store_tick(0);
    {   /* drum drift changed the pattern: the knob follows, and says so */
        static uint32_t seen;
        if (en_dd_changes != seen) {
            char t[12] = "PATTERN ";
            seen = en_dd_changes;
            proj.par[P_PATTERN] = (int16_t)en_pat_next;
            t[8] = (char)('1' + en_pat_next % 10u);
            t[9] = 0;
            if (plat_ms() >= ui.msg_until)
                say("DRUM DRIFT", t);
        }
    }
    if (rm.on) {                                 /* remote: the browser's screen, the browser's lights */
        rm_tick();
        if (rm.on) {
            plat_glow(1);
            plat_leds(rm.lit_b, rm.lit_k);
            return;
        }
    }
    jam_helper_sync();
    input();
    if(!en_jam_on) scene_tick();
    dj_tick();
    af_tick();
    mix_tick();
    smp_tick();
    key_tick();
    glide_tick();
    saver_tick();
    jam_tick();
    {   /* the load: a half that was never rendered (a real dropout) or renders that keep running past their half (the margin is gone) make the engine drop the chorus and hold
         * the DX voices to four, then two, then give up the Echoplex and the vinyl; and it eases back a step after twenty seconds of neither. One step at a time (four seconds
         * after the last on a dropout, ten on the other: a step needs time to show; one overrun used to climb all three at once, the peak of the last second still reading 100 %).
         * (A single late render is not a dropout: the next one starts at once. The average CPU and the peaks were the gauge before, but they sit high in the heavy pieces.) */
        static uint32_t calm_t, step_t, last_xr, last_late, win_t, win_n;
        const uint32_t now = plat_ms(), xr = plat_xruns(), lt = plat_late();
        const int lost = xr != last_xr;
        win_n += lt - last_late;
        last_xr = xr;
        last_late = lt;
        if (!en_playing) {
            calm_t = step_t = win_t = now;
            win_n = 0;
        } else {
            if (now - win_t > 10000u) {             /* the late renders of the last ten seconds */
                win_t = now;
                win_n = 0;
            }
            if (lost || win_n >= 6u) {
                calm_t = now;
                if (en_shed_level < 3 && now - step_t > (lost ? 4000u : 10000u)) {
                    en_shed(en_shed_level + 1);
                    step_t = now;
                    win_n = 0;
                }
            } else if (now - calm_t > 20000u && en_shed_level > 0) {
                en_shed(en_shed_level - 1);
                calm_t = now;
            }
        }
    }
    {   /* the JAM page plays one voice by hand: its own line stops (the engine's hand switch, not the piece's setting), and comes back when you leave */
        static const uint8_t MAN[4] = {P_CM_MAN, P_PAD_MAN, P_AC_MAN, P_DR_MAN};
        const int want = ui.view == V_JAM ? 1 + (ui.jam_voice & 3) : 0;
        if (want != ui.jam_man) {
            if (ui.jam_man)
                en_set(MAN[ui.jam_man - 1], proj.par[MAN[ui.jam_man - 1]]);
            if (want)
                en_set(MAN[want - 1], 1);
            ui.jam_man = (uint8_t)want;
        }
    }
    autosave();
    if (kr.on) {                                 /* the secret: the whole screen, twenty frames a second */
        runner_update();
        if (ui.frame % 3u == 0u) {
            runner_draw(0);
            cv_blit(0, 0);
            runner_draw(1);
            cv_blit(0, 120);
        }
        runner_leds();
        ui.frame++;
        return;
    }
    cv_scan = ui.scan;
    if (!ui.saver_on)
        draw_header();                           /* (the screensaver has the whole screen) */
    draw_main();
    if (!ui.saver_on)
        draw_knobs();
    leds();
    ui.frame++;
}
