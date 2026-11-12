#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Generate the common UI carrier ABI, never UI policy or persisted bytes."""
from pathlib import Path
import argparse
ROOT=Path(__file__).resolve().parents[1]
EFFECT_SETTINGS = 96
FIELDS = [('u8', 'view', []),
 ('u8', 'last_dial', []),
 ('u32', 'btn', []),
 ('u32', 'keys', []),
 ('u32', 'btn_used', []),
 ('u32', 'enc_t', [7]),
 ('u8', 'enc_run', [7]),
 ('u32', 'down_t', [14]),
 ('u32', 'turn_t', []),
 ('u8', 'jam_active', []),
 ('u32', 'jam_revision', []),
 ('u8', 'help', []),
 ('u8', 'save_held', []),
 ('u8', 'seq_held', []),
 ('u8', 'fx_menu', []),
 ('i8', 'fxk', [4]),
 ('u8', 'grp_last', [8]),
 ('u8', 'fx_page', []),
 ('u8', 'fxs', [96]),
 ('u8', 'gx_preset', []),
 ('u8', 'gx_tgt', []),
 ('u8', 'pm_tgt', []),
 ('u16', 'gx_pat', []),
 ('u8', 'trk_m', []),
 ('u8', 'trk_s', []),
 ('u64', 'scene_lock', [4]),
 ('u8', 'scene_m', [4]),
 ('u8', 'scene_s', [4]),
 ('i8', 'scene_k', [4, 4]),
 ('u8', 'scene_set', []),
 ('u8', 'scene_pend', []),
 ('u32', 'scene_t', [4]),
 ('u64', 'undo_lock', []),
 ('u32', 'tap_t', [4]),
 ('u8', 'undo_m', []),
 ('u8', 'undo_s', []),
 ('u8', 'undo_ok', []),
 ('u8', 'tap_n', []),
 ('i8', 'undo_k', [4]),
 ('u8', 'dj_on', []),
 ('u8', 'dj_len', []),
 ('u8', 'dj_piece', []),
 ('u8', 'af_fx', []),
 ('u8', 'af_live', []),
 ('u8', 'af_user', []),
 ('u8', 'af_sec', []),
 ('u8', 'af_said', []),
 ('u8', 'af_last', [2]),
 ('u32', 'af_start', []),
 ('u32', 'af_end', []),
 ('u32', 'af_bar', []),
 ('u32', 'af_quiet', []),
 ('u32', 'af_rng', []),
 ('u8', 'dj_break', []),
 ('u8', 'dj_fillon', []),
 ('u8', 'scene_quant', []),
 ('u8', 'scene_q', []),
 ('u8', 'key_set', []),
 ('u8', 'key_on', []),
 ('u8', 'sec_last', []),
 ('u8', 'viz', []),
 ('u8', 'centre_imp', []),
 ('u8', 'eu_n', [6]),
 ('u8', 'eu_r', [6]),
 ('u8', 'flex_last', []),
 ('u8', 'flex_lock', []),
 ('u8', 'flex_cur', []),
 ('u8', 'saver', []),
 ('u8', 'lights', []),
 ('u8', 'saver_on', []),
 ('u8', 'saver_view', []),
 ('u8', 'saver_viz', []),
 ('u8', 'usb_fixed', []),
 ('u8', 'form', []),
 ('u8', 'start_rand', []),
 ('u8', 'fm_dy', []),
 ('u8', 'scan', []),
 ('u8', 'midi_map', []),
 ('u8', 'min_pads', []),
 ('u8', 'min_ch', []),
 ('u8', 'cc_mode', []),
 ('i8', 'cc_sent', [4]),
 ('u8', 'usb_host', []),
 ('u32', 'min_last', []),
 ('u32', 'min_n', []),
 ('u8', 'pad_note', [48]),
 ('u32', 'saver_t', []),
 ('f32', 'glide_f', [4]),
 ('i8', 'glide_t', [4]),
 ('u8', 'glide_on', []),
 ('u8', 'glide_busy', []),
 ('u8', 'vox_dirty', []),
 ('u8', 'set_dirty', []),
 ('u8', 'set_lazy', []),
 ('u32', 'fx_open_t', []),
 ('u8', 'fx_lastpg', []),
 ('u8', 'mix_trk', []),
 ('u8', 'gr_all', []),
 ('i8', 'pat_key', []),
 ('u8', 'pat_rev', []),
 ('u8', 'pat_held', []),
 ('u8', 'pat_up', []),
 ('u32', 'pat_t', []),
 ('u8', 'scene_g', [4, 13]),
 ('u8', 'jam_man', []),
 ('u8', 'jam_voice', []),
 ('u8', 'jam_mode', []),
 ('u8', 'jam_oct', []),
 ('u8', 'jam_ans', []),
 ('u8', 'jam_n', [27]),
 ('u8', 'jam_vc', [27]),
 ('u8', 'jam_note', [27, 4]),
 ('u8', 'ans_note', [8]),
 ('u8', 'ans_dt', [8]),
 ('u8', 'ans_cnt', []),
 ('u8', 'ans_playing', []),
 ('u8', 'ans_i', []),
 ('u32', 'ans_last', []),
 ('u32', 'ans_t', []),
 ('u16', 'cap_t', [32]),
 ('u8', 'cap_note', [32]),
 ('u8', 'cap_kv', [32]),
 ('u8', 'cap_len', [32]),
 ('u8', 'cap_w', []),
 ('u32', 'home_t', []),
 ('u8', 'mood', []),
 ('u8', 'dj_phase', []),
 ('u8', 'dj_fill', []),
 ('u8', 'dj_rounds', []),
 ('u8', 'dj_ramp', []),
 ('u8', 'intro', []),
 ('u8', 'dj_round', []),
 ('i16', 'dj_t0', []),
 ('i16', 'dj_tlast', []),
 ('i16', 'mx_from', []),
 ('i16', 'mx_to', []),
 ('i16', 'mx_last', []),
 ('u8', 'mx_bars', []),
 ('u8', 'mx_n', []),
 ('u8', 'smp_mode', []),
 ('u8', 'smp_fx', []),
 ('u8', 'smp_gap', []),
 ('u8', 'smp_dry', []),
 ('u8', 'smp_vc', []),
 ('u8', 'smp_nn', []),
 ('u8', 'smp_note', [3]),
 ('i8', 'smp_cur', []),
 ('u32', 'smp_t0', []),
 ('u32', 'smp_off', []),
 ('u16', 'dj_n', []),
 ('u64', 'fx_mask', []),
 ('u64', 'fx_lock', []),
 ('u8', 'lock_mode', []),
 ('u32', 'lock_keys', []),
 ('u32', 'fx_tap', []),
 ('u32', 'fx_down_t', [27]),
 ('u8', 'lane', []),
 ('u8', 'stp', []),
 ('u8', 'astp', []),
 ('u8', 'pstp', []),
 ('u8', 'bstp', []),
 ('u8', 'pgsl', []),
 ('u8', 'slsl', []),
 ('u8', 'sl_pos', []),
 ('u8', 'sl_q', []),
 ('u8', 'as_off', [16]),
 ('u8', 'as_oct', [16]),
 ('u16', 'as_gate', []),
 ('u16', 'as_acc', []),
 ('u16', 'as_slide', []),
 ('u32', 'as_t', []),
 ('u8', 'save_global', []),
 ('u8', 'store_wait', []),
 ('u8', 'store_kind', []),
 ('u32', 'store_t', []),
 ('char', 'store_msg', [12]),
 ('u8', 'factory_done', []),
 ('u8', 'save_fact', []),
 ('u32', 'shift_keys', []),
 ('u8', 'piece_q', []),
 ('u8', 'shift_hint', []),
 ('u8', 'key_voice', [27]),
 ('u8', 'splash', []),
 ('u8', 'theme', []),
 ('u8', 'splash_step', []),
 ('u32', 'boot_t', []),
 ('u32', 'act_t', []),
 ('char', 'msg', [2, 24]),
 ('u32', 'msg_until', []),
 ('i8', 'touched', []),
 ('u32', 'touch_until', []),
 ('u32', 'frame', []),
 ('u32', 'sig', [3])]

