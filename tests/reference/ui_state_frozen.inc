/* SPDX-License-Identifier: GPL-3.0-only */
/* Exact R01 UI carrier declaration; padding is never serialized. */
static struct {
    uint8_t view, last_dial;
    uint32_t btn, keys, btn_used;
    uint32_t enc_t[NE];
    uint8_t enc_run[NE];               /* detents in the current quick run, per knob (accel) */
    uint32_t down_t[NB];               /* when each button went down */
    uint32_t turn_t;                   /* the last knob turn */
    uint8_t jam_active;
    uint32_t jam_revision;
    uint8_t help;                      /* a button's card is up: the button + 1 */
    uint8_t save_held;                 /* SAVE held long enough: the drop was asked for this hold */
    uint8_t seq_held;                  /* SEQ held long enough: the sequencer opened this hold */
    uint8_t fx_menu;                   /* the FX menu is on: the keys are effects, the knobs the FX knobs */
    int8_t fxk[4];                     /* the FX knobs' values (filter -50..50, crush, throw, wash): not stored */
    uint8_t grp_last[8];               /* the page last on in each button's group (a button from elsewhere returns there) */
    uint8_t fx_page;                   /* with the FX menu on: 0 the keys, 1.. an effect's edit page (FXP) */
    uint8_t fxs[EN_NFXS];              /* the effects' settings (en_fxs), mirrored; not stored */
    uint8_t gx_preset, gx_tgt, pm_tgt; /* the gate: its pattern's name (NGXP - 1: the keys' own); its targets; the pump's */
    uint16_t gx_pat;                   /* the gate's pattern: bit s, step s open */
    uint8_t trk_m, trk_s;              /* the tracks muted and soloed (GLO held, the keys): a bit a track, not stored */
    uint64_t scene_lock[4];            /* the four scenes: the effects locked on, the mutes, the solos, the FX knobs; */
    uint8_t scene_m[4], scene_s[4];    /* kept until power-off (not stored) */
    int8_t scene_k[4][4];
    uint8_t scene_set, scene_pend;     /* the slots stored; the keys down that may be a tap (recall) or a hold (store) */
    uint32_t scene_t[4];
    uint64_t undo_lock;                /* UNDO: what the last big change (a scene, reset, clear, the drums' group) left behind */
    uint32_t tap_t[4];                 /* tap tempo: the last taps */
    uint8_t undo_m, undo_s, undo_ok, tap_n;
    int8_t undo_k[4];
    uint8_t dj_on, dj_len, dj_piece;   /* the auto-DJ: on; its groove length (8, 16, 32, 64 bars); a new piece each round */
    uint8_t af_fx, af_live, af_user, af_sec, af_said, af_last[2];   /* AUTO FX (af_tick): the effect planned, sounding now, one played by hand, the section seen,
                                                                  * its name shown, the last two */
    uint32_t af_start, af_end, af_bar, af_quiet, af_rng;   /* where it starts and ends (sixteenths, en_bars; end 0: none), the bar planned on, the bar it
                                                            * may plan again from (after a drop), its dice */
    uint8_t dj_break, dj_fillon;       /* its breakdown (0 short: 4 bars, 1 half the groove, 2 as long); a drum fill before it */
    uint8_t scene_quant, scene_q;      /* a scene waits for the next bar (1) or comes at once (0); the one waiting: slot + 1 */
    uint8_t key_set, key_on, sec_last; /* the drop's key change (0 same, 1 +2, 2 +3, 3 +5, 4 +7); it is on; the section last seen */
    uint8_t viz;                       /* the visualizer on the SELECT dial (VZ_*) */
    uint8_t centre_imp;                /* the sigil's middle: the eye (0) or the imp (SEL + HOME); kept with the visualizer's byte */
    uint8_t eu_n[EN_NLANE], eu_r[EN_NLANE];   /* the sequencer's Euclid tricks: each lane's hits and rotation, as last set */
    uint8_t flex_last;                 /* the Flex Beat pad pressed last (0..14) */
    uint8_t flex_lock;                 /* the Flex pattern that is latched (with the one FX_FLEX bit in fx_lock) */
    uint8_t flex_cur;                  /* the pattern the engine was last told to play (en_flex_select) */
    uint8_t saver, lights, saver_on, saver_view, saver_viz;   /* the STAGE page: the screensaver (off, 1, 3, 10 minutes), the key lights (off, show); it is on; where it came from */
    uint8_t usb_fixed;                           /* the USB page: 1, the USB audio at a fixed level, not the master knob's */
    uint8_t form;
    uint8_t start_rand;                          /* the Settings page's Start: 0 Last, 1 Random (the global record's rsv, bit 8: 0.16.26) */                                /* the Settings page's Form: 0 Song, 1 Loop (kept in the global record's last byte, bit 2: 0.16.26) */
    uint8_t fm_dy;                               /* the Settings page's Voices: 1, the DY set stands in for the DX voices (kept in set_t.usb's bit 1) */
    uint8_t scan;                                /* Scanlines: every other screen row darker (set_t.usb's bit 2) */
    uint8_t midi_map;                            /* MIDI out's map (EN_MAP_*: GM, Model:Samples, MPC Sample, SP-404; set_t.usb's bits 12-14) */
    uint8_t min_pads, min_ch;                    /* MIDI in: 1, a pad controller's notes play PERF (pads_in; set_t.usb's bit 15); 2, a Model:Samples' channels do (ms_in;
                                                  * set_t.viz's bit 3); 3, an Akai MPK mini (mpk_*; bit 6); the channel listened to, 0 all (set_t.uiv[11]) */
    uint8_t cc_mode;                             /* the FX knobs as CCs (0.15.22): bit 0 in, bit 1 out (set_t.viz's bits 4-5) */
    int8_t cc_sent[4];                           /* the FX knobs as last sent (or received: not sent back) */
    uint8_t usb_host;                            /* the USB port is the host of a USB-MIDI device (plat_usb_host); never kept */
    uint32_t min_last, min_n;                    /* MIDI in: the last message (not clock), as a USB-MIDI packet, and how many (the MIDI IN page shows them) */
    uint8_t pad_note[48];                        /* the notes the melodic pads started (banks B-D), for their note-offs */
    uint32_t saver_t;
    float glide_f[4];                  /* a scene's FX knobs gliding: where they are, where they go, which */
    int8_t glide_t[4];
    uint8_t glide_on, glide_busy;
    uint8_t vox_dirty;                 /* the player's DX7 voices changed: written when you leave them alone a moment (voices_save) */
    uint8_t set_dirty;                 /* the effects' / performing settings changed: written when you leave them alone a moment */
    uint8_t set_lazy;                  /* a cosmetic setting changed (the visualizer, the sigil): written when the music is stopped or with the next write, never as a dip in the music of its own */
    uint32_t fx_open_t;                /* when the FX menu was opened (a second tap soon after goes to the settings) */
    uint8_t fx_lastpg;                 /* the edit page of the effect key pressed last */
    uint8_t mix_trk;                   /* the mixer page's layer: pad, drone, acid, pluck, drums (0..4) */
    uint8_t gr_all;                    /* the groove page's all-lanes density, as last set */
    int8_t pat_key;                    /* a pattern key (SEL + white 1-6) held: a tap keeps the pattern, a hold plays it only while down */
    uint8_t pat_rev, pat_held, pat_up; /* the pattern to go back to (+1), the one held, and the key let go after a hold */
    uint32_t pat_t;
    uint8_t scene_g[4][13];            /* a scene's groove: the lanes' densities, their sources, the pattern */
    uint8_t jam_man;                   /* the voice the JAM page has taken off its line (1 + voice), 0 none */
    uint8_t jam_voice, jam_mode, jam_oct, jam_ans;   /* the JAM page: the voice (pluck, pad, acid, drone), the keys (scale, chords, harp), the octave (0..4: -2..+2), the answer */
    uint8_t jam_n[NKEYS], jam_vc[NKEYS], jam_note[NKEYS][4];   /* what each key is sounding, to let go */
    uint8_t ans_note[8], ans_dt[8], ans_cnt, ans_playing, ans_i;   /* the answer's memory: notes, the sixteenths between them; playing, at which */
    uint32_t ans_last, ans_t;
    uint16_t cap_t[32];                          /* Capture's memory (SEL + REC on Jam): the last 32 keys played: the sixteenth (en_bars), the note heard, the key and voice, how long it was held (sixteenths; 0: still down) */
    uint8_t cap_note[32], cap_kv[32], cap_len[32], cap_w;
    uint32_t home_t;                   /* when HOME last went down (twice quickly: the visualizers) */
    uint8_t mood;                      /* the mood macro, 0 (chill) .. 100 (peak); 50 leaves the piece as it is */
    uint8_t dj_phase, dj_fill;         /* groove 0, breakdown 1, build 2; a fill in the bar before the breakdown */
    uint8_t dj_rounds, dj_ramp, intro; /* the SET page: rounds a piece gets (1, 2, 4, 8), the tempo's climb over them, the intro (kept in set_t.usb's bits 3 ..) */
    uint8_t dj_round;                  /* the round the piece is in (from 0) */
    int16_t dj_t0, dj_tlast;           /* the ramp: the piece's tempo at its start, and the last tempo the ramp set (another: the player turned it, so it starts again from there) */
    int16_t mx_from, mx_to, mx_last;   /* the mix (0.16.1, the SET page's Mix): a piece on the bar starts at the tempo playing and glides to its own */
    uint8_t mx_bars, mx_n;             /* over mx_bars bars (0: no glide), mx_n of them gone */
    uint8_t smp_mode, smp_fx, smp_gap; /* the SAMPLE page (0.16.1): what it plays (all, drums, synths, by key), with the effects or dry, the gap between sounds */
    uint8_t smp_dry, smp_vc, smp_nn, smp_note[3];   /* dry now; the voice and notes held */
    int8_t smp_cur;                    /* the sound playing (-1: none) */
    uint32_t smp_t0, smp_off;          /* the run's start (0: none); when the held notes let go */
    uint16_t dj_n;                     /* bars in the phase */
    uint64_t fx_mask;                  /* the effects held (and locked), as last sent */
    uint64_t fx_lock;                  /* the effects latched on */
    uint8_t lock_mode;                 /* (the old SAVE lock mode: gone, kept off) */
    uint32_t lock_keys;                /* keys that turned a latched effect off: not played while they're held */
    uint32_t fx_tap;                   /* FX keys down that may yet be a tap (a latch) */
    uint32_t fx_down_t[NKEYS];
    uint8_t lane;                      /* the sequencer's lane (OCT- / OCT+) */
    uint8_t stp;                       /* the Steps page's step (a white key) */
    uint8_t astp;                      /* the Acid steps page's step (a white key) */
    uint8_t pstp;                      /* the Pluck steps page's */
    uint8_t bstp;                      /* the Bass steps page's (0.16.29) */
    uint8_t pgsl;                      /* the Progression page's slot */
    uint8_t slsl, sl_pos, sl_q;        /* the set list: the slot chosen; the one playing (0xFF none); one waiting for the bar (+ 1) */
    uint8_t as_off[16], as_oct[16];    /* its working copy of the line while it is edited (the engine takes the edits a chunk later) */
    uint16_t as_gate, as_acc, as_slide;
    uint32_t as_t;                     /* when it was last edited */
    uint8_t save_global;               /* SAVE went down on Global: the piece is stored when it comes up */
    uint8_t store_wait;                /* a store while playing: the music fading out to write it (store_tick) */
    uint8_t store_kind;                /* and what: 0 the piece's, 1 the settings, 2 the globals (quiet_write) */
    uint32_t store_t;
    char store_msg[12];
    uint8_t factory_done;              /* SEL + SAVE held on Global: the piece went back to its factory settings */
    uint8_t save_fact;                 /* SAVE went down on Global with SEL held: a factory reset if held on, never a store */
    uint32_t shift_keys;               /* keys pressed while SEL (shift) was held: not notes, not effects */
    uint8_t piece_q;                   /* a piece waiting for the bar (SEL + PRESETS): its number + 1 */
    uint8_t shift_hint;                /* SEL held alone: its hint shown this hold */
    uint8_t key_voice[NKEYS];          /* a key playing a voice: its EN_V_* + 1 (the note-off goes there) */
    uint8_t splash, theme;             /* the boot cards are up (2: the HYP-1 card); the theme drawn */
    uint8_t splash_step;               /* the HYP-1 card's steps lit */
    uint32_t boot_t, act_t;
    char msg[2][24];
    uint32_t msg_until;
    int8_t touched;                    /* the knob turned last (0..3), shown bright for a moment */
    uint32_t touch_until;
    uint32_t frame;
    uint32_t sig[3];                   /* what each band shows: redrawn when it changes */
} ui;
