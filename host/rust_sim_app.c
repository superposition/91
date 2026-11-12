/* SPDX-License-Identifier: GPL-3.0-only */
/* Transitional app capture/action ABI. No scenario parser, assertions, clock,
 * host platform state, file sinks or simulator main is compiled from C. */
#include <stdint.h>
#include <stdlib.h>
#include <stddef.h>
#include <string.h>
#define EN_HOST 1
#define __attribute__(x)
#include "sim_source.h"
extern void lcd_sync(void);
extern void lcd_fill(uint32_t,uint32_t,uint32_t,uint32_t,uint16_t);
extern void lcd_blit(uint32_t,uint32_t,uint32_t,uint32_t,const uint16_t *);
extern uint32_t fm1_sim_host_value(uint32_t,uint32_t);
#include SIM_GFX
#include SIM_APP
#include SIM_PROJECT
#include SIM_SESSION
#include SIM_UI
#include SIM_USB
static uint8_t sim_jam_scratch[JP_MAX];
void fm1_sim_boot(uint32_t seed,uint32_t legacy){en_init();project_start(seed);project_apply();jam_boot_on=0;if(!legacy)jam_session_load();lcd_fill(0,0,240,240,0);ui_init();}
void fm1_sim_reset_project(void){memset(&proj,0,sizeof proj);}
void fm1_sim_service(void){ju_song_service(sim_jam_scratch,0);}
void fm1_sim_splash(uint32_t ms){ui_splash_ms=ms;}
void fm1_sim_accel(uint32_t off){accel_off=(uint8_t)off;}
void fm1_sim_restore_detach(void){rs_buf=0;rs_len=0;rs_state=0;}
/* Object migration metadata; the Rust scenario builder owns byte edits. */
void fm1_sim_layout(uint32_t *p){const uint32_t a[]={IMG_MAGIC,IMG_HDR,IMG_REC,REC_PAR,P_TEMPO,REC_RECIPE,IMG_TAIL,OBJ_GLOB,sizeof(glob_t),GLOB_OLD,sizeof(set_t),EN_NFXS,offsetof(set_t,uiv),offsetof(set_t,gx_pat),P_NPARAMS,P_PAD};memcpy(p,a,sizeof a);}
void fm1_sim_recipe(int pc,int16_t *par,uint16_t *pool,uint16_t *pats){en_piece(pc,par,pool);en_piece_pats(pc,(uint16_t (*)[EN_NLANE])pats);}
typedef struct {float beat;uint32_t wall,gate,done;} sim_row_t;
typedef struct {uint32_t on,over,lane;float beat;sim_row_t rows[16];} sim_runner_t;
void fm1_sim_runner(sim_runner_t *out){unsigned i;out->on=kr.on;out->over=kr.over;out->lane=kr.lane;out->beat=kr.beat;for(i=0;i<16;i++)out->rows[i]=(sim_row_t){kr.row[i].beat,kr.row[i].wall,kr.row[i].gate,kr.row[i].done};}
int fm1_sim_observe(const char *what, int32_t *value)
{
    int got;
    if(!strncmp(what,"jam_",4)) {
        jp_settings_t s;en_jam_get(&s);
        if(!strcmp(what,"jam_on"))got=en_jam_on;
        else if(!strcmp(what,"jam_length"))got=en_jam_length;
        else if(!strcmp(what,"jam_key"))got=s.key+12*s.minor;
        else if(!strcmp(what,"jam_form"))got=s.form;
        else if(!strcmp(what,"jam_hold"))got=s.hold;
        else if(!strcmp(what,"jam_pending"))got=en_jam_pending;
        else if(!strcmp(what,"jam_song"))got=en_jam_source();
        else if(!strcmp(what,"jam_version")){unsigned n;got=en_jam_data(&n)[4];}
        else if(!strcmp(what,"jam_swell"))got=en_jam_option(JAM_SWELL);
        else if(!strcmp(what,"jam_touch"))got=en_jam_option(JAM_TOUCH);
        else return 0;
    }
    else if (!strcmp(what, "playing"))
        got = en_playing;
    else if (!strcmp(what, "frozen"))
        got = en_frozen;
    else if (!strcmp(what, "view"))
        got = ui.view;
    else if (!strcmp(what, "viz"))                  /* the visualizer chosen (VZ_*) */
        got = ui.viz;
    else if (!strcmp(what, "dd_changes"))          /* drum drift: patterns changed, mutations, bars left */
        got = (int)en_dd_changes;
    else if (!strcmp(what, "dd_mutations"))
        got = (int)en_dd_mutations;
    else if (!strcmp(what, "dd_left"))
        got = en_dd_left;
    else if (!strcmp(what, "pattern_synced"))
        got = proj.par[P_PATTERN] == en_pat_next;
    else if (!strcmp(what, "runner"))              /* the secret game (SEL + EDIT): on, lane, rows, gates, ... */
        got = kr.on;
    else if (!strcmp(what, "kr_lane"))
        got = kr.lane;
    else if (!strcmp(what, "kr_rows"))
        got = (int)kr.rows_born;
    else if (!strcmp(what, "kr_gates"))
        got = (int)kr.gates;
    else if (!strcmp(what, "kr_crashes"))
        got = (int)kr.crashes;
    else if (!strcmp(what, "kr_layer"))
        got = kr.layer;
    else if (!strcmp(what, "kr_over"))
        got = kr.over;
    else if (!strcmp(what, "piece"))
        got = proj.piece;
    else if (!strcmp(what, "pool"))               /* the scale, relative to the root (hex ok) */
        got = proj.pool;
    else if (!strcmp(what, "touched"))            /* (0.16.18) the page's knob lit by the last turn (or a learned setting), -1 none; a message on screen */
        got = (int32_t)(ui.touch_until - plat_ms()) > 0 ? ui.touched : -1;
    else if (!strcmp(what, "msgon"))
        got = (int32_t)(ui.msg_until - plat_ms()) > 0;
    else if (!strcmp(what, "acidgate"))           /* the acid line's steps that play (a 16-bit mask) */
        got = en_acid_gate;
    else if (!strncmp(what, "acidoff", 7))        /* the acid line's step N: scale steps above the chord */
        got = en_acid_off[atoi(what + 7) & 15];
    else if (!strncmp(what, "acidoct", 7))        /* its octave */
        got = en_acid_oct[atoi(what + 7) & 15];
    else if (!strncmp(what, "acidkind", 8)) {     /* the acid line's step N as the page says: 0 rest, 1 note, 2 accent, 3 slide, 4 accent and slide */
        const int s = atoi(what + 8) & 15, g = en_acid_gate >> s & 1, ac = en_acid_acc >> s & 1, sl = en_acid_slide >> s & 1;
        got = !g ? 0 : ac ? (sl ? 4 : 2) : (sl ? 3 : 1);
    }
    else if (!strcmp(what, "acidacc"))
        got = en_acid_acc;
    else if (!strcmp(what, "acidslide"))
        got = en_acid_slide;
    else if (!strcmp(what, "hushed"))             /* the output is faded out (a store waiting to write) */
        got = en_hushed;
    else if (!strcmp(what, "storewait"))          /* a store is waiting for the fade */
        got = ui.store_wait;
    else if (!strcmp(what, "dirty"))              /* the bank has changes not yet in flash */
        got = project_pending();
    else if (!strcmp(what, "saved"))              /* the pieces in the bank (bit i: piece i) */
        got = (int)(project_saved(0) | project_saved(1) << 1 | project_saved(2) << 2 | project_saved(3) << 3 |
                    project_saved(4) << 4 | project_saved(5) << 5 | project_saved(6) << 6 | project_saved(7) << 7 |
                    project_saved(8) << 8 | project_saved(9) << 9);
    else if (!strcmp(what, "parsbad")) {          /* settings out of range: the playing piece's and every slot's */
        int p, i;
        got = 0;
        for (p = 0; p < P_NPARAMS; p++) {
            const en_param_t *pi = en_param_info(p);
            got += proj.par[p] < pi->lo || proj.par[p] > pi->hi;
            for (i = 0; i < NPIECES; i++)
                got += proj.slot[i].par[p] < pi->lo || proj.slot[i].par[p] > pi->hi;
        }
        for (i = 0; i < NPIECES; i++)
            got += !(proj.slot[i].pool & 1u) || (proj.slot[i].pool >> 12);
    } else if (!strcmp(what, "parwide")) {        /* settings spanning 255 steps or more (none may: a stored 0xFF means "the recipe's", project.c REC_RECIPE) */
        int p;
        got = 0;
        for (p = 0; p < P_NPARAMS; p++)
            got += en_param_info(p)->hi - en_param_info(p)->lo >= 255;
    } else if (!strcmp(what, "uservoices"))         /* the player's DX7 voices loaded */
        got = en_user_count();
    else if (!strcmp(what, "edited"))             /* the pieces changed since taken up (bit i: piece i) */
        got = proj.edited;
    else if (!strcmp(what, "store_writes"))
        got = (int)fm1_sim_host_value(0,0);
    else if (!strcmp(what, "clocks_out"))
        got = (int)fm1_sim_host_value(1,0);
    else if (!strcmp(what, "sixteenths"))
        got = (int)en_bars;
    else if (!strcmp(what, "ext"))
        got = en_ext;
    else if (!strcmp(what, "midi_out"))
        got = (int)fm1_sim_host_value(2,0);
    else if (!strcmp(what, "pad_level"))
        got = (int)(en_pad_level * 100.0f);
    else if (!strcmp(what, "bass_level"))
        got = (int)(en_bass_level * 100.0f);
    else if (!strcmp(what, "chordroot"))
        got = en_chord_root;
    else if (!strcmp(what, "bspos"))              /* the bass step playing (0.16.29; -1 not playing its steps) */
        got = en_bs_pos;
    else if (!strncmp(what, "bskind", 6))         /* bskindN, bsnoteN, bsoctN: bass step N (0.16.29) */
        got = EN_BS_KIND(en_bs[atoi(what + 6) & 15]);
    else if (!strncmp(what, "bsnote", 6))
        got = EN_BS_NOTE(en_bs[atoi(what + 6) & 15]);
    else if (!strncmp(what, "bsoct", 5))
        got = EN_BS_OCT(en_bs[atoi(what + 5) & 15]);
    else if (!strcmp(what, "bsany"))              /* any bass step set (not a rest) */
        got = (en_bs[0] | en_bs[1] | en_bs[2] | en_bs[3] | en_bs[4] | en_bs[5] | en_bs[6] | en_bs[7] | en_bs[8] | en_bs[9] | en_bs[10] | en_bs[11] | en_bs[12] | en_bs[13] | en_bs[14] | en_bs[15]) != 0;
    else if (!strcmp(what, "walknext"))           /* the walk's next degree, a chord ahead (0.16.27; -1 none) */
        got = en_walk_next;
    else if (!strcmp(what, "chordalt"))           /* the chord playing: 0 plain, 1 a secondary dominant, 2 borrowed */
        got = en_chord_alt;
    else if (!strcmp(what, "midimap"))            /* the MIDI out map (USB & MIDI page): 0 GM, 1 Model:Samples, 2 MPC Sample, 3 SP-404 */
        got = ui.midi_map;
    else if (!strcmp(what, "minpads"))            /* MIDI in (MIDI IN page): pads on, the channel (0 all); a melodic pad's note sounding; a drum lit by a hit (its glow over half) */
        got = ui.min_pads;
    else if (!strcmp(what, "minch"))
        got = ui.min_ch;
    else if (!strcmp(what, "usbhost"))            /* the USB host asked for (MIDI IN page; never kept) */
        got = fm1_sim_host_value(3,0);
    else if (!strncmp(what, "padnote", 7))
        got = ui.pad_note[atoi(what + 7) % 48];
    else if (!strncmp(what, "drumglow", 8))
        got = en_drum_glow[atoi(what + 8) % EN_NDRUM] > 0.5f;
    else if (!strcmp(what, "progpos"))            /* the progression: the slot playing (-1 the walk), Play, Drift's unit, a slot's chord (0 none, 1.. a degree) and bars (1, 2, 4, 8) */
        got = en_prog_pos;
    else if (!strcmp(what, "prgplay"))
        got = en_prg_get(proj.prg, EN_PRG_PLAY, 1);
    else if (!strcmp(what, "prgunit"))
        got = en_prg_get(proj.prg, EN_PRG_UNIT, 2);
    else if (!strncmp(what, "prgslot", 7))
        got = en_prg_get(proj.prg, 3 * (atoi(what + 7) & 7), 3);
    else if (!strncmp(what, "prgtype", 7))       /* a slot's chord type (0.16.6: 0 the piece's, else EN_CH_*) */
        got = (int)(((uint32_t)proj.prk[0] | (uint32_t)proj.prk[1] << 8 | (uint32_t)proj.prk[2] << 16) >> 3 * (atoi(what + 7) & 7) & 7u);
    else if (!strncmp(what, "prgbars", 7))
        got = 1 << en_prg_get(proj.prg, 24 + 2 * (atoi(what + 7) & 7), 2);
    else if (!strcmp(what, "chordiv")) {          /* the pad's notes as intervals over the chord's root (bits) */
        int k;
        got = 0;
        for (k = 0; k < EN_NPAD; k++)
            got |= 1 << ((en_chord[k] - en_chord_root + 120) % 12);
    }
    else if (!strncmp(what, "chord", 5))          /* chordN: pad voice N's note */
        got = en_chord[atoi(what + 5) & 3];
    else if (!strcmp(what, "sec"))                /* the section: 0 groove, 1 breakdown, 2 build */
        got = en_sec;
    else if (!strcmp(what, "aflive"))             /* AUTO FX (0.16.8): one sounding now; one planned (or sounding); its effect; its end on a phrase line (1), off it (0), none (-1) */
        got = ui.af_live;
    else if (!strcmp(what, "afplanned"))
        got = ui.af_end != 0;
    else if (!strcmp(what, "affx"))
        got = ui.af_end ? ui.af_fx : -1;
    else if (!strcmp(what, "afline"))
        got = ui.af_end ? (int)(ui.af_end % (64u << (ui.fxs[EN_FXS_AF_EVERY] & 3)) == 0) : -1;
    else if (!strcmp(what, "fxmask"))             /* the effects held (FX menu) */
        got = (int)(uint32_t)ui.fx_mask;
    else if (!strncmp(what, "fxbit", 5))          /* fxbitN: effect N (FX_*) held (the mask's upper half too) */
        got = (int)(ui.fx_mask >> (atoi(what + 5) & 63) & 1u);
    else if (!strcmp(what, "bassheld"))           /* the bass's held note, its pitch class (0.16.21: PEDAL) */
        got = en_bass_held() % 12;
    else if (!strncmp(what, "padheld", 7))        /* padheldN: pad voice N's note, its pitch class */
        got = en_pad_held(atoi(what + 7)) % 12;
    else if (!strcmp(what, "entrkmute"))          /* the mutes the engine has (0.16.12: a MIDI pad changed the screen only) */
        got = (int)en_trk_heard();
    else if (!strcmp(what, "trkmute"))            /* the tracks muted (GLO held, the keys): a bit a track */
        got = ui.trk_m;
    else if (!strncmp(what, "fxk", 3))           /* fxkN: the FX knobs' values (filter, crush, throw, wash) */
        got = ui.fxk[atoi(what + 3) & 3];
    else if (!strcmp(what, "fxpage"))             /* the FX menu's page: 0 the keys, 1.. an effect's edit page */
        got = ui.fx_menu ? ui.fx_page : -1;
    else if (!strcmp(what, "fxmaskhi"))           /* the effects held: bits 32 and up */
        got = (int)(uint32_t)(ui.fx_mask >> 32);
    else if (!strcmp(what, "scenequant"))         /* scenes on the next bar (1) or at once (0) */
        got = ui.scene_quant;
    else if (!strcmp(what, "djlen"))              /* the auto-DJ's groove length: 0 .. 3 (8, 16, 32, 64 bars) */
        got = ui.dj_len;
    else if (!strcmp(what, "djbreak"))
        got = ui.dj_break;
    else if (!strcmp(what, "sceneq"))             /* the scene waiting for the bar: slot + 1 */
        got = ui.scene_q;
    else if (!strcmp(what, "mood"))               /* the mood macro, 0 .. 100 */
        got = ui.mood;
    else if (!strcmp(what, "flexq"))              /* Flex Beat: quantize, mode, state (0 idle, 1 armed, 2 playing), one shots done, the last pad */
        got = ui.fxs[EN_FXS_FX_QUANT];
    else if (!strcmp(what, "flexmode"))
        got = ui.fxs[EN_FXS_FX_MODE];
    else if (!strcmp(what, "flexstate"))
        got = en_flx_state;
    else if (!strcmp(what, "flexdone"))
        got = (int)en_flx_done;
    else if (!strcmp(what, "flexlast"))
        got = ui.flex_last;
    else if (!strcmp(what, "jamv"))               /* the JAM page: voice, keys, the answer's memory, whether it is playing */
        got = ui.jam_voice;
    else if (!strcmp(what, "jamm"))
        got = ui.jam_mode;
    else if (!strcmp(what, "anscnt"))
        got = ui.ans_cnt;
    else if (!strcmp(what, "ansplay"))
        got = ui.ans_playing;
    else if (!strcmp(what, "smp"))                /* the sampler's voices sounding */
        got = en_smp_active;
    else if (!strcmp(what, "smpmode"))            /* the sampler page's Keys setting */
        got = ui.fxs[EN_FXS_SM_MODE];
    else if (!strcmp(what, "saver"))              /* the STAGE page: the screensaver setting, it is on, the lights */
        got = ui.saver;
    else if (!strcmp(what, "saveron"))
        got = ui.saver_on;
    else if (!strcmp(what, "dyvoices"))           /* the Settings page's Voices: 1, the DY set */
        got = en_fm_dy;
    else if (!strcmp(what, "lights"))
        got = ui.lights;
    else if (!strcmp(what, "zoom"))               /* (0.16.1) the SCREEN page's Zoom, and the knob it shows now (-1: none) */
        got = glob.sl_rsv[0];
    else if (!strcmp(what, "zoomknob"))
        got = zoom_knob();
    else if (!strcmp(what, "mix"))                /* the SET page's Mix, and the glide's bars (0: none) */
        got = glob.sl_rsv[1];
    else if (!strcmp(what, "mixbars"))
        got = ui.mx_bars;
    else if (!strcmp(what, "samplecur"))          /* the SAMPLE page: the sound playing (-1 none), a run going, dry */
        got = ui.smp_cur;
    else if (!strcmp(what, "samplerun"))
        got = ui.smp_t0 != 0;
    else if (!strcmp(what, "sampledry"))
        got = ui.smp_dry;
    else if (!strcmp(what, "glide"))              /* an FX knob still gliding to a scene's value (bits) */
        got = ui.glide_on;
    else if (!strcmp(what, "keyon"))              /* the drop's key change is on */
        got = ui.key_on;
    else if (!strcmp(what, "keyset"))
        got = ui.key_set;
    else if (!strcmp(what, "moodacid"))           /* the acid density as the engine is given it */
        got = mood_val(P_AC_DENS, proj.par[P_AC_DENS]);
    else if (!strcmp(what, "genre"))              /* the bank of pieces: 0 the original, 1 lofi */
        got = proj.genre;
    else if (!strcmp(what, "fxbank"))             /* the FX keys' bank (the dial in the FX menu) */
        got = fx_bank;
    else if (!strcmp(what, "undook"))
        got = ui.undo_ok;
    else if (!strcmp(what, "gxpat"))              /* the gate's pattern (bit s: step s open) */
        got = ui.gx_pat;
    else if (!strcmp(what, "gxpreset"))
        got = ui.gx_preset;
    else if (!strncmp(what, "fxs", 3))            /* fxsN: the effects' setting N (EN_FXS_*) */
        got = ui.fxs[atoi(what + 3) % EN_NFXS];
    else if (!strcmp(what, "scenes"))             /* the scenes stored (a bit a slot) */
        got = ui.scene_set;
    else if (!strncmp(what, "setsl", 5))          /* the set list's slot N (0.15.21): genre * 10 + piece + 1, 0 empty */
        got = glob.sl[atoi(what + 5) % SL_N];
    else if (!strcmp(what, "setpos"))             /* the slot playing (255 none) */
        got = ui.sl_pos;
    else if (!strcmp(what, "seton"))              /* the set list's Play */
        got = glob.sl_on;
    else if (!strcmp(what, "djon"))               /* the auto-DJ: on */
        got = ui.dj_on;
    else if (!strcmp(what, "djphase"))            /* groove 0, breakdown 1, build 2 */
        got = ui.dj_phase;
    else if (!strcmp(what, "djn"))                /* bars in the phase */
        got = ui.dj_n;
    else if (!strcmp(what, "trksolo"))
        got = ui.trk_s;
    else if (!strcmp(what, "tempo"))
        got = proj.par[P_TEMPO];
    else if (!strcmp(what, "pat"))                /* the drum pattern playing (0..5) */
        got = en_pat_now;
    else if (!strncmp(what, "lanesrc", 7) && what[7] >= '0' && what[7] <= '5')   /* the pattern lane N plays (0..5) */
        got = en_lane_src[what[7] - '0'];
    else if (!strncmp(what, "lanemask", 8) && what[8] >= '0' && what[8] <= '5')  /* lane N of the pattern playing, at its density, as it would sound (a 16-bit mask) */
        got = en_lane_mask(what[8] - '0', en_lane_src[what[8] - '0'], proj.par[P_DENS + (what[8] - '0')]);
    else if (!strncmp(what, "lanewrit", 8) && what[8] >= '0' && what[8] <= '5')  /* the same, as written */
        got = en_pats[en_lane_src[what[8] - '0']][what[8] - '0'];
    else if (!strncmp(what, "mod", 3) && what[3] >= '0' && what[3] <= '5' && what[4] == '_')   /* a lane's step trick byte: modL_S (lane L, step S, 0 .. 15) */
        got = en_mod[what[3] - '0'][atoi(what + 5) & 15];
    else if (!strcmp(what, "rsvalid"))            /* the last restore command's check: 1 valid */
        got = fm1_sim_host_value(4,0);
    else if (!strcmp(what, "rsstate"))            /* 0 none, 1 waiting, 2 written, 3 not written */
        got = ui_restore_state();
    else if (!strcmp(what, "storelock"))
        got = fm1_sim_host_value(5,0);
    else if (!strcmp(what, "patkey"))             /* the held pattern's state: 1 a pattern key is down, 2 one let go and waiting to go back */
        got = ui.pat_key >= 0 ? 1 : ui.pat_up ? 2 : 0;
    else if (!strcmp(what, "patnext"))            /* the one waiting for the bar */
        got = en_pat_next;
    else if (!strncmp(what, "lane", 4))           /* laneL: lane L's steps in the pattern chosen (bits) */
        got = (int)en_pats[proj.par[P_PATTERN]][atoi(what + 4) % EN_NLANE];
    else if (!strncmp(what, "pats", 4))           /* patsP_L: pattern P's lane L (bits) */
        got = (int)en_pats[atoi(what + 4) % EN_NPAT][atoi(strchr(what, '_') ? strchr(what, '_') + 1 : "0") % EN_NLANE];
    else if (!strcmp(what, "fxlock"))             /* the effects locked on (SAVE with their keys held) */
        got = (int)ui.fx_lock;
    else if (!strcmp(what, "fxon"))               /* the FX menu is on */
        got = ui.fx_menu;
    else if (!strcmp(what, "held"))               /* keys held playing a voice */
        got = en_held;
    else if (!strcmp(what, "remote"))             /* remote (SEL + GLO) on; the browser answering */
        got = rm.on;
    else if (!strcmp(what, "remotelink"))
        got = rm.link;
    else if (!strncmp(what, "ccout", 5))          /* ccoutN: control changes N sent so far */
        got = (int)fm1_sim_host_value(6,atoi(what + 5) & 127);
    else if (!strcmp(what, "lastvel"))            /* the last note-on out's velocity */
        got = fm1_sim_host_value(7,0);
    else if (!strcmp(what, "ccmode"))             /* MIDI IN's Knob CCs: bit 0 in, bit 1 out */
        got = ui.cc_mode;
    else if (!strcmp(what, "lastout"))            /* the last MIDI out: status << 16 | d1 << 8 | d2 */
        got = (int)fm1_sim_host_value(8,0);
    else if (!strcmp(what, "litbtn"))             /* the buttons lit (bit b: B_*) */
        got = (int)fm1_sim_host_value(9,0);
    else if (!strcmp(what, "keychord")) {         /* the keys show the chord playing (0.15.21): lit = its tones, dim = the rest of the scale, none both, some dim */
        const uint32_t ch = chord_pcs(), ap = pool_abs();
        uint32_t i, ok = fm1_sim_host_value(10,0) != 0 && !(fm1_sim_host_value(11,0) & fm1_sim_host_value(10,0));
        for (i = 0; i < NKEYS; i++) {
            const uint32_t pc = (5u + i) % 12u, l = fm1_sim_host_value(11,0) >> i & 1u, d = fm1_sim_host_value(10,0) >> i & 1u;
            if (l != (ch >> pc & 1u) || d != ((ap >> pc & 1u) && !(ch >> pc & 1u)))
                ok = 0;
        }
        got = (int)ok;
    } else if (!strcmp(what, "dimkeys"))            /* the keys lit dim (the scale while a chord plays, 0.15.21) */
        got = (int)fm1_sim_host_value(10,0);
    else if (!strcmp(what, "litkeys"))            /* the keys lit (bit k: key k) */
        got = (int)fm1_sim_host_value(11,0);
    else if (!strcmp(what, "seqlane"))            /* the sequencer's lane (OCT- / OCT+) */
        got = ui.lane;
    else if (!strcmp(what, "acidnotes"))          /* the acid loop's steps that play */
        got = __builtin_popcount(en_acid_gate);
    else if (!strcmp(what, "theme"))
        got = proj.par[P_THEME];
    else if (!strncmp(what, "enpar", 5))        /* enparN: the engine's own value (the mood's, the JAM page's hand switch) */
        got = en_par_now(atoi(what + 5));
    else if (!strncmp(what, "par", 3))          /* parN: parameter N (amb.h P_*) */
        got = proj.par[atoi(what + 3)];
    else return 0;
    *value=got;return 1;
}
