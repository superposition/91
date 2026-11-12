// SPDX-License-Identifier: GPL-3.0-only
//! Button release/transport/page policy. Effects always follow scoped UI access.
use crate::ui::State;

pub mod id {
    pub const FX: u32 = 0;
    pub const EDIT: u32 = 4;
    pub const HOME: u32 = 6;
    pub const SAVE: u32 = 7;
    pub const PLAY: u32 = 10;
    pub const REC: u32 = 11;
    pub const OCT_DOWN: u32 = 12;
    pub const OCT_UP: u32 = 13;
}
pub mod page {
    pub const TEMPO: u8 = 0;
    pub const SEQUENCER: u8 = 35;
    pub const PERFORM: u8 = 36;
    pub const VISUAL: u8 = 37;
    pub const SONGBOOK: u8 = 38;
    pub const SWELLS: u8 = 42;
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Group {
    pub button: u8,
    pub first: u8,
    pub count: u8,
}
const _: [(); 18] = [(); core::mem::size_of::<[Group; 6]>()];
pub const GROUPS: [Group; 6] = [
    Group {
        button: 6,
        first: 0,
        count: 1,
    },
    Group {
        button: 2,
        first: 1,
        count: 5,
    },
    Group {
        button: 3,
        first: 6,
        count: 5,
    },
    Group {
        button: 8,
        first: 11,
        count: 9,
    },
    Group {
        button: 9,
        first: 20,
        count: 8,
    },
    Group {
        button: 5,
        first: 32,
        count: 2,
    },
];
#[repr(u32)]
pub enum Query {
    Now,
    Playing,
    JamOn,
    Frozen,
    Section,
    SectionRequested,
    JamHold,
    OctaveParameter,
    Parameter,
}
#[repr(u32)]
pub enum Action {
    SaveSession,
    JamHold,
    JamIdea,
    View,
    Play,
    Seed,
    NextChord,
    Freeze,
    SceneReset,
    Knob,
    Section,
}
#[repr(u32)]
pub enum Notice {
    SessionSaved,
    StopToSave,
    Visual,
    FxSettings,
    Fx,
    NextChord,
    Stopped,
    Playing,
    Freeze,
    Octave,
    BreakdownOff,
    Breakdown,
    Build,
    Drop,
}
pub trait Port {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R;
    fn query(&mut self, query: Query, arg: u32) -> u32;
    fn action(&mut self, action: Action, first: u32, second: u32) -> u32;
    fn notice(&mut self, notice: Notice, first: u32, second: u32);
}
fn query(port: &mut impl Port, q: Query) -> u32 {
    port.query(q, 0)
}
fn effect(port: &mut impl Port, a: Action) {
    port.action(a, 0, 0);
}
fn notice(port: &mut impl Port, n: Notice) {
    port.notice(n, 0, 0);
}
fn view(port: &mut impl Port, v: u8) {
    port.action(Action::View, u32::from(v), 0);
}

/// Unknown physical IDs are refused before any state change.
pub fn button(port: &mut impl Port, button: u32) -> bool {
    if button >= 14 {
        return false;
    }
    if port.with_ui(|ui| ui.view >= page::SONGBOOK && ui.view <= page::SWELLS && ui.fx_menu == 0)
        && button == id::SAVE
    {
        let saved = port.action(Action::SaveSession, 0, 0) == 0;
        notice(
            port,
            if saved {
                Notice::SessionSaved
            } else {
                Notice::StopToSave
            },
        );
        return true;
    }
    if port.with_ui(|ui| ui.view == page::SONGBOOK && ui.fx_menu == 0)
        && (button == id::REC || button == id::EDIT)
    {
        if button == id::REC {
            let held = query(port, Query::JamHold);
            port.action(Action::JamHold, u32::from(held == 0), 0);
        } else {
            effect(port, Action::JamIdea);
        }
        return true;
    }
    if button == id::HOME && query(port, Query::JamOn) != 0 {
        view(port, page::SONGBOOK);
        return true;
    }
    if button == id::HOME {
        let now = query(port, Query::Now);
        port.with_ui(|ui| {
            if ui.fx_menu != 0 {
                ui.fx_menu = 0;
                ui.fx_page = 0;
                ui.fx_open_t = 0;
            }
        });
        if port.with_ui(|ui| ui.view == page::VISUAL) {
            view(port, page::PERFORM);
            port.with_ui(|ui| ui.home_t = 0);
            return true;
        }
        if port.with_ui(|ui| ui.home_t != 0 && now.wrapping_sub(ui.home_t) < 450) {
            port.with_ui(|ui| ui.home_t = 0);
            view(port, page::VISUAL);
            let visual = port.with_ui(|ui| ui.viz % 10);
            port.notice(Notice::Visual, u32::from(visual), 0);
            return true;
        }
        port.with_ui(|ui| ui.home_t = if now == 0 { 1 } else { now });
        let next = port.with_ui(|ui| {
            if ui.view == page::TEMPO {
                page::PERFORM
            } else {
                page::TEMPO
            }
        });
        view(port, next);
        return true;
    }
    for (index, group) in GROUPS.iter().enumerate() {
        if u32::from(group.button) != button {
            continue;
        }
        let next = port.with_ui(|ui| {
            let current = ui.view;
            if ui.fx_menu != 0 {
                ui.fx_menu = 0;
                ui.fx_page = 0;
                ui.fx_open_t = 0;
            }
            let in_group = GROUPS
                .iter()
                .position(|g| current >= g.first && current < g.first + g.count)
                == Some(index);
            if in_group && group.count > 1 {
                group.first + (current - group.first + 1) % group.count
            } else if ui.grp_last[index] >= group.first
                && ui.grp_last[index] < group.first + group.count
            {
                ui.grp_last[index]
            } else {
                group.first
            }
        });
        view(port, next);
        return true;
    }
    match button {
        id::FX => {
            let now = query(port, Query::Now);
            let edit = port.with_ui(|ui| {
                ui.fx_menu != 0
                    && ui.fx_page == 0
                    && ui.fx_open_t != 0
                    && now.wrapping_sub(ui.fx_open_t) < 350
            });
            if edit {
                let page = port.with_ui(|ui| {
                    ui.fx_open_t = 0;
                    ui.fx_page = if ui.fx_lastpg > 0 && ui.fx_lastpg <= 35 {
                        ui.fx_lastpg
                    } else {
                        1
                    };
                    ui.fx_page
                });
                port.notice(Notice::FxSettings, u32::from(page), 0);
            } else {
                let mode = port.with_ui(|ui| {
                    ui.fx_menu = u8::from(ui.fx_menu == 0);
                    ui.fx_page = 0;
                    ui.fx_open_t = if ui.fx_menu != 0 {
                        if now == 0 {
                            1
                        } else {
                            now
                        }
                    } else {
                        0
                    };
                    if ui.fx_menu != 0 {
                        1u32
                    } else if ui.fx_lock != 0 {
                        2u32
                    } else {
                        0u32
                    }
                });
                port.notice(Notice::Fx, mode, 0);
            }
        }
        id::EDIT => {
            effect(port, Action::NextChord);
            let playing = query(port, Query::Playing);
            port.notice(Notice::NextChord, playing, 0);
        }
        id::PLAY => {
            if query(port, Query::Playing) != 0 {
                port.action(Action::Play, 0, 0);
                notice(port, Notice::Stopped);
            } else {
                let now = query(port, Query::Now);
                port.action(Action::Seed, now, 0);
                port.action(Action::Play, 1, 0);
                notice(port, Notice::Playing);
            }
        }
        id::REC => {
            let frozen = query(port, Query::Frozen);
            port.notice(Notice::Freeze, frozen, 0);
            let frozen = query(port, Query::Frozen);
            port.action(Action::Freeze, u32::from(frozen == 0), 0);
        }
        id::OCT_DOWN | id::OCT_UP => {
            if port.with_ui(|ui| ui.fx_menu != 0 && ui.fx_page == 0) && button == id::OCT_DOWN {
                effect(port, Action::SceneReset);
            } else if port.with_ui(|ui| ui.view == page::SEQUENCER) {
                port.with_ui(|ui| {
                    ui.lane = ((u32::from(ui.lane)
                        + if button == id::OCT_UP { 1u32 } else { 5u32 })
                        % 6) as u8;
                    ui.touched = -1;
                });
            } else {
                let current = port.with_ui(|ui| ui.view);
                let parameter = port.query(Query::OctaveParameter, u32::from(current));
                let value = port.query(Query::Parameter, parameter) as i32;
                port.action(
                    Action::Knob,
                    parameter,
                    value.wrapping_add(if button == id::OCT_UP { 1 } else { -1 }) as u32,
                );
                let value = port.query(Query::Parameter, parameter);
                port.notice(Notice::Octave, parameter, value);
            }
        }
        id::SAVE => {
            let section = query(port, Query::Section);
            if section == 0 && query(port, Query::SectionRequested) == 1 {
                port.action(Action::Section, 2, 0);
                notice(port, Notice::BreakdownOff);
            } else if query(port, Query::Section) == 0 {
                port.action(Action::Section, 0, 0);
                let playing = query(port, Query::Playing);
                port.notice(Notice::Breakdown, playing, 0);
            } else if query(port, Query::Section) == 1 {
                port.action(Action::Section, 1, 0);
                notice(port, Notice::Build);
            } else {
                port.action(Action::Section, 2, 0);
                notice(port, Notice::Drop);
            }
        }
        _ => {}
    }
    true
}
