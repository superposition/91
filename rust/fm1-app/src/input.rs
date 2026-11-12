// SPDX-License-Identifier: GPL-3.0-only
//! Held controls, keys and encoder routing. State loans end before all effects.
use crate::{buttons::page, controls, ui::State};
use fm1_types::ParamId;
const FX: u32 = 0;
const SEL: u32 = 1;
const ENV: u32 = 2;
const EDIT: u32 = 4;
const GLOBAL: u32 = 5;
const HOME: u32 = 6;
const SAVE: u32 = 7;
const SEQ: u32 = 9;
const PLAY: u32 = 10;
const REC: u32 = 11;
const OCT_DOWN: u32 = 12;
const OCT_UP: u32 = 13;
const GLOBAL_PAGE: u8 = 32;
const JAM_PAGE: u8 = 34;
const WHITE: [u8; 16] = [0, 2, 4, 6, 7, 9, 11, 12, 14, 16, 18, 19, 21, 23, 24, 26];
const BLACK: [u8; 11] = [1, 3, 5, 8, 10, 13, 15, 17, 20, 22, 25];
const HAND: [ParamId; 3] = [ParamId::PAD_MAN, ParamId::DR_MAN, ParamId::AC_MAN];
#[repr(u32)]
pub enum Query {
    Now,
    Buttons,
    Keys,
    Encoder,
    Playing,
    Section,
    DrumPosition,
    RunnerOn,
    FxBank,
    FlexDone,
    Parameter,
    FxPosition,
    SettingPosition,
    FxOrder,
    SettingOrder,
    FxCount,
    SettingCount,
    FxPageBit,
    FxPageHold,
    FxBitLow,
    FxBitHigh,
    KeyFlex,
    FxKeyPage,
    FlexPattern,
    FillKey,
    FlexEffect,
    FillEffect,
    RiserEffect,
    SetlistPlay,
    SetlistStep,
    Piece,
    Genre,
    GlobalReserved,
    Jam,
    RemoteAllowed,
    ViewKnob,
    GenreBars,
    ProgressionFillCount,
}
#[repr(u32)]
pub enum Action {
    RunnerInput,
    RunnerStart,
    Button,
    RemoteEnter,
    Play,
    Knob,
    Note,
    ShiftLane,
    JamCapture,
    View,
    PieceStore,
    ShiftKey,
    MixKey,
    Fill,
    FlexSelect,
    FlexHold,
    Effects,
    SampleKeys,
    GateStep,
    StepToggle,
    ProgressionFill,
    GlobalReserved,
    Pedal,
    Globals,
    Jam,
    ProjectApply,
    JamDown,
    JamUp,
    ToggleNote,
    Midi,
    ControlOut,
    Turn,
    FxBank,
    Genre,
    Setlist,
    SetlistBar,
    Piece,
    PieceBar,
    Mood,
    ShiftSound,
    Mode,
    PieceFactory,
    Section,
    SequenceTrick,
    ProgressionUnit,
    Help,
}
#[repr(u32)]
pub enum Notice {
    FxPage,
    RemoteStop,
    StopFirst,
    ByHandPage,
    ByHand,
    Links,
    Progression,
    Fill,
    LanePattern,
    Pedal,
    Tempo,
    Visual,
    Mood,
    SetNext,
    PieceNext,
    Factory,
    Drop,
    Drift,
    Mutates,
    FxBank,
}
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Jam {
    On,
    Key,
    Bars,
    Form,
    Move,
    Energy,
    Hold,
    Bass,
    Idea,
    Song = 11,
    Swell,
    Variation,
    KeysProgram,
    BassProgram,
    Touch,
    Release,
    BassGate,
    SwellVoice,
    BassAttack,
    Brightness,
}
pub trait Port {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R;
    fn with_cache<R>(&mut self, f: impl FnOnce(&mut u32, &mut u8) -> R) -> R;
    fn query(&mut self, q: Query, first: u32, second: u32) -> u32;
    fn action(&mut self, a: Action, first: u32, second: u32) -> u32;
    fn notice(&mut self, n: Notice, first: u32, second: u32);
}
macro_rules! ui {
    ($p:ident,$u:ident,$e:expr) => {
        $p.with_ui(|$u| $e)
    };
}
fn q(p: &mut impl Port, q: Query) -> u32 {
    p.query(q, 0, 0)
}
fn parameter(p: &mut impl Port, id: ParamId) -> i32 {
    p.query(Query::Parameter, id.index() as u32, 0) as i32
}
fn act(p: &mut impl Port, a: Action) {
    p.action(a, 0, 0);
}
fn say(p: &mut impl Port, n: Notice) {
    p.notice(n, 0, 0);
}
fn held(mask: u32, key: u32) -> bool {
    mask & (1u32 << key) != 0
}
fn bit(index: u32) -> u64 {
    1u64.checked_shl(index).unwrap_or(0)
}
fn fxbit(p: &mut impl Port, key: u32) -> u64 {
    let lo = p.query(Query::FxBitLow, key, 0);
    let hi = p.query(Query::FxBitHigh, key, 0);
    u64::from(lo) | (u64::from(hi) << 32)
}
fn note(p: &mut impl Port, voice: u8, key: u8, on: bool) {
    p.action(
        Action::Note,
        u32::from(voice) | (u32::from(key) << 8),
        u32::from(on),
    );
}
fn jam(p: &mut impl Port, field: Jam) -> i32 {
    p.query(Query::Jam, field as u32, 0) as i32
}
fn set_jam(p: &mut impl Port, field: Jam, value: i32) {
    p.action(Action::Jam, field as u32, value as u32);
}

