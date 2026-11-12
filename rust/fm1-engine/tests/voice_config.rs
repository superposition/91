// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::{
    scheduling_tables,
    voices::{self, config, data},
};
fn words(v: &serde_json::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
}
fn droneinput(w: &[u32]) -> config::DroneInput {
    assert_eq!(w.len(), 25);
    config::DroneInput {
        playing: w[0],
        played: w[1],
        line_slide: w[2],
        bent: w[3],
        filter: w[4] as i32,
        squash_target: w[5] as i32,
        squash_drive: w[6] as i32,
        squash_amount: w[7] as i32,
        volume: f32::from_bits(w[8]),
        resonance: f32::from_bits(w[9]),
        envelope: f32::from_bits(w[10]),
        velocity: f32::from_bits(w[11]),
        lfo: f32::from_bits(w[12]),
        modulation: f32::from_bits(w[13]),
        arc: f32::from_bits(w[14]),
        motion: f32::from_bits(w[15]),
        section: f32::from_bits(w[16]),
        duck: f32::from_bits(w[17]),
        pulse: f32::from_bits(w[18]),
        link: f32::from_bits(w[19]),
        bend: f32::from_bits(w[20]),
        tuning: f32::from_bits(w[21]),
        gain0: f32::from_bits(w[22]),
        gain1: f32::from_bits(w[23]),
        inverse: f32::from_bits(w[24]),
    }
}
fn acidinput(w: &[u32]) -> config::AcidInput {
    assert_eq!(w.len(), 15);
    config::AcidInput {
        filter: w[0] as i32,
        squash_target: w[1] as i32,
        squash_drive: w[2] as i32,
        squash_amount: w[3] as i32,
        squash_wow: w[4] as i32,
        sliding: w[5],
        bent: w[6],
        volume: f32::from_bits(w[7]),
        cutoff: f32::from_bits(w[8]),
        resonance: f32::from_bits(w[9]),
        accent: f32::from_bits(w[10]),
        bend: f32::from_bits(w[11]),
        gain0: f32::from_bits(w[12]),
        gain1: f32::from_bits(w[13]),
        inverse: f32::from_bits(w[14]),
    }
}
fn padinput(w: &[u32]) -> config::PadInput {
    assert_eq!(w.len(), 26);
    config::PadInput {
        playing: w[0],
        jam: w[1],
        dx: w[2],
        genre: w[3],
        filter: w[4] as i32,
        volume: f32::from_bits(w[5]),
        level: f32::from_bits(w[6]),
        gate: f32::from_bits(w[7]),
        detune: f32::from_bits(w[8]),
        motion: f32::from_bits(w[9]),
        modulation: [
            f32::from_bits(w[10]),
            f32::from_bits(w[11]),
            f32::from_bits(w[12]),
            f32::from_bits(w[13]),
            f32::from_bits(w[14]),
            f32::from_bits(w[15]),
        ],
        pitch: f32::from_bits(w[16]),
        bright: f32::from_bits(w[17]),
        arc: f32::from_bits(w[18]),
        filter_mod: f32::from_bits(w[19]),
        panning: f32::from_bits(w[20]),
        tuning: f32::from_bits(w[21]),
        stab_decay: f32::from_bits(w[22]),
        swell: f32::from_bits(w[23]),
        section: f32::from_bits(w[24]),
        link: f32::from_bits(w[25]),
    }
}
fn padbusinput(w: &[u32]) -> config::PadBusInput {
    assert_eq!(w.len(), 8);
    config::PadBusInput {
        bright: f32::from_bits(w[0]),
        arc: f32::from_bits(w[1]),
        filter_mod: f32::from_bits(w[2]),
        link: f32::from_bits(w[3]),
        gain0: f32::from_bits(w[4]),
        gain1: f32::from_bits(w[5]),
        inverse: f32::from_bits(w[6]),
        start: w[7],
    }
}
fn drumsinput(w: &[u32]) -> config::DrumsInput {
    assert_eq!(w.len(), 21);
    config::DrumsInput {
        volume: f32::from_bits(w[0]),
        drive: f32::from_bits(w[1]),
        gains: [
            f32::from_bits(w[2]),
            f32::from_bits(w[3]),
            f32::from_bits(w[4]),
            f32::from_bits(w[5]),
            f32::from_bits(w[6]),
            f32::from_bits(w[7]),
        ],
        hat_inc: [
            f32::from_bits(w[8]),
            f32::from_bits(w[9]),
            f32::from_bits(w[10]),
            f32::from_bits(w[11]),
        ],
        gain0: [
            f32::from_bits(w[12]),
            f32::from_bits(w[13]),
            f32::from_bits(w[14]),
            f32::from_bits(w[15]),
        ],
        gain1: [
            f32::from_bits(w[16]),
            f32::from_bits(w[17]),
            f32::from_bits(w[18]),
            f32::from_bits(w[19]),
        ],
        inverse: f32::from_bits(w[20]),
    }
}
fn pluckinput(w: &[u32]) -> config::PluckInput {
    assert_eq!(w.len(), 7);
    config::PluckInput {
        volume: f32::from_bits(w[0]),
        bend: f32::from_bits(w[1]),
        gain0: f32::from_bits(w[2]),
        gain1: f32::from_bits(w[3]),
        inverse: f32::from_bits(w[4]),
        old_level: f32::from_bits(w[5]),
        bent: w[6],
    }
}
fn breath_words(v: &voices::pad::Breath) -> Vec<u32> {
    vec![
        v.attack.to_bits(),
        v.hold.to_bits(),
        v.release.to_bits(),
        v.rest.to_bits(),
        v.playing,
        v.manual,
        v.oscillator,
    ]
}
fn drone_words(v: &voices::drone::Config) -> Vec<u32> {
    vec![
        v.oscillator,
        v.filter as u32,
        v.line,
        v.playing,
        v.drive.to_bits(),
        v.k1.to_bits(),
        v.k2.to_bits(),
        v.input.to_bits(),
        v.gain.to_bits(),
        v.saw.to_bits(),
        v.sub.to_bits(),
        v.target.to_bits(),
        v.rate.to_bits(),
        v.fm_index.to_bits(),
        v.pulse_width.to_bits(),
        v.glide.to_bits(),
        v.pitch.to_bits(),
        v.bend.to_bits(),
        v.tuning.to_bits(),
        v.fm_ratio.to_bits(),
        v.cutoff.to_bits(),
        v.envelope.to_bits(),
        v.motion.to_bits(),
        v.arc.to_bits(),
        v.break_pos.to_bits(),
        v.break_neg.to_bits(),
        v.duck.to_bits(),
        v.links.to_bits(),
        v.gain0.to_bits(),
        v.gain1.to_bits(),
        v.inverse.to_bits(),
    ]
}
fn acid_words(v: &voices::acid::Config) -> Vec<u32> {
    vec![
        v.oscillator,
        v.filter as u32,
        v.gain.to_bits(),
        v.drive.to_bits(),
        v.resonance.to_bits(),
        v.envelope.to_bits(),
        v.accent.to_bits(),
        v.base.to_bits(),
        v.k1.to_bits(),
        v.k2.to_bits(),
        v.input.to_bits(),
        v.wow.to_bits(),
        v.velocity.to_bits(),
        v.glide.to_bits(),
        v.bend.to_bits(),
        v.gain0.to_bits(),
        v.gain1.to_bits(),
        v.inverse.to_bits(),
        v.fm_ratio.to_bits(),
        v.fm_index0.to_bits(),
        v.fm_index1.to_bits(),
        v.fm_rise,
    ]
}
fn pad_words(v: &voices::pad::Config) -> Vec<u32> {
    let mut w = Vec::new();
    w.extend(breath_words(&v.breath));
    w.push(v.gain.to_bits());
    w.push(v.gate.to_bits());
    w.push(v.spread.to_bits());
    w.push(v.motion.to_bits());
    w.extend(v.modulation.into_iter().map(f32::to_bits));
    w.push(v.pitch.to_bits());
    w.push(v.bright.to_bits());
    w.push(v.arc.to_bits());
    w.push(v.filter_mod.to_bits());
    w.push(v.panning.to_bits());
    w.push(v.sound_cut.to_bits());
    w.push(v.tuning.to_bits());
    w.push(v.stab_decay.to_bits());
    w.push(v.swell.to_bits());
    w.extend(v.breath_scale.into_iter().map(f32::to_bits));
    w.extend(v.vowels.into_iter().flatten().map(f32::to_bits));
    w.push(v.stabs);
    w.push(v.dx);
    w.push(v.filter as u32);
    w
}
fn bus_words(v: &voices::pad::BusConfig) -> Vec<u32> {
    vec![
        v.sound_cut.to_bits(),
        v.bright.to_bits(),
        v.arc.to_bits(),
        v.filter_mod.to_bits(),
        v.link_amount.to_bits(),
        v.link_env.to_bits(),
        v.gain0.to_bits(),
        v.gain1.to_bits(),
        v.inverse.to_bits(),
        v.start,
    ]
}
fn drums_words(v: &voices::DrumsConfig) -> Vec<u32> {
    let mut w = Vec::new();
    w.extend(v.gains.into_iter().map(f32::to_bits));
    w.extend(v.hat_inc.into_iter().map(f32::to_bits));
    w.push(v.drive.to_bits());
    w.push(v.drop.to_bits());
    w.push(v.click.to_bits());
    w.push(v.rumble.to_bits());
    w.push(v.sn_body.to_bits());
    w.push(v.sn_noise.to_bits());
    w.push(v.cl_gap.to_bits());
    w.push(v.h_noise.to_bits());
    w.extend(v.gains0.into_iter().map(f32::to_bits));
    w.extend(v.gains1.into_iter().map(f32::to_bits));
    w.push(v.inverse.to_bits());
    w
}
fn pluck_words(v: &voices::PluckConfig) -> Vec<u32> {
    vec![
        v.gain.to_bits(),
        v.pitch.to_bits(),
        v.ratio.to_bits(),
        v.index0.to_bits(),
        v.index1.to_bits(),
        v.gain0.to_bits(),
        v.gain1.to_bits(),
        v.inverse.to_bits(),
        v.old_level.to_bits(),
        v.rise,
        v.fm,
    ]
}
#[test]
fn all_block_configurations_match_independent_frozen_c() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("voice-config-native.json")).unwrap();
    let mut counts = [0u32; 6];
    let mut sounds = [false; 31];
    let mut destinations = [false; 8];
    let mut squash = [false; 3];
    for case in fixture["cases"].as_array().unwrap() {
        let params: config::Params = case["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap() as i16)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let raw = words(&case["raw"]);
        let expected = words(&case["config"]);
        let kind = case["kind"].as_u64().unwrap() as usize;
        counts[kind] += 1;
        let actual = match kind {
            0 => {
                let r = droneinput(&raw);
                destinations[params[fm1_types::ParamId::DL_DEST.index()] as usize] = true;
                squash[r.squash_target as usize] = true;
                drone_words(&config::drone(
                    &params,
                    &r,
                    &data::DRONE_SOUND,
                    &data::DRONE_FR,
                ))
            }
            1 => {
                let r = acidinput(&raw);
                let sound = params[fm1_types::ParamId::AC_WAVE.index()];
                if (0..31).contains(&sound) {
                    sounds[sound as usize] = true;
                }
                acid_words(&config::acid(
                    &params,
                    &r,
                    &data::ACID_SOUND,
                    &data::ACID_FM,
                ))
            }
            2 => {
                let r = padinput(&raw);
                pad_words(&config::pad(
                    &params,
                    &r,
                    &data::PAD_SOUND,
                    &scheduling_tables::SECTIONS,
                    &data::BREATH,
                    &data::VOWEL,
                ))
            }
            3 => bus_words(&config::pad_bus(
                &params,
                &padbusinput(&raw),
                &data::PAD_SOUND,
            )),
            4 => {
                let ids = words(&case["kits"]);
                drums_words(&config::drums(
                    &drumsinput(&raw),
                    [
                        &data::KITS[ids[0] as usize],
                        &data::KITS[ids[1] as usize],
                        &data::KITS[ids[2] as usize],
                        &data::KITS[ids[3] as usize],
                    ],
                ))
            }
            _ => pluck_words(&config::pluck(&params, &pluckinput(&raw), &data::CM_SOUND)),
        };
        assert_eq!(actual, expected, "configuration {}", case["test"]);
    }
    assert_eq!(counts, [256; 6]);
    assert!(sounds.into_iter().all(|v| v));
    assert!(destinations.into_iter().all(|v| v));
    assert!(squash.into_iter().all(|v| v));
}
