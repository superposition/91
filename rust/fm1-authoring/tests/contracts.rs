// SPDX-License-Identifier: GPL-3.0-only
use fm1_authoring::{
    compile, decode, factory, inspect_midi,
    preview::{plan, Options},
    song,
};
#[test]
fn authored_output_uses_the_instrument_validator() {
    for d in [
        factory(),
        song(0).unwrap(),
        song(1).unwrap(),
        song(2).unwrap(),
    ] {
        let b = compile(&d, None).unwrap();
        fm1_formats::jam::JamPack::parse(&b).unwrap();
        assert_eq!(compile(decode(&b, None).unwrap().value(), None).unwrap(), b);
    }
}
#[test]
fn malformed_files_and_counts_do_not_panic() {
    let mut seed = 91u32;
    for n in 0..4096 {
        let mut b = vec![0; n];
        for v in &mut b {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *v = (seed >> 24) as u8;
        }
        let _ = inspect_midi(&b);
        let _ = decode(&b, None);
    }
    for count in [0, 33, 128, 255] {
        let mut b = compile(&song(0).unwrap(), None).unwrap();
        b[5] = count;
        b[1124] = count;
        assert!(decode(&b, None).is_err());
    }
    let midi=b"MThd\0\0\0\x06\0\x01\0\x01\x01\xe0MTrk\0\0\0\x0c\0\x90\x24\x64\x81\x70\x80\x24\0\0\xff\x2f\0";
    for len in 0..midi.len() {
        let _ = inspect_midi(&midi[..len]);
    }
    for at in 0..midi.len() {
        for value in [0, 127, 128, 255] {
            let mut data = midi.to_vec();
            data[at] = value;
            let _ = inspect_midi(&data);
        }
    }
}
#[test]
fn preview_duration_matches_legacy_rounding() {
    let p = plan(Options {
        bars: 8,
        bpm: 127,
        key: 11,
        minor: true,
        phrases: 3,
    })
    .unwrap();
    assert!(p.contains("jam 1 23\n"));
    assert!(p.contains("wait 45354\n"));
    assert!(plan(Options {
        bars: 7,
        bpm: 127,
        key: 0,
        minor: false,
        phrases: 3
    })
    .is_err());
}