/// Returns immediately when game/remote/saver consumes the complete input poll.
fn button_edges(p: &mut impl Port, buttons: u32, keys: u32) -> bool {
    let changes = ui!(p, u, {
        let changed = buttons ^ u.btn;
        u.btn = buttons;
        changed
    });
    for index in 0..14u32 {
        let mask = 1u32 << index;
        if changes & mask == 0 {
            continue;
        }
        if buttons & mask != 0 {
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.btn_used &= !mask;
                u.down_t[index as usize] = now;
                if index == SAVE {
                    u.save_held = 0;
                    u.save_global = 0;
                }
            });
            if index == SAVE && ui!(p, u, u.view == GLOBAL_PAGE && u.fx_menu == 0) {
                ui!(p, u, {
                    u.save_global = 1;
                    u.factory_done = 0;
                    u.save_fact = u8::from(held(buttons, SEL));
                    if u.save_fact != 0 {
                        u.btn_used |= 1u32 << SEL;
                    }
                    u.btn_used |= mask;
                    u.save_held = 1;
                });
            }
            if index == FX && held(buttons, SEL) {
                let (menu, page) = ui!(p, u, (u.fx_menu, u.fx_page));
                let next = if menu != 0
                    && (p.query(Query::SettingPosition, u32::from(page), 0) as i32) < 0
                {
                    (p.query(Query::FxPosition, u32::from(page), 0) as i32) + 1
                } else {
                    0
                };
                let count = q(p, Query::FxCount) as i32;
                let page = if next >= 0 && next < count {
                    p.query(Query::FxOrder, next as u32, 0) as u8
                } else {
                    0
                };
                ui!(p, u, {
                    u.fx_page = page;
                    u.fx_menu = 1;
                    u.lock_mode = 0;
                });
                p.notice(Notice::FxPage, u32::from(page), 0);
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
            }
            if index == SEQ {
                ui!(p, u, u.seq_held = 0);
            }
            if index == GLOBAL && held(buttons, SEL) && q(p, Query::RemoteAllowed) != 0 {
                if q(p, Query::Playing) != 0 {
                    ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
                    say(p, Notice::RemoteStop);
                } else {
                    act(p, Action::RemoteEnter);
                    return true;
                }
            }
            if index == EDIT && held(buttons, SEL) && q(p, Query::Playing) != 0 {
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
                say(p, Notice::StopFirst);
            } else if index == EDIT && held(buttons, SEL) {
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
                p.action(Action::Play, 1, 0);
                ui!(p, u, {
                    u.btn = buttons;
                    u.keys = keys;
                });
                act(p, Action::RunnerStart);
                return true;
            }
            if index == REC && held(buttons, SEL) {
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
                act(p, Action::JamCapture);
                continue;
            }
            if index == ENV && held(buttons, SEL) {
                let view = ui!(p, u, u.view);
                if let Some(voice) = controls::voice_of(i32::from(view)) {
                    let parameter_id = HAND[voice as usize];
                    let value = i32::from(parameter(p, parameter_id) == 0);
                    p.action(Action::Knob, parameter_id.index() as u32, value as u32);
                    if parameter(p, parameter_id) == 0 {
                        for key in 0..27usize {
                            if ui!(p, u, u.key_voice[key] == voice + 1) {
                                note(p, voice, 53 + key as u8, false);
                                ui!(p, u, u.key_voice[key] = 0);
                            }
                        }
                    }
                    let by_hand = u32::from(parameter(p, parameter_id) != 0);
                    p.notice(Notice::ByHand, u32::from(voice), by_hand);
                } else {
                    say(p, Notice::ByHandPage);
                }
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
            }
            if index == SEQ && held(buttons, SEL) {
                let value = i32::from(parameter(p, ParamId::LINKS) == 0);
                p.action(Action::Knob, ParamId::LINKS.index() as u32, value as u32);
                let on = u32::from(parameter(p, ParamId::LINKS) != 0);
                p.notice(Notice::Links, on, 0);
                ui!(p, u, {
                    u.btn_used |= mask | 1u32 << SEL;
                    u.seq_held = 1;
                });
            }
            if index == SEL {
                ui!(p, u, u.shift_hint = 0);
            }
            if (index == OCT_DOWN || index == OCT_UP)
                && held(buttons, SEL)
                && ui!(p, u, u.view == page::SEQUENCER)
            {
                p.action(Action::ShiftLane, u32::from(index == OCT_UP), 0);
                ui!(p, u, u.btn_used |= mask | 1u32 << SEL);
            }
            if index == PLAY {
                p.action(Action::Button, index, 0);
            }
        } else if index == SAVE && ui!(p, u, u.save_global != 0) {
            let store = ui!(p, u, {
                u.save_global = 0;
                u.factory_done == 0 && u.save_fact == 0
            });
            if store {
                act(p, Action::PieceStore);
            }
        } else if ui!(p, u, u.btn_used & mask == 0) && index != PLAY {
            if index == SEQ && ui!(p, u, u.view == page::SEQUENCER) {
                p.action(Action::View, 20, 0);
            } else {
                p.action(Action::Button, index, 0);
            }
        }
    }
    false
}

