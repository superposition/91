// SPDX-License-Identifier: GPL-3.0-only
use fm1_types::{CommandKind, CommandWord, ParamId, StepMod, PARAMETERS, PARAMETER_COUNT};

#[test]
fn parameter_metadata_matches_the_frozen_c_export() {
    let reference: serde_json::Value = serde_json::from_str(include_str!("params.json")).unwrap();
    let entries = reference.as_array().unwrap();
    assert_eq!(entries.len(), PARAMETER_COUNT);
    for (index, expected) in entries.iter().enumerate() {
        let actual = &PARAMETERS[index];
        assert_eq!(index, expected["i"].as_u64().unwrap() as usize);
        assert_eq!(actual.name, expected["n"].as_str().unwrap());
        assert_eq!(actual.min, expected["min"].as_i64().unwrap() as i16);
        assert_eq!(actual.max, expected["max"].as_i64().unwrap() as i16);
        assert_eq!(actual.default, expected["def"].as_i64().unwrap() as i16);
    }
    assert!(ParamId::new(PARAMETER_COUNT).is_err());
    assert!(ParamId::new(usize::MAX).is_err());
}

#[test]
fn packed_signed_commands_and_step_fields_preserve_every_payload() {
    for value in i16::MIN..=i16::MAX {
        let command = CommandWord::set(ParamId::TUNE, value);
        let expected = (73u32 << 16) | (value as u16 as u32);
        assert_eq!(command.bits(), expected);
        assert_eq!(command.value(), value);
        assert_eq!(command.argument(), 73);
    }
    for value in 0..=u8::MAX {
        let step = StepMod::from_bits(value);
        assert_eq!(step.ratchet(), value & 7);
        assert_eq!(step.push(), value >> 3 & 7);
        assert_eq!(step.level(), value >> 6);
    }
}

#[test]
fn typed_commands_preserve_all_kinds_and_signed_payloads() {
    for kind in 0..=38 {
        let typed = CommandKind::try_from(kind).unwrap();
        for value in i16::MIN..=i16::MAX {
            let word = CommandWord::encode(typed, 255, value);
            let command = word.decode().unwrap();
            assert_eq!(command.kind, typed);
            assert_eq!(command.argument, 255);
            assert_eq!(command.value, value);
            assert_eq!(
                word.bits(),
                (kind as u32) << 24 | 255 << 16 | value as u16 as u32
            );
        }
    }
    assert_eq!(CommandKind::Jam as u8, 38);
    for kind in 40..=255 {
        assert!(CommandWord::new(kind, 0, 0).decode().is_err());
    }
}

#[test]
fn synth_key_command_matches_c_id_and_preserves_owner_note_velocity() {
    let source = include_str!("../../../firmware/src/dsp/amb.c");
    let commands = source
        .split("enum { C_SET,")
        .nth(1)
        .unwrap()
        .split("};")
        .next()
        .unwrap();
    let ids = core::iter::once("C_SET")
        .chain(commands.split(',').map(str::trim))
        .collect::<Vec<_>>();
    assert_eq!(ids[CommandKind::Jam as usize], "C_JAM");
    assert_eq!(ids[CommandKind::JamKey as usize], "C_JAMKEY");
    assert_eq!(CommandKind::JamKey as u8, 39);
    for owner in [1, 27, 128, 188, 255] {
        for note in 0..=127 {
            for velocity in 0..=127 {
                let value = note | velocity << 8;
                let word = CommandWord::encode(CommandKind::JamKey, owner, value);
                assert_eq!(
                    word.bits(),
                    39u32 << 24 | u32::from(owner) << 16 | value as u32
                );
                let decoded = word.decode().unwrap();
                assert_eq!(decoded.kind, CommandKind::JamKey);
                assert_eq!(decoded.argument, owner);
                assert_eq!(decoded.value & 127, note);
                assert_eq!((decoded.value >> 8) & 127, velocity);
            }
        }
    }
}
