// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::{
    buttons::{self, Action, Notice, Port, Query},
    ui::State,
};
struct Panel {
    ui: State,
    synth: bool,
    actions: Vec<(u32, u32, u32)>,
}
impl Port for Panel {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.ui)
    }
    fn query(&mut self, q: Query, _: u32) -> u32 {
        match q {
            Query::JamOn => 1,
            Query::JamSynth => u32::from(self.synth),
            _ => 0,
        }
    }
    fn action(&mut self, a: Action, b: u32, c: u32) -> u32 {
        self.actions.push((a as u32, b, c));
        0
    }
    fn notice(&mut self, _: Notice, _: u32, _: u32) {}
}
#[test]
fn home_exits_synth_without_transport_or_chart_reset() {
    for (synth, fx_menu, fx_page) in [
        (false, 0, 0),
        (true, 0, 0),
        (true, 1, 0),
        (true, 1, 1),
        (true, 1, 32),
    ] {
        // UI ABI carrier contains only numeric fields/arrays; zero is valid.
        let mut p = Panel {
            ui: unsafe { core::mem::zeroed() },
            synth,
            actions: vec![],
        };
        p.ui.view = buttons::page::SWELLS;
        p.ui.fx_menu = fx_menu;
        p.ui.fx_page = fx_page;
        assert!(buttons::button(&mut p, buttons::id::HOME));
        let mut want = vec![];
        if synth {
            want.push((Action::JamSynth as u32, 0, 0));
        }
        want.push((Action::View as u32, buttons::page::SONGBOOK as u32, 0));
        assert_eq!(p.actions, want);
        assert_eq!((p.ui.fx_menu, p.ui.fx_page), (0, 0));
    }
}

#[test]
fn play_only_controls_transport_and_preserves_synth_mode() {
    // UI ABI carrier contains only numeric fields/arrays; zero is valid.
    let mut p = Panel {
        ui: unsafe { core::mem::zeroed() },
        synth: true,
        actions: vec![],
    };
    assert!(buttons::button(&mut p, buttons::id::PLAY));
    assert_eq!(p.actions, vec![(Action::Play as u32, 1, 0)]);
}