fn key_edges(p: &mut impl Port, buttons: u32, keys: u32) -> u32 {
    let changes = ui!(p, u, {
        let changed = keys ^ u.keys;
        u.keys = keys;
        u.shift_keys &= keys;
        changed
    });
    let pattern = ui!(p, u, u.pat_key);
    if (0..27).contains(&pattern) && !held(keys, pattern as u32) {
        ui!(p, u, u.pat_key = -1);
        let now = q(p, Query::Now);
        ui!(
            p,
            u,
            if now.wrapping_sub(u.pat_t) < 250 {
                u.pat_rev = 0;
            } else {
                u.pat_up = 1;
            }
        );
    }
    if held(buttons, SEL) && changes & keys != 0 && ui!(p, u, u.view == JAM_PAGE && u.fx_menu == 0)
    {
        ui!(p, u, u.btn_used |= 1u32 << SEL);
    } else if held(buttons, SEL) && changes & keys != 0 {
        ui!(p, u, {
            u.shift_keys |= changes & keys;
            u.btn_used |= 1u32 << SEL;
        });
        for key in 0..27u32 {
            if held(changes & keys, key) {
                p.action(Action::ShiftKey, key, 0);
            }
        }
    }
    let performance = ui!(
        p,
        u,
        (u.view == page::PERFORM || u.view == page::VISUAL) && u.fx_menu == 0
    );
    let pressed = ui!(p, u, changes & keys & !u.shift_keys);
    if !held(buttons, SEL) && (held(buttons, GLOBAL) || performance) && pressed != 0 {
        ui!(p, u, {
            u.shift_keys |= pressed;
            if held(buttons, GLOBAL) {
                u.btn_used |= 1u32 << GLOBAL;
            }
        });
        for key in 0..27u32 {
            if held(pressed, key) {
                p.action(Action::MixKey, key, 0);
            }
        }
    }
    if held(buttons, EDIT) && !held(buttons, SEL) && ui!(p, u, !held(u.btn_used, EDIT)) {
        let now = q(p, Query::Now);
        if ui!(p, u, now.wrapping_sub(u.down_t[EDIT as usize]) >= 400) {
            ui!(p, u, {
                u.btn_used |= 1u32 << EDIT;
                u.fx_menu = 0;
                u.fx_page = 0;
            });
            p.action(Action::View, 5, 0);
            say(p, Notice::Progression);
        }
    }
    if held(buttons, GLOBAL) && !held(buttons, SEL) && ui!(p, u, !held(u.btn_used, GLOBAL)) {
        let now = q(p, Query::Now);
        if ui!(p, u, now.wrapping_sub(u.down_t[GLOBAL as usize]) >= 400) {
            ui!(p, u, u.btn_used |= 1u32 << GLOBAL);
        }
    }
    ui!(p, u, {
        u.lock_keys &= keys;
        if u.fx_menu == 0 || u.fx_page != 0 {
            u.fx_tap = 0;
        }
    });
    changes
}

