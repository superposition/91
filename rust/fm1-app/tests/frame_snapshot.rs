// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::{
    buttons::page,
    controller::{self, Action, FrameState, Port, Query, Views},
    ui::State,
};

struct Panel {
    ui: State,
    frame: FrameState,
    on: bool,
    captured_on: bool,
    exit_on_input: bool,
    snapshot_ok: bool,
    actions: Vec<Action>,
}
impl Panel {
    fn new(view: u8) -> Self {
        // UI ABI carrier contains only numeric fields/arrays; zero is valid.
        let mut ui: State = unsafe { core::mem::zeroed() };
        ui.view = view;
        Self {
            ui,
            frame: FrameState::default(),
            on: true,
            captured_on: false,
            exit_on_input: false,
            snapshot_ok: true,
            actions: vec![],
        }
    }
    fn draw(&mut self) {
        controller::frame(
            self,
            Views {
                jam: 34,
                songbook: page::SONGBOOK,
                arrangement: 39,
                swells: page::SWELLS,
            },
        );
    }
}
impl Port for Panel {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.ui)
    }
    fn with_frame<R>(&mut self, f: impl FnOnce(&mut FrameState) -> R) -> R {
        f(&mut self.frame)
    }
    fn query(&mut self, query: Query) -> u32 {
        if query == Query::JamOn {
            u32::from(self.on)
        } else {
            0
        }
    }
    fn action(&mut self, action: Action, _: u32, _: u32) -> u32 {
        self.actions.push(action);
        match action {
            Action::Input if self.exit_on_input => {
                self.on = false;
                self.ui.view = page::PERFORM;
            }
            Action::JamSnapshot => {
                if self.snapshot_ok {
                    self.captured_on = self.on;
                }
                return u32::from(self.snapshot_ok);
            }
            Action::Header => assert_eq!(self.captured_on, self.on),
            _ => {}
        }
        0
    }
}

#[test]
fn every_page_captures_current_song_before_drawing_its_header() {
    for view in 0..43 {
        let mut p = Panel::new(view);
        p.draw();
        assert!(p.actions.ends_with(&[
            Action::Scan,
            Action::JamSnapshot,
            Action::Header,
            Action::Main,
            Action::Knobs,
            Action::Leds,
        ]));
        assert_eq!(p.ui.frame, 1);
    }
}

#[test]
fn songbook_exit_refreshes_stale_on_state_before_the_legacy_header() {
    let mut p = Panel::new(page::SONGBOOK);
    p.captured_on = true;
    p.exit_on_input = true;
    p.draw();
    assert_eq!(p.ui.view, page::PERFORM);
    assert!(!p.captured_on);
    assert!(p.actions.contains(&Action::Header));
}

#[test]
fn failed_snapshot_preserves_picture_on_every_page_and_still_updates_leds() {
    for view in 0..43 {
        let mut p = Panel::new(view);
        p.snapshot_ok = false;
        p.draw();
        assert!(p
            .actions
            .ends_with(&[Action::Scan, Action::JamSnapshot, Action::Leds]));
        for draw in [Action::Header, Action::Main, Action::Knobs] {
            assert!(!p.actions.contains(&draw));
        }
        assert_eq!(p.ui.frame, 1);
    }
}
