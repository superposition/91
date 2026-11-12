/* SPDX-License-Identifier: GPL-3.0-only */
enum { V_TEMPO, V_PAD, V_PADENV, V_PADLFO, V_CHORDS, V_PROG, V_DRONE, V_DRENV, V_DRLFO, V_DRBASS, V_BLSTEP, V_ACID, V_ACENV, V_ACLINE, V_ACANS, V_ACSTEP, V_CMEL, V_CMEL2, V_CMVU, V_CMSTEP,
       V_DRUMS, V_LINKS, V_KIT, V_DDRIFT, V_GROOVE, V_STEPS, V_FEEL, V_PHRASE, V_SPACE, V_REVERB, V_MASTER, V_MIXER, V_GLOBAL, V_SETL, V_JAM, NDIAL, V_SEQ = NDIAL, V_PERF, V_VIZ, V_JAMHELP, V_JAMARR, V_JAMKEYS, V_JAMBASS, V_JAMSWELL, NVIEWS };
static const struct {
    uint8_t btn, first, n;
} GROUP[] = {
    {B_HOME, V_TEMPO, 1}, {B_ENV, V_PAD, 5}, {B_LFO, V_DRONE, 5}, {B_ARP, V_ACID, 9}, {B_SEQ, V_DRUMS, 8}, {B_GLO, V_GLOBAL, 2},
};
#define NGROUP (sizeof GROUP/sizeof GROUP[0])
static int group_of(int v)
{
    int g;
    for (g = 0; g < NGROUP; g++)
        if (v >= GROUP[g].first && v < GROUP[g].first + GROUP[g].n)
            return g;
    return -1;
}
static int oct_par(int v)                       /* what OCT moves on page v */
{
    if (v >= V_PAD && v <= V_CHORDS)
        return P_PAD_OCT;
    if (v >= V_DRONE && v <= V_BLSTEP)
        return P_BASSOCT;
    if ((v >= V_ACID && v <= V_ACLINE) || v == V_ACSTEP)
        return P_AC_OCT;
    if (v == V_CMEL || v == V_CMEL2 || v == V_CMVU || v == V_CMSTEP)
        return P_CM_OCT;
    return P_OCT;
}
static void button(int b)
{
    int g;
    if(ui.view>=V_JAMHELP && ui.view<=V_JAMSWELL && !ui.fx_menu && b==B_SAVE) {
        say(jam_session_save()==0?"SESSION SAVED":"STOP TO SAVE",0); return;
    }
    if(ui.view==V_JAMHELP && !ui.fx_menu && (b==B_REC || b==B_EDIT)) {
        jp_settings_t s; en_jam_get(&s); en_jam_set(b==B_REC?JAM_HOLD:JAM_IDEA,b==B_REC?!s.hold:1); return;
    }
    if (b == B_HOME && en_jam_on) { set_view(V_JAMHELP); return; }
    if (b == B_HOME) {                           /* HOME: the tempo page; again, the perform screen; and back; twice quickly, the visualizers */
        const uint32_t now = plat_ms();
        if (ui.fx_menu) {                        /* HOME leaves the FX menu too (0.14.7; latched effects play on), then does its usual */
            ui.fx_menu = 0;
            ui.fx_page = 0;
            ui.fx_open_t = 0;
        }
        if (ui.view == V_VIZ) {                  /* (HOME leaves them: the perform screen) */
            set_view(V_PERF);
            ui.home_t = 0;
            return;
        }
        if (ui.home_t && now - ui.home_t < 450u) {
            ui.home_t = 0;
            set_view(V_VIZ);
            say("VISUAL", VZ_NAME[ui.viz % VZ_N]);
            return;
        }
        ui.home_t = now ? now : 1u;
        set_view(ui.view == V_TEMPO ? V_PERF : V_TEMPO);
        return;
    }
    for (g = 0; g < NGROUP; g++)                 /* a page's button: its page, or the next in its group */
        if (GROUP[g].btn == b) {
            int v = ui.view;
            if (ui.fx_menu) {                    /* a page's button leaves the FX menu (latched effects play on) */
                ui.fx_menu = 0;
                ui.fx_page = 0;
                ui.fx_open_t = 0;
            }
            if (group_of(v) == g && GROUP[g].n > 1)
                set_view(GROUP[g].first + (v - GROUP[g].first + 1) % GROUP[g].n);       /* inside: the next page */
            else if (ui.grp_last[g] >= GROUP[g].first && ui.grp_last[g] < GROUP[g].first + GROUP[g].n)
                set_view(ui.grp_last[g]);                                               /* from elsewhere: back where you were */
            else
                set_view(GROUP[g].first);
            return;
        }
    switch (b) {
    case B_FX: {
        const uint32_t now = plat_ms();
        if (ui.fx_menu && !ui.fx_page && ui.fx_open_t && now - ui.fx_open_t < 350u) {   /* FX twice quickly: the settings of the effect you used last (the first page if none) */
            ui.fx_open_t = 0;
            ui.fx_page = ui.fx_lastpg ? ui.fx_lastpg : 1;
            say(FXP[ui.fx_page - 1].name, "SETTINGS");
            break;
        }
        ui.fx_menu = (uint8_t)!ui.fx_menu;
        ui.fx_page = 0;                          /* (from an edit page too: closed) */
        ui.fx_open_t = ui.fx_menu ? (now ? now : 1u) : 0u;
        say("FX", ui.fx_menu ? "TAP: LATCH  HOLD: PLAY" : ui.fx_lock ? "LATCHED ONES STAY ON" : "OFF");
        break;
    }
    case B_EDIT:
        en_next();
        say("NEXT CHORD", en_playing ? "ON THE BAR" : 0);
        break;
    case B_PLAY:
        if (en_playing) {
            en_play(0);
            say("STOPPED", 0);
        } else {
            en_seed(plat_ms());
            en_play(1);
            say("PLAYING", 0);
        }
        break;
    case B_REC:
        say("FREEZE", en_frozen ? "OFF" : "ON");
        en_freeze(!en_frozen);                   /* the engine takes it at its next block */
        break;
    case B_OCTDN: case B_OCTUP:
        if (ui.fx_menu && !ui.fx_page && b == B_OCTDN) {   /* in FX, OCT-: everything back (effects, knobs, mutes), undoable */
            scene_reset();
            break;
        }
        if (ui.view == V_SEQ) {                  /* the sequencer: the lane */
            ui.lane = (uint8_t)((ui.lane + (b == B_OCTUP ? 1 : EN_NLANE - 1)) % EN_NLANE);
            ui.touched = -1;
        } else {                                 /* the page's voice's octave, or the song's */
            int p = oct_par(ui.view);
            char t[12];                          /* (en_param_text writes up to 10) */
            knob_set(p, proj.par[p] + (b == B_OCTUP ? 1 : -1));
            en_param_text(p, proj.par[p], t);
            say(p == P_PAD_OCT ? "PAD OCTAVE" : p == P_BASSOCT ? "BASS OCTAVE" : p == P_AC_OCT ? "ACID OCTAVE" : p == P_CM_OCT ? "PLUCK OCTAVE" : "SONG OCTAVE", t);
        }
        break;
    case B_SAVE:                                 /* nothing is saved by it: SAVE is the breakdown (and the build) */
        if (en_sec == EN_SEC_GROOVE && en_sec_req == EN_CMD_BREAK + 1) {
            en_section(EN_CMD_DROP);             /* asked twice before the bar: never mind */
            say("BREAKDOWN", "OFF");
        } else if (en_sec == EN_SEC_GROOVE) {
            en_section(EN_CMD_BREAK);
            say("BREAKDOWN", en_playing ? "NEXT BAR" : 0);
        } else if (en_sec == EN_SEC_BREAK) {
            en_section(EN_CMD_BUILD);
            say("BUILD", "4 BARS, DROP");
        } else {
            en_section(EN_CMD_DROP);
            say("DROP", "NEXT BAR");
        }
        break;
    default: break;
    }
}