fn effects(p: &mut impl Port, keys: u32, changes: u32) {
    let bank = q(p, Query::FxBank);
    if ui!(
        p,
        u,
        u.fx_menu != 0 && ((u.fx_page == 0 && bank == 3) || u.fx_page == 20)
    ) {
        let down = ui!(p, u, changes & keys & !u.shift_keys);
        for key in 0..27u32 {
            if held(down, key) {
                note(p, 3, 53 + key as u8, true);
                ui!(p, u, u.key_voice[key as usize] = 4);
            }
        }
    }
    if ui!(p, u, u.fx_menu != 0 && u.fx_page == 0) {
        let down = ui!(p, u, changes & keys & !u.shift_keys);
        let up = changes & !keys;
        for key in 0..27u32 {
            let mask = fxbit(p, key);
            if bank == 0 && key == q(p, Query::FillKey) && held(down, key) {
                ui!(p, u, u.fx_open_t = 0);
                act(p, Action::Fill);
                say(p, Notice::Fill);
                continue;
            }
            if mask == 0 {
                continue;
            }
            if held(down, key) {
                ui!(p, u, u.fx_open_t = 0);
                let flex = p.query(Query::KeyFlex, key, 0) as i32;
                if ui!(
                    p,
                    u,
                    u.fx_lock & mask != 0 && (flex < 0 || i32::from(u.flex_lock) == flex)
                ) {
                    ui!(p, u, {
                        u.fx_lock &= !mask;
                        u.lock_keys |= 1u32 << key;
                    });
                } else {
                    let now = q(p, Query::Now);
                    ui!(p, u, {
                        u.fx_down_t[key as usize] = now;
                        u.fx_tap |= 1u32 << key;
                    });
                    let page = p.query(Query::FxKeyPage, key, 0);
                    if page != 0 {
                        ui!(p, u, u.fx_lastpg = page as u8);
                    }
                }
            }
            if held(up, key) && ui!(p, u, held(u.fx_tap, key)) {
                ui!(p, u, u.fx_tap &= !(1u32 << key));
                let now = q(p, Query::Now);
                if ui!(p, u, now.wrapping_sub(u.fx_down_t[key as usize]) < 250) {
                    ui!(p, u, u.fx_lock |= mask);
                    let flex = p.query(Query::KeyFlex, key, 0) as i32;
                    if flex >= 0 {
                        ui!(p, u, u.flex_lock = flex as u8);
                    }
                }
            }
        }
    }
    ui!(
        p,
        u,
        if u.fx_menu == 0 {
            u.lock_mode = 0;
        }
    );
    let done = q(p, Query::FlexDone);
    let flex_bit = bit(q(p, Query::FlexEffect));
    let finished = p.with_cache(|seen, _| {
        let changed = *seen != done;
        *seen = done;
        changed
    });
    if finished {
        ui!(p, u, u.fx_lock &= !flex_bit);
    }
    let mut mask = ui!(p, u, u.fx_lock);
    let (menu, page) = ui!(p, u, (u.fx_menu, u.fx_page));
    if menu != 0 && page != 0 {
        let effect = p.query(Query::FxPageBit, u32::from(page) - 1, 0);
        if effect != 63 {
            let hold = p.query(Query::FxPageHold, u32::from(page) - 1, 0);
            if hold == 0 || ui!(p, u, keys & !u.shift_keys != 0) {
                mask |= bit(effect);
            }
        }
    }
    let mut held_flex = 0u8;
    let mut selected = if mask & flex_bit != 0 {
        ui!(p, u, i32::from(u.flex_lock))
    } else {
        -1
    };
    if ui!(p, u, u.fx_menu != 0 && u.fx_page == 0) {
        for key in 0..27u32 {
            if ui!(p, u, held(keys & !u.shift_keys & !u.lock_keys, key)) {
                mask |= fxbit(p, key);
                let flex = p.query(Query::KeyFlex, key, 0) as i32;
                if flex >= 0 && held_flex == 0 {
                    let pattern = p.query(Query::FlexPattern, key, 0);
                    ui!(p, u, u.flex_last = pattern as u8);
                    selected = pattern as i32;
                    held_flex = 1;
                }
            }
        }
    }
    if selected >= 0 && mask & flex_bit != 0 && ui!(p, u, i32::from(u.flex_cur) != selected) {
        ui!(p, u, u.flex_cur = selected as u8);
        p.action(Action::FlexSelect, selected as u32, 0);
    }
    let changed = p.with_cache(|_, previous| {
        let changed = *previous != held_flex;
        *previous = held_flex;
        changed
    });
    if changed {
        p.action(Action::FlexHold, u32::from(held_flex), 0);
    }
    ui!(p, u, u.af_user = u8::from(mask != 0));
    if ui!(p, u, u.af_live != 0 && u.af_user == 0) {
        let effect = ui!(p, u, u.af_fx);
        mask |= bit(u32::from(effect));
    }
    if ui!(p, u, u.dj_on != 0) && q(p, Query::Playing) != 0 {
        if ui!(p, u, u.dj_fill != 0) {
            mask |= bit(q(p, Query::FillEffect));
        }
        if q(p, Query::Section) == 2 {
            mask |= bit(q(p, Query::RiserEffect));
        }
    }
    if ui!(p, u, u.smp_dry != 0) {
        mask = 0;
    }
    if ui!(p, u, u.fx_mask != mask) {
        ui!(p, u, u.fx_mask = mask);
        p.action(Action::Effects, mask as u32, (mask >> 32) as u32);
    }
}

