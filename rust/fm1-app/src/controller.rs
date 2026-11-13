// SPDX-License-Identifier: GPL-3.0-only
//! Foreground frame ordering. Every application callback runs between state borrows.
use crate::ui::State;

/// Replaces the seven existing frame-local static words, without another copy.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameState {
    pub drift_seen: u32,
    pub calm_at: u32,
    pub step_at: u32,
    pub last_xruns: u32,
    pub last_late: u32,
    pub window_at: u32,
    pub window_count: u32,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    Now,
    Buttons,
    Keys,
    SplashDuration,
    Theme,
    Playing,
    DriftChanges,
    NextPattern,
    JamOn,
    Xruns,
    Late,
    ShedLevel,
    GameOn,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Splash,
    ClearScreen,
    Play,
    Theme,
    Store,
    Drift,
    Remote,
    JamSync,
    Input,
    Scene,
    Dj,
    AutoFx,
    Mix,
    Sample,
    Key,
    Glide,
    Saver,
    Jam,
    Shed,
    ManualVoice,
    Autosave,
    GameUpdate,
    GameDraw,
    GameLeds,
    Scan,
    JamSnapshot,
    Header,
    Main,
    Knobs,
    Leds,
}

/// Closures cannot return a reference tied to their state argument. Implementors
/// must end that exclusive access before any query/action can reenter the app.
///
/// ```compile_fail
/// use fm1_app::{controller::Port, ui::State};
/// fn retained_ui<P: Port>(port: &mut P) -> &mut State {
///     port.with_ui(|ui| ui)
/// }
/// ```
pub trait Port {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R;
    fn with_frame<R>(&mut self, f: impl FnOnce(&mut FrameState) -> R) -> R;
    fn query(&mut self, query: Query) -> u32;
    fn action(&mut self, action: Action, first: u32, second: u32) -> u32;
}

pub struct Views {
    pub jam: u8,
    pub songbook: u8,
    pub arrangement: u8,
    pub swells: u8,
}

/// Same wrapping clock/counter rules as the original foreground load guard.
/// Returns a requested level; applying it is a separate, potentially reentrant effect.
pub fn load_shed(
    state: &mut FrameState,
    now: u32,
    xruns: u32,
    late: u32,
    playing: bool,
    level: u32,
) -> Option<u32> {
    let lost = xruns != state.last_xruns;
    state.window_count = state
        .window_count
        .wrapping_add(late.wrapping_sub(state.last_late));
    state.last_xruns = xruns;
    state.last_late = late;
    if !playing {
        state.calm_at = now;
        state.step_at = now;
        state.window_at = now;
        state.window_count = 0;
    } else {
        if now.wrapping_sub(state.window_at) > 10_000 {
            state.window_at = now;
            state.window_count = 0;
        }
        if lost || state.window_count >= 6 {
            state.calm_at = now;
            if level < 3 && now.wrapping_sub(state.step_at) > if lost { 4_000 } else { 10_000 } {
                state.step_at = now;
                state.window_count = 0;
                return Some(level + 1);
            }
        } else if now.wrapping_sub(state.calm_at) > 20_000 && level > 0 {
            state.calm_at = now;
            return Some(level - 1);
        }
    }
    None
}

fn effect(port: &mut impl Port, action: Action) {
    port.action(action, 0, 0);
}

