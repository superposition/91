// SPDX-License-Identifier: GPL-3.0-only
use fm1_authoring::{compile, decode, request, song};
use fm1_engine::planner::{bass_event, plan_into, Phrase, Seed, Settings};
use fm1_formats::jam::{JamPack, PackError, BASS, V2_CHART, V2_HEAD};
use serde_json::{json, Value};

const TONES: [[u8; 6]; 17] = [
    [0, 4, 7, 11, 4, 7],
    [0, 3, 7, 10, 3, 7],
    [0, 4, 7, 10, 4, 7],
    [0, 3, 6, 10, 3, 6],
    [0, 3, 6, 9, 3, 6],
    [0, 4, 7, 9, 4, 7],
    [0, 3, 7, 9, 3, 7],
    [0, 5, 7, 10, 5, 7],
    [0, 3, 7, 11, 3, 7],
    [0, 4, 8, 10, 4, 8],
    [4, 7, 10, 13, 4, 7],
    [4, 7, 11, 14, 4, 7],
    [3, 7, 10, 14, 3, 7],
    [4, 7, 10, 14, 4, 7],
    [4, 8, 10, 13, 4, 8],
    [0, 4, 7, 12, 4, 7],
    [0, 3, 7, 12, 3, 7],
];
fn chart() -> Value {
    let mut d = song(0).unwrap();
    d["version"] = json!(3);
    d["nodes"] = json!((0..17).map(|i| json!({"bass":i%12,"sixth":0,"scale":0,"label":0,"mode":0,"emphasis":0,"borrow":0,"quality":i})).collect::<Vec<_>>());
    d["cadence"] = json!([[0, 1, 2], [0, 1, 2]]);
    d["charts"] = json!((0..2)
        .map(|m| (0..2)
            .map(|l| (0..2)
                .map(|p| (0..if l == 0 { 16 } else { 32 })
                    .map(|i| ((m * 4 + l * 2 + p) * 3 + i) % 17)
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>())
        .collect::<Vec<_>>());
    d
}
#[test]
fn shared_charts_flags_and_signed_octaves_roundtrip_without_changing_defaults() {
    let mut d = chart();
    let baseline = compile(&d, None).unwrap();
    assert_eq!(baseline[V2_HEAD + 21..V2_HEAD + 25], [0, 0, 0, 0]);
    assert_eq!(
        compile(decode(&baseline, None).unwrap().value(), None).unwrap(),
        baseline
    );
    for flags in 0..4 {
        for keys in -2i8..=2 {
            for bass in -2i8..=2 {
                d["parts"]["voice_flags"] = json!(flags);
                d["parts"]["keys_octave"] = json!(keys);
                d["parts"]["bass_octave"] = json!(bass);
                let b = compile(&d, None).unwrap();
                let mut want = baseline.clone();
                want[V2_HEAD + 22] = flags;
                want[V2_HEAD + 23] = keys as u8;
                want[V2_HEAD + 24] = bass as u8;
                assert_eq!(b, want);
                assert_eq!(compile(decode(&b, None).unwrap().value(), None).unwrap(), b);
            }
        }
    }
    for (offset, invalid) in [(21, 2), (22, 4), (23, 3), (23, 253), (24, 127)] {
        let mut b = baseline.clone();
        b[V2_HEAD + offset] = invalid;
        assert_eq!(JamPack::parse(&b).unwrap_err(), PackError::Song);
        assert!(decode(&b, None).is_err());
    }
    let mut old = song(0).unwrap();
    let baseline = compile(&old, None).unwrap();
    old["parts"]["voice_flags"] = json!(3);
    old["parts"]["keys_octave"] = json!(2);
    assert_eq!(compile(&old, None).unwrap(), baseline);
}

#[test]
fn literal_schema_rejects_malformed_banks_and_accepts_thirty_two_shared_nodes() {
    for bad in [json!(3.0), json!(true), json!("3")] {
        let mut d = chart();
        d["version"] = bad;
        assert!(compile(&d, None).is_err());
    }
    for field in ["quality", "voice_flags", "keys_octave", "bass_octave"] {
        for bad in [json!(true), json!(1.0), json!("1"), Value::Null] {
            let mut d = chart();
            if field == "quality" {
                d["nodes"][0][field] = bad;
            } else {
                d["parts"][field] = bad;
            }
            assert!(compile(&d, None).is_err());
        }
    }
    for bank in ["cadence", "charts"] {
        let mut d = chart();
        d[bank].as_array_mut().unwrap().pop();
        assert!(compile(&d, None).is_err());
    }
    let mut d = chart();
    let node = d["nodes"][0].clone();
    d["nodes"].as_array_mut().unwrap().resize(32, node);
    d["charts"][1][1][1][31] = json!(31);
    d["cadence"][1][0] = json!(31);
    let b = compile(&d, None).unwrap();
    assert_eq!(JamPack::parse(&b).unwrap().node_count(), 32);
    assert_eq!(compile(decode(&b, None).unwrap().value(), None).unwrap(), b);
}

#[test]
fn all_qualities_keys_and_voicings_preserve_exact_pitch_sets_and_bass_quality() {
    let d = chart();
    let mut b = compile(&d, None).unwrap();
    for (quality, tones) in TONES.iter().enumerate() {
        for root in 0..12 {
            let node = if quality < 16 { 64 + quality * 8 } else { 1152 };
            b[node] = root;
            b[V2_CHART..V2_CHART + 256].fill(quality as u8);
            for key in 0..12 {
                for movement in 0..4 {
                    for voicing in 0..2 {
                        b[V2_HEAD + 21] = voicing;
                        for motif in 0..b[6] as usize {
                            b[BASS + motif * 36] = 130;
                            b[BASS + motif * 36 + 1] = 131;
                            b[BASS + motif * 36 + 16] = 100;
                            b[BASS + motif * 36 + 17] = 100;
                        }
                        let pack = JamPack::parse(&b).unwrap();
                        let s = Settings {
                            key,
                            movement,
                            bars: 16,
                            ..Settings::default()
                        };
                        let mut phrase = Phrase::EMPTY;
                        let mut seed = Seed::new(91);
                        plan_into(pack, &s, 3, None, &mut phrase, &mut seed).unwrap();
                        let drop = movement == 2 || (movement == 0 && voicing == 1);
                        for beat in phrase.active_beats() {
                            let mut want = [0u8; 12];
                            let mut got = [0u8; 12];
                            for interval in &tones[..4] {
                                want[((key + root + interval) % 12) as usize] += 1;
                            }
                            for n in beat.notes {
                                assert!((if drop { 45 } else { 48 }..=76).contains(&n));
                                got[(n % 12) as usize] += 1;
                            }
                            assert_eq!(got, want);
                            assert!(beat.notes.windows(2).all(|n| n[0] < n[1]));
                            assert_eq!(beat.passing, 0);
                        }
                        for step in 0..2 {
                            let bass = bass_event(pack, &s, &phrase, &phrase, 0, step).unwrap();
                            assert_eq!(
                                bass.note % 12,
                                ((key + root + tones[4 + step]) % 12) as i16
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn authored_half_bars_and_cadences_are_never_rewritten() {
    let b = compile(&chart(), None).unwrap();
    let pack = JamPack::parse(&b).unwrap();
    for minor in 0..2 {
        for bars in [8, 16] {
            for form in 0..4 {
                for movement in 0..4 {
                    let s = Settings {
                        minor,
                        bars,
                        form,
                        movement,
                        ..Settings::default()
                    };
                    let mut previous = Phrase::EMPTY;
                    let mut seed = Seed::new(91);
                    for section in 0..8 {
                        let mut phrase = Phrase::EMPTY;
                        plan_into(pack, &s, section, Some(&previous), &mut phrase, &mut seed)
                            .unwrap();
                        let part = usize::from(phrase.role == 1 || phrase.role == 2);
                        let base = V2_CHART
                            + ((minor as usize * 2 + usize::from(bars == 16)) * 2 + part) * 32;
                        for (i, beat) in phrase.active_beats().iter().enumerate() {
                            assert_eq!(beat.node, b[base + i / 2]);
                        }
                        previous = phrase;
                    }
                }
            }
        }
    }
}

#[test]
fn device_version_negotiation_and_octave_controls_are_explicit() {
    let b = compile(&chart(), None).unwrap();
    assert_eq!(
        request(&json!({"method":"device-needs-features","bytes":b})).unwrap(),
        true
    );
    assert!(request(
        &json!({"method":"device-compatible","version":3,"features":{"pack_versions":[1,2]}})
    )
    .is_err());
    assert!(request(
        &json!({"method":"device-compatible","version":3,"features":{"pack_versions":[1,2,3]}})
    )
    .is_ok());
    for field in ["keys_octave", "bass_octave"] {
        for value in -3..=3 {
            assert_eq!(
                request(&json!({"method":"device-control-check","field":field,"value":value}))
                    .is_ok(),
                (-2..=2).contains(&value)
            );
            let action = fm1_authoring::device::start(
                &json!("control"),
                &json!({"field":field,"value":value}),
            );
            if (-2..=2).contains(&value) {
                assert_eq!(
                    action.unwrap()["action"]["data"],
                    json!([if field == "keys_octave" { 22 } else { 23 }, 0, value + 2])
                );
            } else {
                assert!(action.is_err());
            }
        }
    }
    let features = fm1_authoring::device::start(&json!("features"), &json!({})).unwrap();
    let answer =
        fm1_authoring::device::advance(features["state"].clone(), &json!([7, 8, 3, 6]), None)
            .unwrap();
    assert_eq!(answer["value"]["pack_versions"], json!([1, 2, 3]));
}

#[test]
fn synth_mode_roundtrips_in_one_reserved_byte_and_space_uses_existing_sound() {
    let mut d = chart();
    let baseline = compile(&d, None).unwrap();
    assert_eq!(baseline[V2_HEAD + 25], 0);
    d["parts"]["synth"] = json!(1);
    let b = compile(&d, None).unwrap();
    let mut want = baseline.clone();
    want[V2_HEAD + 25] = 1;
    assert_eq!(b, want);
    let restored = decode(&b, None).unwrap();
    assert_eq!(restored.value()["parts"]["synth"], 1);
    assert_eq!(
        restored.value()["sounds"],
        decode(&baseline, None).unwrap().value()["sounds"]
    );
    assert_eq!(compile(restored.value(), None).unwrap(), b);
    assert_eq!(
        decode(&baseline, None).unwrap().value()["parts"]["synth"],
        0
    );
    for bad in [
        json!(-1),
        json!(2),
        json!(true),
        json!(1.0),
        json!("1"),
        Value::Null,
    ] {
        d["parts"]["synth"] = bad;
        assert!(compile(&d, None).is_err());
    }
    for bad in [2, 128, 255] {
        let mut damaged = baseline.clone();
        damaged[V2_HEAD + 25] = bad;
        assert!(JamPack::parse(&damaged).is_err());
        assert!(decode(&damaged, None).is_err());
    }
    let mut old = song(0).unwrap();
    let prior = compile(&old, None).unwrap();
    old["parts"]["synth"] = json!(1);
    assert_eq!(compile(&old, None).unwrap(), prior);
    let fields = fm1_authoring::device::fields();
    assert_eq!(fields["synth"], 24);
    assert_eq!(fields["space"], 25);
    for (field, maximum) in [("synth", 1), ("space", 100)] {
        for value in [-1, 0, 1, maximum, maximum + 1] {
            let action = fm1_authoring::device::start(
                &json!("control"),
                &json!({"field":field,"value":value}),
            );
            if (0..=maximum).contains(&value) {
                assert_eq!(
                    action.unwrap()["action"]["data"],
                    json!([fields[field], 0, value])
                );
            } else {
                assert!(action.is_err());
            }
        }
    }
}