fn play_keys(p: &mut impl Port, keys: u32, changes: u32) {
    if ui!(p, u, u.fx_menu != 0 && u.fx_page == 32) {
        let down = ui!(p, u, changes & keys & !u.shift_keys);
        p.action(Action::SampleKeys, down, changes & !keys);
    }
    if ui!(p, u, u.fx_menu != 0 && u.fx_page == 1) {
        for key in 0..27u32 {
            if ui!(p, u, held(changes & keys & !u.shift_keys, key)) {
                for (step, &white) in WHITE.iter().enumerate() {
                    if u32::from(white) == key {
                        p.action(Action::GateStep, step as u32, 0);
                    }
                }
            }
        }
    }
    if ui!(p, u, u.fx_menu == 0) {
        for key in 0..27u32 {
            if ui!(p, u, held(changes & keys & !u.shift_keys, key)) {
                let view = ui!(p, u, u.view);
                match view {
                    page::SEQUENCER => {
                        for (step, &white) in WHITE.iter().enumerate() {
                            if u32::from(white) == key {
                                p.action(Action::StepToggle, step as u32, 0);
                            }
                        }
                        for (step, &black) in BLACK.iter().take(7).enumerate() {
                            if u32::from(black) == key {
                                let lane = ui!(p, u, u.lane);
                                let parameter = ParamId::LSRC.index() as u32 + u32::from(lane);
                                let source = if step < 6 { (step + 1) as u32 } else { 0 };
                                p.action(Action::Knob, parameter, source);
                                p.notice(Notice::LanePattern, u32::from(lane), step as u32);
                            }
                        }
                    }
                    15 | 33 | 5 | 10 | 19 | 25 => {
                        let count = if view == 5 { 8 } else { 16 };
                        for (step, &white) in WHITE.iter().take(count).enumerate() {
                            if u32::from(white) == key {
                                ui!(
                                    p,
                                    u,
                                    match view {
                                        15 => u.astp = step as u8,
                                        33 => u.slsl = step as u8,
                                        5 => u.pgsl = step as u8,
                                        10 => u.bstp = step as u8,
                                        19 => u.pstp = step as u8,
                                        _ => u.stp = step as u8,
                                    }
                                );
                            }
                        }
                        if view == 5 {
                            let fills = q(p, Query::ProgressionFillCount) as usize;
                            for (fill, &black) in BLACK.iter().take(fills).enumerate() {
                                if u32::from(black) == key {
                                    p.action(Action::ProgressionFill, fill as u32, 0);
                                }
                            }
                            for voice in 0..2u32 {
                                if u32::from(WHITE[8 + voice as usize]) == key {
                                    let flags = (p.query(Query::GlobalReserved, 2, 0)
                                        ^ (1u32 << voice))
                                        & 7;
                                    p.action(Action::GlobalReserved, 2, flags);
                                    p.action(Action::Pedal, flags & 3, 0);
                                    act(p, Action::Globals);
                                    p.notice(Notice::Pedal, voice, (flags >> voice) & 1);
                                }
                            }
                        }
                    }
                    page::SONGBOOK => {
                        if key == u32::from(WHITE[0]) {
                            let hold = jam(p, Jam::Hold);
                            set_jam(p, Jam::Hold, i32::from(hold == 0));
                        }
                        if key == u32::from(WHITE[1]) {
                            set_jam(p, Jam::Idea, 1);
                        }
                        if key == u32::from(WHITE[2]) {
                            let bass = jam(p, Jam::Bass);
                            set_jam(p, Jam::Bass, bass + 1);
                        }
                        if key == u32::from(WHITE[3]) {
                            act(p, Action::Fill);
                        }
                        if key == u32::from(WHITE[7]) {
                            set_jam(p, Jam::On, 0);
                            act(p, Action::ProjectApply);
                            p.action(Action::View, u32::from(page::PERFORM), 0);
                        }
                    }
                    JAM_PAGE => {
                        p.action(Action::JamDown, key, 0);
                    }
                    _ => {
                        if let Some(voice) = controls::voice_of(i32::from(view)) {
                            if parameter(p, HAND[voice as usize]) != 0 {
                                ui!(p, u, u.key_voice[key as usize] = voice + 1);
                                note(p, voice, 53 + key as u8, true);
                            } else {
                                p.action(Action::ToggleNote, (5 + key) % 12, 0);
                            }
                        } else {
                            p.action(Action::ToggleNote, (5 + key) % 12, 0);
                        }
                    }
                }
            }
        }
    }
    for key in 0..27u32 {
        if ui!(p, u, u.jam_n[key as usize] != 0) && !held(keys, key) {
            p.action(Action::JamUp, key, 0);
        }
    }
    for key in 0..27u32 {
        let voice = ui!(p, u, u.key_voice[key as usize]);
        if voice != 0 && !held(keys, key) {
            note(p, voice - 1, 53 + key as u8, false);
            ui!(p, u, u.key_voice[key as usize] = 0);
        }
    }
}

