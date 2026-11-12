// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{sequencer::Rng, voices::pad::*};
// Caller-side sink for portable vectors. Native comparisons exercise the real
// C callback/queue; this model only captures its serialized16-entry admission.
struct Midi {
    statuses: [u8; 16],
    data1: [u8; 16],
    data2: [u8; 16],
    count: u8,
}
impl Midi {
    fn push(&mut self, s: u8, a: u8, b: u8) {
        if self.count < 16 {
            let n = self.count as usize;
            self.statuses[n] = s;
            self.data1[n] = a;
            self.data2[n] = b;
            self.count += 1;
        }
    }
}
fn words(v: &serde_json::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
fn floats(w: &[u32]) -> Vec<f32> {
    w.iter().map(|v| f32::from_bits(*v)).collect()
}
fn bytes(v: &serde_json::Value) -> Vec<u8> {
    words(v).iter().map(|v| *v as u8).collect()
}
fn state(w: &[u32]) -> State {
    let f = |i| f32::from_bits(w[i]);
    State {
        ph: [f(0), f(1), f(2), f(3)],
        inc: [f(4), f(5), f(6), f(7)],
        sub: f(8),
        note: f(9),
        target: f(10),
        t: f(11),
        rate: f(12),
        peak: f(13),
        amp: f(14),
        fa: f(15),
        fh: f(16),
        fr: f(17),
        ic: [f(18), f(19), f(20), f(21)],
        c1: [f(22), f(23), f(24), f(25)],
        c2: [f(26), f(27), f(28), f(29)],
        vib: f(30),
        pan: f(31),
        hf: f(32),
        midi_note: w[33] as u8,
        hand: (w[33] >> 8) as u8,
        voices: (w[33] >> 16) as u8,
        play: (w[33] >> 24) as u8,
        pnote: w[34] as u8,
        pord: w[35],
        pamp: f(36),
    }
}
fn state_words(s: &State) -> Vec<u32> {
    let mut w = Vec::new();
    for a in [
        &s.ph[..],
        &s.inc,
        &[
            s.sub, s.note, s.target, s.t, s.rate, s.peak, s.amp, s.fa, s.fh, s.fr,
        ],
        &s.ic,
        &s.c1,
        &s.c2,
        &[s.vib, s.pan, s.hf],
    ] {
        w.extend(a.iter().map(|v| v.to_bits()));
    }
    w.push(
        s.midi_note as u32
            | ((s.hand as u32) << 8)
            | ((s.voices as u32) << 16)
            | ((s.play as u32) << 24),
    );
    w.push(s.pnote as u32);
    w.push(s.pord);
    w.push(s.pamp.to_bits());
    w
}
fn config(w: &[u32]) -> Config {
    let f = |i| f32::from_bits(w[i]);
    Config {
        breath: Breath {
            attack: f(0),
            hold: f(1),
            release: f(2),
            rest: f(3),
            playing: w[4],
            manual: w[5],
            oscillator: w[6],
        },
        gain: f(7),
        gate: f(8),
        spread: f(9),
        motion: f(10),
        modulation: [f(11), f(12), f(13), f(14), f(15), f(16)],
        pitch: f(17),
        bright: f(18),
        arc: f(19),
        filter_mod: f(20),
        panning: f(21),
        sound_cut: f(22),
        tuning: f(23),
        stab_decay: f(24),
        swell: f(25),
        breath_scale: [f(26), f(27), f(28), f(29)],
        vowels: [
            [f(30), f(31)],
            [f(32), f(33)],
            [f(34), f(35)],
            [f(36), f(37)],
        ],
        stabs: w[38],
        dx: w[39],
        filter: w[40] as i32,
    }
}
fn bus_config(w: &[u32]) -> BusConfig {
    let f = |i| f32::from_bits(w[i]);
    BusConfig {
        sound_cut: f(0),
        bright: f(1),
        arc: f(2),
        filter_mod: f(3),
        link_amount: f(4),
        link_env: f(5),
        gain0: f(6),
        gain1: f(7),
        inverse: f(8),
        start: w[9],
    }
}
fn hash_byte(h: u32, b: u8) -> u32 {
    (h ^ b as u32).wrapping_mul(16777619)
}
fn hash_word(mut h: u32, w: u32) -> u32 {
    for b in w.to_le_bytes() {
        h = hash_byte(h, b);
    }
    h
}
#[test]
fn all_pad_sounds_breaths_handoffs_keys_queue_and_bus_match_frozen_c() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("pad-native.json")).unwrap();
    let sine: serde_json::Value = serde_json::from_str(include_str!("voices-native.json")).unwrap();
    let table: [f32; 1025] = floats(&words(&sine["sine"])).try_into().unwrap();
    let mut covered = [false; 6];
    let mut count = 0;
    for case in corpus["cases"].as_array().unwrap() {
        let id = case["test"].as_u64().unwrap();
        let w = words(&case["config"]);
        let mut hash = 2166136261;
        let mut peak = 0.0f32;
        if id < 300 {
            let state_input = words(&case["state"]);
            let mut voices = core::array::from_fn(|i| state(&state_input[i * 37..]));
            let c = config(&w);
            let metrics = words(&case["metrics"]);
            let mut rt = Runtime {
                gain: f32::from_bits(metrics[0]),
                stab_env: f32::from_bits(metrics[1]),
                controls: words(&case["controls"]).try_into().unwrap(),
            };
            let mut midi = Midi {
                statuses: bytes(&case["midi"][0]).try_into().unwrap(),
                data1: bytes(&case["midi"][1]).try_into().unwrap(),
                data2: bytes(&case["midi"][2]).try_into().unwrap(),
                count: case["midi_count"].as_u64().unwrap() as u8,
            };
            let mut rng = Rng(case["seed"].as_u64().unwrap() as u32);
            let mut notes: [u8; 4] = bytes(&case["ui_notes"]).try_into().unwrap();
            let mut amps: [f32; 4] = floats(&words(&case["ui_amps"])).try_into().unwrap();
            for block in 0..128 {
                let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][block % 9];
                let (mut left, mut right) = ([0.0; 256], [0.0; 256]);
                let before = voices;
                let controls = rt.controls;
                let seed = rng.0;
                let publication = process(
                    &mut voices,
                    &mut rt,
                    &c,
                    &mut rng,
                    &table,
                    &mut left[..n],
                    &mut right[..n],
                    &mut |s, a, b| midi.push(s, a, b),
                );
                covered[0] |= midi.count == 16;
                covered[1] |= rng.0 != seed;
                covered[2] |= publication.mask == 0;
                covered[3] |= c.dx != 0 && c.stabs != 0;
                for i in 0..4 {
                    covered[4] |= before[i].hand == 1 && voices[i].hand == 2;
                    covered[5] |= before[i].play == 2 && voices[i].play == 0;
                    if publication.mask >> i & 1 != 0 {
                        notes[i] = publication.notes[i];
                        amps[i] = publication.amplitudes[i];
                    }
                    if rt.controls[i] < controls[i] {
                        assert!(n > 0);
                    }
                }
                for b in [&left, &right] {
                    for v in &b[..n] {
                        assert!(v.is_finite(), "case {id}");
                        peak = peak.max(v.abs());
                        hash = hash_word(hash, v.to_bits());
                    }
                }
                for v in &voices {
                    for w in state_words(v) {
                        hash = hash_word(hash, w);
                    }
                }
                for w in [rt.gain.to_bits(), rt.stab_env.to_bits()] {
                    hash = hash_word(hash, w);
                }
                for w in rt.controls {
                    hash = hash_word(hash, w);
                }
                for b in [&midi.statuses, &midi.data1, &midi.data2] {
                    for v in b {
                        hash = hash_byte(hash, *v);
                    }
                }
                hash = hash_byte(hash, midi.count);
                hash = hash_word(hash, rng.0);
                for note in notes {
                    hash = hash_byte(hash, note);
                }
                for amp in amps {
                    hash = hash_word(hash, amp.to_bits());
                }
                hash = hash_word(hash, publication.level.to_bits());
                assert!(midi.count <= 16);
                if n > 0 {
                    midi.count = 0;
                }
            }
        } else {
            let mut c = bus_config(&w);
            let mut filter: [f32; 4] = floats(&words(&case["filter"])).try_into().unwrap();
            let mut dry: [f32; 512] = floats(&words(&corpus["dry_initial"])).try_into().unwrap();
            for block in 0..128 {
                let n = [0, 1, 31, 32, 33, 127, 128, 129, 256][block % 9];
                let (mut left, mut right, mut out_left, mut out_right) =
                    ([0.0; 256], [0.0; 256], [0.0; 256], [0.0; 256]);
                for j in 0..n {
                    left[j] = (j as i32 - 128) as f32 * 0.0005;
                    right[j] = (128 - j as i32) as f32 * 0.0003;
                }
                bus(
                    &mut filter,
                    &c,
                    Bus {
                        left: &left[..n],
                        right: &right[..n],
                        out_left: &mut out_left[..n],
                        out_right: &mut out_right[..n],
                        dry: &mut dry,
                    },
                );
                for b in [&out_left, &out_right] {
                    for v in &b[..n] {
                        assert!(v.is_finite());
                        peak = peak.max(v.abs());
                        hash = hash_word(hash, v.to_bits());
                    }
                }
                for v in filter {
                    hash = hash_word(hash, v.to_bits());
                }
                for v in dry {
                    hash = hash_word(hash, v.to_bits());
                }
                hash = hash_word(hash, c.start);
                c.start = c.start.wrapping_add(n as u32);
            }
        }
        assert_eq!(hash, case["hash"].as_u64().unwrap() as u32, "case {id}");
        assert!(peak < 64.0);
        count += 1;
    }
    assert_eq!(count, 312);
    assert!(
        covered.into_iter().all(|b| b),
        "queue/RNG/silence/DX/handoff/key-release coverage: {covered:?}"
    );
}
#[test]
fn split_ln2_exponential_matches_frozen_c_across_clamping_and_reduction() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("pad-native.json")).unwrap();
    for (i, w) in words(&corpus["exp_samples"]).into_iter().enumerate() {
        let input = -90.0 + i as f32 * 0.1875;
        assert_eq!(
            fm1_engine::effects::math::exp(input).to_bits(),
            w,
            "input {input}"
        );
    }
}
#[test]
fn breath_callbacks_preserve_old_off_then_new_on_and_rng_when_silent() {
    for (playing, manual, old, count) in [(1, 0, 60, 2), (1, 1, 60, 1), (0, 0, 60, 0), (1, 0, 0, 1)]
    {
        let mut state = State {
            midi_note: old,
            target: 64.0,
            ..State::default()
        };
        let config = Breath {
            attack: 0.1,
            hold: 0.2,
            release: 0.3,
            rest: 0.0,
            playing,
            manual,
            oscillator: 3,
        };
        let mut rng = Rng(91);
        let mut expected = Rng(91);
        expected.next_u32();
        expected.next_u32();
        let mut events = Vec::new();
        breathe(&mut state, &config, 1.21, &mut rng, &mut |s, a, b| {
            events.push((s, a, b))
        });
        assert_eq!(rng.0, expected.0);
        assert_eq!(state.note, 64.0);
        assert_eq!(state.voices, 3);
        assert_eq!(events.len(), count);
        if playing != 0 && old != 0 {
            assert_eq!(events[0], (0x80, old, 0));
        }
        if playing != 0 && manual == 0 {
            assert_eq!(
                events.last().copied(),
                Some((0x90, 64, (50.0 + 40.0 * state.peak) as u8))
            );
        }
    }
}
