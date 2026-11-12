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
static int kit_view(int v) { return v == V_DRUMS || v == V_LINKS || v == V_KIT || v == V_SEQ; }
static int voice_of(int v)                      /* the voice page v belongs to (EN_V_*), or -1 */
{
    if (v >= V_PAD && v <= V_CHORDS)
        return EN_V_PAD;
    if (v >= V_DRONE && v <= V_BLSTEP)
        return EN_V_DRONE;
    if (v >= V_ACID && v <= V_ACLINE)
        return EN_V_ACID;
    return -1;
}