fn turn(p: &mut impl Port, role: u32, knob: u32, delta: i32) {
    p.action(Action::Turn, role | (knob << 8), delta as u32);
}
fn detail(view: u8) -> bool {
    (39..=42).contains(&view)
}
fn encoder_select(p: &mut impl Port, buttons: u32, delta: i32) {
    let (menu, page) = ui!(p, u, (u.fx_menu, u.fx_page));
    if !held(buttons, HOME) && menu != 0 && page == 0 {
        let old = q(p, Query::FxBank);
        let bank = if delta > 0 {
            (old + 1).min(3)
        } else {
            old.saturating_sub(1)
        };
        p.action(Action::FxBank, bank, 0);
        p.notice(Notice::FxBank, bank, 0);
    } else if !held(buttons, HOME) && menu != 0 && page != 0 {
        let setting = (p.query(Query::SettingPosition, u32::from(page), 0) as i32) >= 0;
        let position = (p.query(Query::FxPosition, u32::from(page), 0) as i32)
            + if delta > 0 { 1 } else { -1 };
        let count = q(
            p,
            if setting {
                Query::SettingCount
            } else {
                Query::FxCount
            },
        ) as i32;
        if position >= 0 && position < count {
            let page = p.query(
                if setting {
                    Query::SettingOrder
                } else {
                    Query::FxOrder
                },
                position as u32,
                0,
            ) as u8;
            ui!(p, u, u.fx_page = page);
            p.notice(Notice::FxPage, u32::from(page), u32::from(setting));
        }
    } else if held(buttons, HOME) {
        turn(p, 0, ParamId::TEMPO.index() as u32, delta);
        ui!(p, u, u.btn_used |= 1u32 << HOME);
        if ui!(p, u, u.view == page::TEMPO) {
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.touched = 0;
                u.touch_until = now.wrapping_add(1200);
            });
        } else {
            say(p, Notice::Tempo);
        }
    } else if ui!(p, u, u.view == page::VISUAL) {
        let visual = ui!(p, u, {
            u.viz = ((u32::from(u.viz) + if delta > 0 { 1 } else { 9 }) % 10) as u8;
            u.set_lazy = 1;
            u.viz
        });
        p.notice(Notice::Visual, u32::from(visual), 0);
    } else {
        let current = ui!(p, u, u.view);
        let base = if current == page::SEQUENCER {
            20
        } else if current == page::PERFORM {
            0
        } else {
            i32::from(current)
        };
        let next = base + if delta > 0 { 1 } else { -1 };
        if current == JAM_PAGE && delta > 0 {
            p.action(Action::View, u32::from(page::SONGBOOK), 0);
            set_jam(p, Jam::On, 1);
        } else if (page::SONGBOOK..=page::SWELLS).contains(&current) {
            let next = i32::from(current) + if delta > 0 { 1 } else { -1 };
            p.action(
                Action::View,
                if next > 42 {
                    0
                } else if next < 38 {
                    34
                } else {
                    next as u32
                },
                0,
            );
        } else if (0..35).contains(&next) {
            p.action(Action::View, next as u32, 0);
        }
    }
    let now = q(p, Query::Now);
    ui!(p, u, u.turn_t = now);
}
fn encoder_algorithm(p: &mut impl Port, buttons: u32, delta: i32) -> bool {
    let (view, menu) = ui!(p, u, (u.view, u.fx_menu));
    let mut valid = true;
    if (page::SONGBOOK..=page::SWELLS).contains(&view) && menu == 0 {
        let current = jam(p, Jam::Energy);
        if let Some(value) = delta
            .checked_mul(5)
            .and_then(|change| current.checked_add(change))
        {
            set_jam(p, Jam::Energy, value);
        } else {
            valid = false;
        }
    } else if (view == page::PERFORM || view == page::VISUAL) && menu == 0 && !held(buttons, SEL) {
        let old = ui!(p, u, i32::from(u.mood));
        if let Some(value) = delta
            .checked_mul(2)
            .and_then(|change| old.checked_add(change))
        {
            let mood = value.clamp(0, 100) as u8;
            ui!(p, u, u.mood = mood);
            act(p, Action::Mood);
            p.notice(Notice::Mood, u32::from(mood), 0);
        } else {
            valid = false;
        }
    } else if held(buttons, SEL) {
        ui!(p, u, u.btn_used |= 1u32 << SEL);
        p.action(Action::ShiftSound, if delta > 0 { 1 } else { u32::MAX }, 0);
    } else {
        let value = parameter(p, ParamId::MODE) + if delta > 0 { 1 } else { -1 };
        p.action(Action::Mode, value as u32, 0);
    }
    let now = q(p, Query::Now);
    ui!(p, u, u.turn_t = now);
    valid
}
fn encoder_preset(p: &mut impl Port, buttons: u32, delta: i32) {
    if ui!(p, u, u.view == page::SONGBOOK && u.fx_menu == 0) {
        if held(buttons, GLOBAL) {
            ui!(p, u, u.btn_used |= 1u32 << GLOBAL);
            let form = jam(p, Jam::Form);
            set_jam(p, Jam::Form, form + if delta > 0 { 1 } else { -1 });
        } else {
            let song = jam(p, Jam::Song);
            set_jam(p, Jam::Song, (song + if delta > 0 { 1 } else { 6 }) % 7);
        }
    } else if held(buttons, GLOBAL) && !held(buttons, SEL) {
        ui!(p, u, u.btn_used |= 1u32 << GLOBAL);
        p.action(Action::Genre, if delta > 0 { 1 } else { u32::MAX }, 0);
    } else if q(p, Query::SetlistPlay) != 0 {
        let queued = held(buttons, SEL) && q(p, Query::Playing) != 0;
        let from = ui!(
            p,
            u,
            if queued && u.sl_q != 0 {
                u.sl_q - 1
            } else {
                u.sl_pos
            }
        );
        let slot = p.query(
            Query::SetlistStep,
            u32::from(from),
            if delta > 0 { 1 } else { u32::MAX },
        ) as i32;
        ui!(p, u, {
            if held(buttons, SEL) {
                u.btn_used |= 1u32 << SEL;
            }
            u.piece_q = 0;
        });
        if slot >= 0 && queued {
            ui!(p, u, u.sl_q = (slot + 1) as u8);
            p.notice(Notice::SetNext, slot as u32, 0);
            let now = q(p, Query::Now);
            ui!(p, u, u.msg_until = now.wrapping_add(60000));
        } else if slot >= 0 {
            ui!(p, u, u.sl_q = 0);
            p.action(Action::Setlist, slot as u32, 0);
        }
    } else if held(buttons, SEL) && q(p, Query::Playing) != 0 {
        let queued = ui!(p, u, u.piece_q);
        let current = q(p, Query::Piece) as i32;
        let base = if queued != 0 {
            i32::from(queued) - 1
        } else {
            current
        };
        let piece = (base + if delta > 0 { 1 } else { -1 } + 10) % 10;
        ui!(p, u, {
            u.btn_used |= 1u32 << SEL;
            u.piece_q = if piece == current {
                0
            } else {
                (piece + 1) as u8
            };
        });
        p.notice(Notice::PieceNext, piece as u32, u32::from(piece == current));
        if piece != current {
            let now = q(p, Query::Now);
            ui!(p, u, u.msg_until = now.wrapping_add(60000));
        }
    } else {
        ui!(p, u, {
            if held(buttons, SEL) {
                u.btn_used |= 1u32 << SEL;
            }
            u.piece_q = 0;
        });
        let piece = q(p, Query::Piece) as i32 + if delta > 0 { 1 } else { -1 };
        p.action(Action::Piece, piece as u32, 0);
    }
    let now = q(p, Query::Now);
    ui!(p, u, u.turn_t = now);
}

