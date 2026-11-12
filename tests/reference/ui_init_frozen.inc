/* SPDX-License-Identifier: GPL-3.0-only */
static void frozen_ui_init(void)
{
    memset(&ui, 0, sizeof ui);
    rs_state = 0;
    ui.pat_key = -1;                             /* (no pattern key held) */
    ui.gr_all = 50;
    ui.smp_gap = 1;                              /* (the SAMPLE page: a second between sounds; none playing) */
    ui.smp_cur = -1;
    {   /* the effects' settings start at the engine's defaults; the gate at its first pattern, on the synths */
        int i;
        for (i = 0; i < EN_NFXS; i++)
            ui.fxs[i] = (uint8_t)EN_FXS_DEF[i];
        ui.gx_pat = GX_PRE[0].pat;
        ui.gx_preset = 0;
        ui.gx_tgt = 0;
        ui.scene_quant = 1;                    /* a scene on the next bar */
        ui.dj_len = 1;                         /* the auto-DJ: 16 bars of groove, a half-length breakdown, a fill before it */
        ui.dj_break = 1;
        ui.dj_fillon = 1;
        ui.mood = 50;                          /* the mood macro: the middle leaves the piece as it is */
        ui.jam_oct = 2;                        /* the JAM page: octave 0 */
        ui.fm_dy = 1;                          /* the DY voices (0.14.21): the human-made set by default */
        en_fm_set_dy(1);
    }
}