pub fn frame(port: &mut impl Port, views: Views) {
    if port.with_ui(|ui| ui.splash != 0) {
        let held = port.query(Query::Buttons) | port.query(Query::Keys);
        let now = port.query(Query::Now);
        let boot = port.with_ui(|ui| ui.boot_t);
        let duration = port.query(Query::SplashDuration);
        if now.wrapping_sub(boot) < duration && held == 0 {
            effect(port, Action::Splash);
            return;
        }
        port.with_ui(|ui| ui.splash = 0);
        let buttons = port.query(Query::Buttons);
        let keys = port.query(Query::Keys);
        port.with_ui(|ui| {
            ui.btn = buttons;
            ui.keys = keys;
            ui.btn_used = buttons;
            ui.shift_keys = keys;
        });
        effect(port, Action::ClearScreen);
        if port.query(Query::Playing) == 0 {
            effect(port, Action::Play);
        }
    }
    let theme = port.query(Query::Theme) as u8;
    if port.with_ui(|ui| ui.theme != theme) {
        port.with_ui(|ui| {
            ui.theme = theme;
            ui.sig.fill(0);
        });
        port.action(Action::Theme, u32::from(theme), 0);
    }
    effect(port, Action::Store);
    let changes = port.query(Query::DriftChanges);
    if port.with_frame(|state| {
        let changed = state.drift_seen != changes;
        state.drift_seen = changes;
        changed
    }) {
        let pattern = port.query(Query::NextPattern);
        port.action(Action::Drift, pattern, 0);
    }
    if port.action(Action::Remote, 0, 0) != 0 {
        return;
    }
    effect(port, Action::JamSync);
    effect(port, Action::Input);
    if port.query(Query::JamOn) == 0 {
        effect(port, Action::Scene);
    }
    for action in [
        Action::Dj,
        Action::AutoFx,
        Action::Mix,
        Action::Sample,
        Action::Key,
        Action::Glide,
        Action::Saver,
        Action::Jam,
    ] {
        effect(port, action);
    }
    let now = port.query(Query::Now);
    let xruns = port.query(Query::Xruns);
    let late = port.query(Query::Late);
    let playing = port.query(Query::Playing) != 0;
    let level = port.query(Query::ShedLevel);
    if let Some(level) = port.with_frame(|state| load_shed(state, now, xruns, late, playing, level))
    {
        port.action(Action::Shed, level, 0);
    }
    let (old, want) = port.with_ui(|ui| {
        (
            ui.jam_man,
            if ui.view == views.jam {
                1 + (ui.jam_voice & 3)
            } else {
                0
            },
        )
    });
    if old != want {
        if old != 0 {
            port.action(Action::ManualVoice, u32::from(old - 1), 0);
        }
        if want != 0 {
            port.action(Action::ManualVoice, u32::from(want - 1), 1);
        }
        port.with_ui(|ui| ui.jam_man = want);
    }
    effect(port, Action::Autosave);
    if port.query(Query::GameOn) != 0 {
        effect(port, Action::GameUpdate);
        if port.with_ui(|ui| ui.frame % 3 == 0) {
            port.action(Action::GameDraw, 0, 0);
            port.action(Action::GameDraw, 1, 0);
        }
        effect(port, Action::GameLeds);
        port.with_ui(|ui| ui.frame = ui.frame.wrapping_add(1));
        return;
    }
    let scan = port.with_ui(|ui| ui.scan);
    port.action(Action::Scan, u32::from(scan), 0);
    // Song context appears in the header on every page. Capture after input,
    // including a Songbook exit, so the header cannot retain the prior mode.
    if port.action(Action::JamSnapshot, 0, 0) == 0 {
        effect(port, Action::Leds);
        port.with_ui(|ui| ui.frame = ui.frame.wrapping_add(1));
        return;
    }
    if port.with_ui(|ui| ui.saver_on == 0) {
        effect(port, Action::Header);
    }
    effect(port, Action::Main);
    if port.with_ui(|ui| ui.saver_on == 0) {
        effect(port, Action::Knobs);
    }
    effect(port, Action::Leds);
    port.with_ui(|ui| ui.frame = ui.frame.wrapping_add(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_requires_a_full_calm_period() {
        let mut state = FrameState::default();
        assert_eq!(load_shed(&mut state, 4000, 1, 0, true, 0), None);
        assert_eq!(load_shed(&mut state, 4001, 2, 0, true, 0), Some(1));
        assert_eq!(load_shed(&mut state, 24001, 2, 0, true, 1), None);
        assert_eq!(load_shed(&mut state, 24002, 2, 0, true, 1), Some(0));
    }

    #[test]
    fn stop_resets_windows_and_counters_wrap() {
        let mut state = FrameState::default();
        assert_eq!(
            load_shed(&mut state, u32::MAX - 3, u32::MAX, u32::MAX, false, 3),
            None
        );
        assert_eq!(state.window_count, 0);
        assert_eq!(load_shed(&mut state, 3997, 0, 0, true, 3), None);
        assert_eq!(state.window_count, 1);
        assert_eq!(load_shed(&mut state, 24000, 0, 0, true, 3), Some(2));
    }
}