fn queued_changes(p: &mut impl Port) {
    if ui!(p, u, u.sl_q != 0 && u.store_wait == 0)
        && (q(p, Query::Playing) == 0 || q(p, Query::DrumPosition) == 15)
    {
        let slot = ui!(p, u, u.sl_q - 1);
        let from = parameter(p, ParamId::TEMPO);
        let playing = q(p, Query::Playing);
        ui!(p, u, u.sl_q = 0);
        p.action(
            Action::SetlistBar,
            u32::from(slot) | (u32::from(playing != 0) << 8),
            from as u32,
        );
    }
    if ui!(p, u, u.piece_q != 0) && (q(p, Query::Playing) == 0 || q(p, Query::DrumPosition) == 15) {
        let piece = ui!(p, u, u.piece_q - 1);
        let from = parameter(p, ParamId::TEMPO);
        let playing = q(p, Query::Playing);
        ui!(p, u, u.piece_q = 0);
        p.action(
            Action::PieceBar,
            u32::from(piece) | (u32::from(playing != 0) << 8),
            from as u32,
        );
    }
}
fn held_actions(p: &mut impl Port, buttons: u32) {
    if held(buttons, SEL)
        && ui!(
            p,
            u,
            !held(u.btn_used, SEL) && u.shift_hint == 0 && !(u.view == JAM_PAGE && u.fx_menu == 0)
        )
    {
        let now = q(p, Query::Now);
        if ui!(p, u, now.wrapping_sub(u.down_t[SEL as usize]) >= 400) {
            ui!(p, u, u.shift_hint = 1);
        }
    }
    if ui!(
        p,
        u,
        u.save_global != 0 && u.save_fact != 0 && u.factory_done == 0
    ) && held(buttons, SAVE)
    {
        let now = q(p, Query::Now);
        let elapsed = ui!(p, u, now.wrapping_sub(u.down_t[SAVE as usize]));
        if elapsed >= 1500 {
            ui!(p, u, u.factory_done = 1);
            act(p, Action::PieceFactory);
        } else if elapsed >= 400 {
            let now = q(p, Query::Now);
            if ui!(p, u, now >= u.msg_until) {
                say(p, Notice::Factory);
                ui!(
                    p,
                    u,
                    u.msg_until = u.down_t[SAVE as usize].wrapping_add(1500)
                );
            }
        }
    }
    if held(buttons, SAVE) && ui!(p, u, u.save_held == 0) {
        let now = q(p, Query::Now);
        if ui!(p, u, now.wrapping_sub(u.down_t[SAVE as usize]) >= 500) {
            ui!(p, u, {
                u.save_held = 1;
                u.btn_used |= 1u32 << SAVE;
            });
            if q(p, Query::Section) != 0 {
                p.action(Action::Section, 2, 0);
                say(p, Notice::Drop);
            } else {
                p.action(Action::Button, SAVE, 0);
            }
        }
    }
}
fn knob_encoders(p: &mut impl Port, buttons: u32) -> u32 {
    const FIELDS: [[Jam; 4]; 4] = [
        [Jam::Form, Jam::Bars, Jam::Variation, Jam::Swell],
        [Jam::KeysProgram, Jam::Touch, Jam::Brightness, Jam::Release],
        [
            Jam::BassProgram,
            Jam::BassAttack,
            Jam::BassGate,
            Jam::Energy,
        ],
        [Jam::SwellVoice, Jam::Swell, Jam::Variation, Jam::Move],
    ];
    let mut rejected = 0;
    for knob in 0..4u32 {
        let role = 3 + knob;
        let delta = p.query(Query::Encoder, role, 0) as i32;
        if delta == 0 {
            continue;
        }
        let (view, menu) = ui!(p, u, (u.view, u.fx_menu));
        if detail(view) && menu == 0 {
            let field = FIELDS[(view - 39) as usize][knob as usize];
            let current = jam(p, field);
            let value = if field == Jam::Bars {
                Some(if delta > 0 { 16 } else { 8 })
            } else {
                current.checked_add(delta)
            };
            if let Some(value) = value {
                set_jam(p, field, value);
            } else {
                rejected += 1;
            }
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.act_t = now;
                u.turn_t = now;
            });
            continue;
        }
        if view == page::SONGBOOK && menu == 0 {
            match knob {
                0 => {
                    let key = jam(p, Jam::Key);
                    if let Some(value) = key.checked_add(delta) {
                        set_jam(p, Jam::Key, value.rem_euclid(24));
                    } else {
                        rejected += 1;
                    }
                }
                1 => {
                    let tempo = parameter(p, ParamId::TEMPO);
                    if let Some(value) = tempo.checked_add(delta) {
                        p.action(Action::Knob, ParamId::TEMPO.index() as u32, value as u32);
                    } else {
                        rejected += 1;
                    }
                }
                2 => set_jam(p, Jam::Bars, if delta > 0 { 16 } else { 8 }),
                _ => {
                    let movement = jam(p, Jam::Move);
                    set_jam(p, Jam::Move, movement + if delta > 0 { 1 } else { -1 });
                }
            }
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.act_t = now;
                u.turn_t = now;
            });
            continue;
        }
        if view == page::SEQUENCER && held(buttons, SEL) {
            ui!(p, u, u.btn_used |= 1u32 << SEL);
            p.action(Action::SequenceTrick, knob, delta as u32);
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.act_t = now;
                u.turn_t = now;
            });
            continue;
        }
        if view == page::TEMPO && knob == 3 && held(buttons, SEL) {
            let default = if q(p, Query::GenreBars) != 0 { 1 } else { 2 };
            let next = if delta > 0 { 1 } else { 2 };
            ui!(p, u, u.btn_used |= 1u32 << SEL);
            p.action(
                Action::ProgressionUnit,
                if next == default { 0 } else { next },
                0,
            );
            p.notice(Notice::Drift, next, 0);
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.act_t = now;
                u.turn_t = now;
            });
            continue;
        }
        if view == 13 && knob == 3 && held(buttons, SEL) {
            ui!(p, u, u.btn_used |= 1u32 << SEL);
            turn(p, role, ParamId::AC_CHG.index() as u32, delta);
            say(p, Notice::Mutates);
            let now = q(p, Query::Now);
            ui!(p, u, {
                u.act_t = now;
                u.turn_t = now;
            });
            continue;
        }
        let parameter = p.query(Query::ViewKnob, knob, 0);
        turn(p, role, parameter, delta);
        ui!(p, u, u.touched = knob as i8);
        let now = q(p, Query::Now);
        ui!(p, u, u.touch_until = now.wrapping_add(1200));
        let now = q(p, Query::Now);
        ui!(p, u, {
            u.act_t = now;
            u.turn_t = now;
        });
    }
    rejected
}

