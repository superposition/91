// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::{
    buttons::page,
    input::{self, Action, Notice, Port, Query},
    ui::State,
};
struct Panel {
    ui: State,
    cache: (u32, u8),
    version: u32,
    field: u32,
    current: i32,
    encoder: u32,
    delta: i32,
    actions: Vec<(u32, i32)>,
    synth: bool,
    keys: u32,
    played: Vec<(u32, u32)>,
    ordinary_keys: Vec<u32>,
    midi_polls: u32,
}
impl Panel {
    fn new(view: u8, version: u32, field: u32, current: i32, encoder: u32, delta: i32) -> Self {
        // State is the repr(C) UI ABI carrier containing only integer/float fields
        // and arrays of those fields; zero is valid for every field.
        let mut ui: State = unsafe { core::mem::zeroed() };
        ui.view = view;
        Self {
            ui,
            cache: (0, 0),
            version,
            field,
            current,
            encoder,
            delta,
            actions: vec![],
            synth: false,
            keys: 0,
            played: vec![],
            ordinary_keys: vec![],
            midi_polls: 0,
        }
    }
}
impl Port for Panel {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.ui)
    }
    fn with_cache<R>(&mut self, f: impl FnOnce(&mut u32, &mut u8) -> R) -> R {
        f(&mut self.cache.0, &mut self.cache.1)
    }
    fn query(&mut self, q: Query, a: u32, _: u32) -> u32 {
        match q {
            Query::JamVersion => self.version,
            Query::Jam if a == 0 || a == 24 => u32::from(self.synth),
            Query::Keys => self.keys,
            Query::Jam if a == self.field => self.current as u32,
            Query::Encoder if a == self.encoder => self.delta as u32,
            _ => 0,
        }
    }
    fn action(&mut self, a: Action, first: u32, second: u32) -> u32 {
        if matches!(a, Action::Jam) {
            self.actions.push((first, second as i32));
        }
        if matches!(a, Action::SynthKey) {
            self.played.push((first, second));
        }
        if matches!(a, Action::Midi) {
            self.midi_polls += 1;
        }
        if matches!(
            a,
            Action::Note
                | Action::ShiftKey
                | Action::MixKey
                | Action::GateStep
                | Action::StepToggle
                | Action::ToggleNote
                | Action::Fill
        ) || (matches!(a, Action::SampleKeys) && first | second != 0)
        {
            self.ordinary_keys.push(a as u32);
        }
        0
    }
    fn notice(&mut self, _: Notice, _: u32, _: u32) {}
}

#[test]
fn synth_screensaver_keeps_midi_releases_live_but_panel_keys_only_wake() {
    for (version, synth, expected_polls) in
        [(3, true, 1), (3, false, 0), (2, true, 0), (1, true, 0)]
    {
        let mut p = Panel::new(page::SWELLS, version, 1, 0, 0, 0);
        p.synth = synth;
        p.ui.saver_on = 1;
        p.keys = 1;
        input::poll(&mut p);
        assert_eq!(p.midi_polls, expected_polls);
        assert!(p.played.is_empty());
        assert!(p.actions.is_empty());
        assert!(p.ordinary_keys.is_empty());
        assert_eq!(p.ui.keys, 1);
    }
}

#[test]
fn synth_page_reuses_existing_id_and_routes_mode_program_octave_space() {
    for (knob, field) in [(0, 24), (1, 14), (2, 22), (3, 25)] {
        let mut p = Panel::new(page::SWELLS, 3, field, 0, 3 + knob, 1);
        input::poll(&mut p);
        assert_eq!(p.actions, vec![(field, 1)]);
    }
    for version in [1, 2] {
        let mut p = Panel::new(page::SWELLS, version, 19, 0, 3, 1);
        input::poll(&mut p);
        assert_eq!(p.actions, vec![(19, 1)]);
    }
}

#[test]
fn synth_keys_are_exclusive_across_pages_and_release_after_navigation() {
    for (view, fx_menu, fx_page) in [
        (page::SONGBOOK, 0, 0),
        (page::SWELLS, 0, 0),
        (page::PERFORM, 0, 0),
        (40, 0, 0),
        (41, 0, 0),
        (page::SONGBOOK, 1, 0),
        (page::SONGBOOK, 1, 1),
        (page::SONGBOOK, 1, 20),
        (page::SONGBOOK, 1, 32),
    ] {
        let mut p = Panel::new(view, 3, 1, 0, 0, 0);
        p.synth = true;
        p.ui.fx_menu = fx_menu;
        p.ui.fx_page = fx_page;
        p.keys = (1 << 27) - 1;
        input::poll(&mut p);
        assert_eq!(p.played, (0..27).map(|i| (i, 100)).collect::<Vec<_>>());
        assert!(p.actions.is_empty()); // F3/G3/etc are notes, never Band commands.
        assert!(p.ordinary_keys.is_empty());
        assert_eq!(p.ui.keys, p.keys); // Keep physical edges across navigation.
        p.ui.view = page::SONGBOOK;
        p.ui.fx_menu = 1 - fx_menu;
        p.ui.fx_page = 32;
        p.keys = 0;
        input::poll(&mut p);
        assert_eq!(
            &p.played[27..],
            &(0..27).map(|i| (i, 0)).collect::<Vec<_>>()
        );
        assert!(p.ordinary_keys.is_empty());
    }
}
#[test]
fn literal_key_dial_wraps_twelve_and_preserves_selected_bank() {
    assert_eq!(Query::JamVersion as u32, 38);
    for (key, delta, want) in [(11, 1, 0), (0, -1, 11), (23, 1, 12), (12, -1, 23)] {
        let mut p = Panel::new(page::SONGBOOK, 3, 1, key, 3, delta);
        input::poll(&mut p);
        assert_eq!(p.actions, vec![(1, want)]);
    }
    for version in [1, 2] {
        let mut p = Panel::new(page::SONGBOOK, version, 1, 11, 3, 1);
        input::poll(&mut p);
        assert_eq!(p.actions, vec![(1, 12)]);
    }
}
#[test]
fn direct_octave_knob_routes_signed_values_only_for_literal_packs() {
    for (view, field, legacy) in [(40, 22, 16), (41, 23, 20)] {
        let mut p = Panel::new(view, 3, field, 0, 4, -1);
        input::poll(&mut p);
        assert_eq!(p.actions, vec![(field, -1)]);
        for version in [1, 2] {
            let mut p = Panel::new(view, version, legacy, 50, 4, 1);
            input::poll(&mut p);
            assert_eq!(p.actions, vec![(legacy, 51)]);
        }
    }
}