CTYPES={'u8':'uint8_t','i8':'int8_t','u16':'uint16_t','i16':'int16_t','u32':'uint32_t','u64':'uint64_t','f32':'float','char':'char'}
def rust_type(base,dims):
    value='u8' if base=='char' else base
    for n in reversed(dims):value=f'[{value}; {n}]'
    return value
def zero(base,dims):
    value='0.0' if base=='f32' else '0'
    for n in reversed(dims):value=f'[{value}; {n}]'
    return value
def render():
    r='// SPDX-License-Identifier: GPL-3.0-only\n// Generated by tools/ui_abi.py; plain fields, no pointers or raw padding IO.\n#[repr(C)]\npub struct State {\n'
    c='/* SPDX-License-Identifier: GPL-3.0-only */\n/* Generated by tools/ui_abi.py. */\n#ifndef FM1_RUST_UI_STATE_H\n#define FM1_RUST_UI_STATE_H\n#include <stdint.h>\ntypedef struct {\n'
    clear='impl State {\n    pub fn clear(&mut self) {\n'
    for base,name,dims in FIELDS:
        r+=f'    pub {name}: {rust_type(base,dims)},\n'
        c+='    '+CTYPES[base]+' '+name+''.join(f'[{n}]'for n in dims)+';\n'
        clear+=f'        self.{name} = {zero(base,dims)};\n' if not dims else f'        self.{name}.fill({zero(base,dims[1:])});\n'
    r+='}\n'+clear+'    }\n}\n'
    c+='} fm1_ui_state;\n'
    c+=f'_Static_assert(EN_NFXS == {EFFECT_SETTINGS}, "UI effects settings count");\n'
    c+='_Static_assert(NE == 7 && NB == 14 && NKEYS == 27 && EN_NLANE == 6, "UI control dimensions");\n'
    c+=f'#define FM1_UI_FIELD_COUNT {len(FIELDS)}u\n#define FM1_UI_LAYOUT_WORDS {2+2*len(FIELDS)}u\n'
    c+='#define FM1_UI_FIELDS(X) '+(' \\\n    '.join('X('+n+')'for _,n,_ in FIELDS))+'\n'
    c+='#endif\n'
    bridge='''// SPDX-License-Identifier: GPL-3.0-only
// Generated field projection by tools/ui_abi.py; no raw padding read.
use fm1_app::ui_state::State;
/// # Safety
/// State is aligned/live and output has capacity distinct writable u32 slots;
/// no concurrent/reentrant access. Numeric layout only, never state bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_layout(state:*const State,out:*mut u32,capacity:u32)->u32 {
'''
    bridge+=f'    const WORDS:u32={2+2*len(FIELDS)};\n'
    bridge+='    if state.is_null()||out.is_null()||capacity<WORDS {crate::app::reject();return 0;}\n'
    bridge+='    // SAFETY: allocated state and complete disjoint output; pointer projections do not read padding.\n    unsafe {\n'
    bridge+='        out.add(0).write(core::mem::size_of::<State>() as u32);\n        out.add(1).write(core::mem::align_of::<State>() as u32);\n'
    for i,(base,name,dims) in enumerate(FIELDS):
        bridge+=f'        out.add({2+i*2}).write((core::ptr::addr_of!((*state).{name}) as usize).wrapping_sub(state as usize) as u32);\n'
        bridge+=f'        out.add({3+i*2}).write(core::mem::size_of::<{rust_type(base,dims)}>() as u32);\n'
    bridge+='    }\n    WORDS\n}\n'
    return {ROOT/'rust/fm1-app/src/ui_state.rs':r,ROOT/'firmware/src/dsp/rust_ui_state.h':c,ROOT/'rust/fm1-jam-bridge/src/ui_layout.rs':bridge}
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
    for path,text in render().items():
        if a.check:
            if not path.exists()or path.read_text()!=text:raise SystemExit('stale generated UI carrier: '+str(path))
        else:path.write_bytes(text.encode())
if __name__=='__main__':main()