/// Number of malformed arithmetic packets refused; all valid inputs preserve
/// legacy order. Each of the two caller-owned cache cells replaces its old C static.
pub fn poll(p: &mut impl Port) -> u32 {
    let buttons = q(p, Query::Buttons);
    let keys = q(p, Query::Keys);
    if q(p, Query::RunnerOn) != 0 {
        p.action(Action::RunnerInput, buttons, keys);
        return 0;
    }
    if ui!(p, u, u.saver_on != 0) {
        let changed = ui!(p, u, {
            let down = buttons & !u.btn;
            let notes = keys & !u.keys;
            u.btn_used |= down;
            u.shift_keys |= notes;
            u.btn = buttons;
            u.keys = keys;
            down | notes
        });
        for role in 0..7u32 {
            if p.query(Query::Encoder, role, 0) != 0 {
                let now = q(p, Query::Now);
                ui!(p, u, u.turn_t = now);
            }
        }
        if changed != 0 {
            let now = q(p, Query::Now);
            ui!(p, u, u.act_t = now);
        }
        return 0;
    }
    if button_edges(p, buttons, keys) {
        return 0;
    }
    let changes = key_edges(p, buttons, keys);
    effects(p, keys, changes);
    play_keys(p, keys, changes);
    if buttons != 0 || keys != 0 || changes != 0 {
        let now = q(p, Query::Now);
        ui!(p, u, u.act_t = now);
    }
    act(p, Action::Midi);
    act(p, Action::ControlOut);
    if held(buttons, SEQ) && ui!(p, u, u.seq_held == 0 && u.view != page::SEQUENCER) {
        let now = q(p, Query::Now);
        if ui!(p, u, now.wrapping_sub(u.down_t[SEQ as usize]) >= 400) {
            ui!(p, u, {
                u.seq_held = 1;
                u.btn_used |= 1u32 << SEQ;
            });
            p.action(Action::View, u32::from(page::SEQUENCER), 0);
        }
    }
    let delta = p.query(Query::Encoder, 0, 0) as i32;
    if delta != 0 {
        encoder_select(p, buttons, delta);
    }
    let mut rejected = 0;
    let delta = p.query(Query::Encoder, 1, 0) as i32;
    if delta != 0 && !encoder_algorithm(p, buttons, delta) {
        rejected += 1;
    }
    let delta = p.query(Query::Encoder, 2, 0) as i32;
    if delta != 0 {
        encoder_preset(p, buttons, delta);
    }
    queued_changes(p);
    held_actions(p, buttons);
    rejected += knob_encoders(p, buttons);
    p.action(Action::Help, buttons, keys);
    rejected
}
