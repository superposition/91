// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{
    persistence::access,
    persistence_ui::*,
    project::{list_find, list_step, normalize},
};
#[test]
fn restore_lock_and_session_gate_preserve_failure_policy() {
    for (available, locked, restore, expected) in [
        (false, false, false, -9),
        (false, false, true, -9),
        (false, true, false, -9),
        (false, true, true, -9),
        (true, false, false, 0),
        (true, false, true, 1),
        (true, true, false, -8),
        (true, true, true, 1),
    ] {
        assert_eq!(access(available, locked, restore), expected);
    }
    for flags in 0..8 {
        assert_eq!(
            session_save_allowed(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0),
            flags == 0
        );
    }
    for (requested, expected) in [
        (i32::MIN, None),
        (-1, None),
        (0, Some(0)),
        (4, Some(4)),
        (5, None),
        (i32::MAX, None),
    ] {
        assert_eq!(session_slot(requested), expected);
    }
}
#[test]
fn autosave_priorities_and_wrapping_clock_are_explicit() {
    let mut clock = [5, 0xffff_ff00];
    let mut input = Autosave {
        place: 5,
        remembered: 1,
        now: 20_000,
        global_quiet: 2000,
        kit_quiet: 3000,
        ..Autosave::default()
    };
    assert_eq!(autosave(input, &mut clock), 256 | 3); // remember and write project
    input.playing = 1;
    assert_eq!(autosave(input, &mut clock), 0); // live without seamless
    input.seamless = 1;
    assert_eq!(autosave(input, &mut clock), 256 | 4);
    input.voices = 1;
    input.settings = 1;
    assert_eq!(autosave(input, &mut clock), 256 | 1);
    input.voices = 0;
    assert_eq!(autosave(input, &mut clock), 256 | 2);
    input.buttons = 1;
    assert_eq!(autosave(input, &mut clock), 256);
    input.place = 6;
    assert_eq!(autosave(input, &mut clock), 0);
    assert_eq!(clock, [6, 20_000]);
}
#[test]
fn session_fallback_retains_unknown_tail_and_missing_pack_source() {
    let default = boot_session(&[]);
    assert_eq!(session_values(&default), [0, 16, 0, 3, 60, 0, 0, 0, 100, 1]);
    let mut saved = default;
    saved[13] = 4;
    saved[16] = 2;
    saved[17] = b'S';
    saved[18] = 213;
    saved[19] = 91;
    assert_eq!(boot_session(&saved), saved);
    assert_eq!(resolve_session(&mut saved, false), Some(2));
    assert_eq!(saved[13], 0);
    assert_eq!(&saved[18..], [213, 91]);
    saved[5] = 12;
    assert_eq!(boot_session(&saved), default);
}
#[test]
fn malformed_settings_and_bounded_session_mutations_do_not_escape() {
    let defaults = [17; 100];
    let mut seed = 0x9e3779b9u32;
    for length in 0..512 {
        let mut raw = vec![0; length];
        for b in &mut raw {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *b = seed as u8;
        }
        let mut out = [0x69; 128];
        assert!(!migrate_settings(&raw, &defaults, &mut out));
        assert_eq!(out, [0x69; 128]);
        let session = boot_session(&raw);
        assert!(fm1_formats::session::JamSession::decode(&session).is_ok());
    }
    let base = boot_session(&[]);
    for index in 0..20 {
        for byte in 0..=255 {
            let mut changed = base;
            changed[index] = byte;
            assert!(fm1_formats::session::JamSession::decode(&boot_session(&changed)).is_ok());
        }
    }
}
#[test]
fn set_list_wraps_and_preserves_unknown_nonzero_entries() {
    let mut list = [0; 16];
    list[0] = 255;
    list[15] = 3;
    assert_eq!(list_step(&list, 255, 1), 0);
    assert_eq!(list_step(&list, 255, -1), 15);
    assert_eq!(list_step(&list, 15, 1), 0);
    assert_eq!(list_find(&list, 4, 0, 2), 15);
    assert_eq!(list_step(&list, i32::MIN, i32::MAX), -1);
    assert_eq!(normalize(i32::MIN, 10), 2);
    assert_eq!(normalize(i32::MAX, 19), 2);
}
