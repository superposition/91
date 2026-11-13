// SPDX-License-Identifier: GPL-3.0-only
//! Foreground scenes, undo, tempo glide and page transitions over scoped loans.
use crate::{buttons::GROUPS, ui::State};
use fm1_types::ParamId;

#[repr(u32)]
pub enum Query {
    Now,
    Playing,
    JamOn,
    Section,
    DrumPosition,
    Parameter,
    MixSetting,
    FxFirst,
    SectionRequested,
    PatternNow,
    HostMessage,
    Bars,
    SectionBars,
    IntroFraction,
    DriftBars,
    BuildLength,
    SetlistPlay,
    SetlistStep,
    Piece,
    Genre,
    FxPages,
    FxValue,
    FxRange,
    SetlistEntry,
    SetlistOn,
    ProgressionScale,
    BassStep,
    PluckRung,
    PluckLevel,
    ModStep,
    CurrentParameter,
    JamSettings,
}
#[repr(u32)]
pub enum Action {
    Knob,
    Tracks,
    EngineSet,
    ProjectSet,
    BassCapture,
    Intro,
    MidiMap,
    UsbHost,
    DjSync,
    Fill,
    Section,
}
#[repr(u32)]
pub enum Notice {
    Nothing,
    Undo,
    Tempo,
    Tap,
    Scene,
    Reset,
    Key,
    Breadcrumb,
    PatternBack,
    Host,
    AutoFx,
    MixTrack,
    MixDrums,
    AllTracks,
    DjOn,
    DjLength,
    DjPieces,
}
#[repr(u32)]
pub enum Operation {
    UndoSave,
    UndoSwap,
    Tap,
    SceneStore,
    SceneApply,
    SceneRecall,
    SceneReset,
    SceneTick,
    MixStart,
    MixTick,
    KeyTick,
    View,
    DjNext,
    DjBar,
    AutoFx,
    MixKey,
}
pub trait Port {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R;
    fn query(&mut self, q: Query, argument: u32) -> u32;
    fn action(&mut self, a: Action, first: u32, second: u32);
    fn notice(&mut self, n: Notice, first: u32, second: u32);
}
macro_rules! ui {
    ($p:ident,$u:ident,$e:expr) => {
        $p.with_ui(|$u| $e)
    };
}
pub(crate) fn query(p: &mut impl Port, q: Query) -> u32 {
    p.query(q, 0)
}
pub(crate) fn parameter(p: &mut impl Port, id: ParamId) -> i32 {
    p.query(Query::Parameter, id.index() as u32) as i32
}
pub(crate) fn knob(p: &mut impl Port, id: u32, value: i32) {
    p.action(Action::Knob, id, value as u32);
}
fn engine(p: &mut impl Port, id: ParamId, value: i32) {
    p.action(Action::EngineSet, id.index() as u32, value as u32);
}
pub(crate) fn tempo(p: &mut impl Port, value: i32) {
    p.action(
        Action::ProjectSet,
        ParamId::TEMPO.index() as u32,
        value as u32,
    );
    engine(p, ParamId::TEMPO, value);
}
pub fn undo_save(p: &mut impl Port) {
    ui!(p, u, {
        u.undo_lock = u.fx_lock;
        u.undo_m = u.trk_m;
        u.undo_s = u.trk_s;
        u.undo_k = u.fxk;
        u.undo_ok = 1;
    });
}
pub(crate) fn undo_swap(p: &mut impl Port) {
    if ui!(p, u, u.undo_ok == 0) {
        p.notice(Notice::Nothing, 0, 0);
        return;
    }
    let previous = ui!(p, u, (u.fx_lock, u.trk_m, u.trk_s, u.fxk));
    let tracks = ui!(p, u, {
        u.fx_lock = u.undo_lock;
        u.trk_m = u.undo_m;
        u.trk_s = u.undo_s;
        (u.trk_m, u.trk_s)
    });
    p.action(Action::Tracks, u32::from(tracks.0), u32::from(tracks.1));
    for index in 0..4 {
        let first = query(p, Query::FxFirst);
        let value = ui!(p, u, i32::from(u.undo_k[index]));
        knob(p, first + index as u32, value);
    }
    ui!(p, u, {
        u.undo_lock = previous.0;
        u.undo_m = previous.1;
        u.undo_s = previous.2;
        u.undo_k = previous.3;
    });
    p.notice(Notice::Undo, 0, 0);
}
pub(crate) fn tap(p: &mut impl Port) -> bool {
    let now = query(p, Query::Now);
    let result = ui!(p, u, {
        if u.tap_n != 0
            && now.wrapping_sub(u.tap_t[usize::from(u.tap_n.wrapping_sub(1) & 3)]) > 2000
        {
            u.tap_n = 0;
        }
        u.tap_t[usize::from(u.tap_n & 3)] = now;
        u.tap_n = u.tap_n.wrapping_add(1);
        if u.tap_n >= 2 {
            let count = u32::from(u.tap_n.min(4));
            let last = u.tap_t[usize::from(u.tap_n.wrapping_sub(1) & 3)];
            let first = u.tap_t[usize::from(u.tap_n.wrapping_sub(count as u8) & 3)];
            let elapsed = last.wrapping_sub(first);
            Some((count, elapsed))
        } else {
            None
        }
    });
    if let Some((count, elapsed)) = result {
        // Simultaneous synthetic taps had an undefined divide-by-zero in C.
        // Keep the tap history, but refuse that tempo update.
        if elapsed == 0 {
            return false;
        }
        let bpm = (60000u32.wrapping_mul(count - 1).wrapping_add(elapsed / 2)) / elapsed;
        knob(p, ParamId::TEMPO.index() as u32, bpm as i32);
        p.notice(Notice::Tempo, 0, 0);
    } else {
        p.notice(Notice::Tap, 0, 0);
    }
    true
}
fn scene_store(p: &mut impl Port, scene: usize) {
    ui!(p, u, {
        u.scene_lock[scene] = u.fx_lock;
        u.scene_m[scene] = u.trk_m;
        u.scene_s[scene] = u.trk_s;
        u.scene_k[scene] = u.fxk;
    });
    for lane in 0..6usize {
        let density = p.query(Query::Parameter, ParamId::DENS.index() as u32 + lane as u32);
        ui!(p, u, u.scene_g[scene][lane] = density as u8);
        let source = p.query(Query::Parameter, ParamId::LSRC.index() as u32 + lane as u32);
        ui!(p, u, u.scene_g[scene][6 + lane] = source as u8);
    }
    let pattern = parameter(p, ParamId::PATTERN);
    ui!(p, u, {
        u.scene_g[scene][12] = pattern as u8;
        u.scene_set |= 1u8 << scene;
    });
    p.notice(Notice::Scene, scene as u32, 1);
}
pub(crate) fn scene_apply(p: &mut impl Port, scene: usize) {
    undo_save(p);
    let tracks = ui!(p, u, {
        u.fx_lock = u.scene_lock[scene];
        u.trk_m = u.scene_m[scene];
        u.trk_s = u.scene_s[scene];
        (u.trk_m, u.trk_s)
    });
    p.action(Action::Tracks, u32::from(tracks.0), u32::from(tracks.1));
    for lane in 0..6usize {
        let density = ui!(p, u, i32::from(u.scene_g[scene][lane]));
        knob(p, ParamId::DENS.index() as u32 + lane as u32, density);
        let source = ui!(p, u, i32::from(u.scene_g[scene][6 + lane]));
        knob(p, ParamId::LSRC.index() as u32 + lane as u32, source);
    }
    let pattern = ui!(p, u, i32::from(u.scene_g[scene][12]));
    knob(p, ParamId::PATTERN.index() as u32, pattern);
    ui!(p, u, {
        for index in 0..4 {
            u.glide_t[index] = u.scene_k[scene][index];
            u.glide_f[index] = f32::from(u.fxk[index]);
        }
        u.glide_on = 15;
    });
    p.notice(Notice::Scene, scene as u32, 0);
}
fn scene_recall(p: &mut impl Port, scene: usize) {
    if ui!(p, u, u.scene_set & (1u8 << scene) == 0) {
        p.notice(Notice::Scene, scene as u32, 2);
        return;
    }
    if ui!(p, u, u.scene_quant != 0) && query(p, Query::Playing) != 0 {
        ui!(p, u, u.scene_q = (scene + 1) as u8);
        p.notice(Notice::Scene, scene as u32, 3);
    } else {
        ui!(p, u, u.scene_q = 0);
        scene_apply(p, scene);
    }
}
pub(crate) fn scene_reset(p: &mut impl Port) {
    undo_save(p);
    ui!(p, u, {
        u.fx_lock = 0;
        u.trk_m = 0;
        u.trk_s = 0;
    });
    p.action(Action::Tracks, 0, 0);
    for index in 0..4 {
        let first = query(p, Query::FxFirst);
        knob(p, first + index, 0);
    }
    p.notice(Notice::Reset, 0, 0);
}
fn scene_tick(p: &mut impl Port) {
    for scene in 0..4usize {
        if ui!(p, u, u.scene_pend & (1u8 << scene) != 0) {
            if ui!(p, u, u.keys & (1u32 << (19 + scene)) == 0) {
                ui!(p, u, u.scene_pend &= !(1u8 << scene));
                scene_recall(p, scene);
            } else {
                let now = query(p, Query::Now);
                if ui!(p, u, now.wrapping_sub(u.scene_t[scene]) >= 600) {
                    ui!(p, u, u.scene_pend &= !(1u8 << scene));
                    scene_store(p, scene);
                }
            }
        }
    }
}
fn mix_start(p: &mut impl Port, from: i32) {
    const BARS: [u8; 5] = [0, 4, 8, 16, 32];
    let setting = query(p, Query::MixSetting);
    let bars = BARS[(setting % 5) as usize];
    ui!(p, u, u.mx_bars = 0);
    if bars == 0 || from == parameter(p, ParamId::TEMPO) {
        return;
    }
    ui!(p, u, u.mx_from = from as i16);
    let to = parameter(p, ParamId::TEMPO);
    ui!(p, u, {
        u.mx_to = to as i16;
        u.mx_bars = bars;
        u.mx_n = 0;
    });
    tempo(p, from);
    ui!(p, u, u.mx_last = from as i16);
}
fn mix_tick(p: &mut impl Port) {
    if query(p, Query::JamOn) != 0 {
        ui!(p, u, u.mx_bars = 0);
        return;
    }
    if ui!(p, u, u.mx_bars == 0) {
        return;
    }
    if query(p, Query::Playing) == 0
        || parameter(p, ParamId::TEMPO) != ui!(p, u, i32::from(u.mx_last))
    {
        ui!(p, u, u.mx_bars = 0);
        return;
    }
    let position = query(p, Query::DrumPosition) as i32;
    let target = ui!(p, u, {
        let span = i32::from(u.mx_bars) * 16;
        let at = ((i32::from(u.mx_n) - 1) * 16 + position).max(0);
        let distance = (i32::from(u.mx_to) - i32::from(u.mx_from)) * at;
        if at >= span {
            u.mx_bars = 0;
            i32::from(u.mx_to)
        } else {
            i32::from(u.mx_from)
                + if distance >= 0 {
                    (distance + span / 2) / span
                } else {
                    -((-distance + span / 2) / span)
                }
        }
    });
    if target != parameter(p, ParamId::TEMPO) {
        tempo(p, target);
    }
    ui!(p, u, u.mx_last = target as i16);
}
fn key_tick(p: &mut impl Port) {
    const SHIFTS: [i32; 5] = [0, 2, 3, 5, 7];
    if query(p, Query::JamOn) != 0 {
        ui!(p, u, u.key_on = 0);
        let section = query(p, Query::Section);
        ui!(p, u, u.sec_last = section as u8);
        return;
    }
    let section = if query(p, Query::Playing) != 0 {
        query(p, Query::Section)
    } else {
        0
    };
    let shift = ui!(p, u, SHIFTS[usize::from(u.key_set % 5)]);
    if section != ui!(p, u, u32::from(u.sec_last)) {
        if section == 0
            && ui!(p, u, u.sec_last != 0)
            && shift != 0
            && query(p, Query::Playing) != 0
            && ui!(p, u, u.key_on == 0)
        {
            ui!(p, u, u.key_on = 1);
            let root = parameter(p, ParamId::ROOT);
            engine(p, ParamId::ROOT, (root + shift) % 12);
            p.notice(Notice::Key, shift as u32, 0);
        } else if section == 1 && ui!(p, u, u.key_on != 0) {
            ui!(p, u, u.key_on = 0);
            let root = parameter(p, ParamId::ROOT);
            engine(p, ParamId::ROOT, root);
        }
        ui!(p, u, u.sec_last = section as u8);
    }
    if query(p, Query::Playing) == 0 && ui!(p, u, u.key_on != 0) {
        ui!(p, u, u.key_on = 0);
        let root = parameter(p, ParamId::ROOT);
        engine(p, ParamId::ROOT, root);
    }
}
fn view(p: &mut impl Port, value: i32) {
    const TABS: [(u8, u8); 10] = [
        (0, 1),
        (1, 5),
        (6, 5),
        (11, 5),
        (16, 4),
        (20, 8),
        (28, 2),
        (30, 2),
        (32, 2),
        (34, 1),
    ];
    if value == 10 {
        p.action(Action::BassCapture, 0, 0);
    }
    for (index, group) in GROUPS.iter().enumerate() {
        if value >= i32::from(group.first)
            && value < i32::from(group.first) + i32::from(group.count)
        {
            ui!(p, u, u.grp_last[index] = value as u8);
        }
    }
    if value != ui!(p, u, i32::from(u.view)) && value < 35 {
        for (index, &(first, count)) in TABS.iter().enumerate() {
            if value >= i32::from(first) && value < i32::from(first) + i32::from(count) && count > 1
            {
                p.notice(Notice::Breadcrumb, index as u32, value as u32);
                break;
            }
        }
    }
    ui!(p, u, {
        u.view = value as u8;
        if value < 35 {
            u.last_dial = value as u8;
        }
        u.touched = -1;
    });
}
pub fn dispatch(p: &mut impl Port, operation: u32, argument: i32) -> bool {
    match operation {
        x if x == Operation::UndoSave as u32 => undo_save(p),
        x if x == Operation::UndoSwap as u32 => undo_swap(p),
        x if x == Operation::Tap as u32 => return tap(p),
        x if x == Operation::SceneStore as u32 && (0..4).contains(&argument) => {
            scene_store(p, argument as usize)
        }
        x if x == Operation::SceneApply as u32 && (0..4).contains(&argument) => {
            scene_apply(p, argument as usize)
        }
        x if x == Operation::SceneRecall as u32 && (0..4).contains(&argument) => {
            scene_recall(p, argument as usize)
        }
        x if x == Operation::SceneReset as u32 => scene_reset(p),
        x if x == Operation::SceneTick as u32 => scene_tick(p),
        x if x == Operation::MixStart as u32 => mix_start(p, argument),
        x if x == Operation::MixTick as u32 => mix_tick(p),
        x if x == Operation::KeyTick as u32 => key_tick(p),
        x if x == Operation::View as u32 && (0..43).contains(&argument) => view(p, argument),
        x if x == Operation::DjNext as u32 => crate::automatic::dj_next(p),
        x if x == Operation::DjBar as u32 => crate::automatic::dj_bar(p),
        x if x == Operation::AutoFx as u32 => crate::automatic::auto_fx(p),
        x if x == Operation::MixKey as u32 && (0..27).contains(&argument) => {
            return crate::automatic::mix_key(p, argument as usize)
        }
        _ => return false,
    }
    true
}
