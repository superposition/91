// SPDX-License-Identifier: GPL-3.0-only
//! Chunk-rate motion and note-on coefficients over the existing fixed voices.
use super::{acid, data, drone, pad, Drum, Pluck};
use crate::{
    effects::math::{clamp, cos, exp, exp2, max, min, sin},
    sequencer::Rng,
};
const SR: f32 = 44_100.0;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Motion {
    pub value: f32,
    pub target: f32,
    pub rate: f32,
    pub left: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Lfo {
    pub phase: f32,
    pub value: f32,
    pub from: f32,
    pub to: f32,
}
pub fn motion(states: &mut [Motion; 10], frames: u32, rng: &mut Rng) {
    for s in states {
        if s.left <= frames {
            let seconds = 7.0 + 33.0 * rng.fraction();
            s.target = rng.fraction() * 2.0 - 1.0;
            s.left = (seconds * SR) as u32;
            s.rate = frames as f32 / (seconds * SR * 0.5);
        } else {
            s.left -= frames;
        }
        s.value += (s.target - s.value) * min(s.rate, 1.0);
    }
}
pub fn lfo(s: &mut Lfo, rate: i32, shape: i32, frames: f32, step: f32, rng: &mut Rng) {
    let rate = if (rate as u32) < 12 { rate as usize } else { 0 };
    s.phase += frames / (step * 4.0 * data::LFO_RATE_BEATS[rate]);
    if s.phase >= 1.0 {
        s.phase -= s.phase as i32 as f32;
        s.from = s.to;
        s.to = rng.fraction() * 2.0 - 1.0;
    }
    let p = s.phase;
    s.value = match shape {
        0 => sin(core::f32::consts::TAU * p),
        1 => {
            if p < 0.5 {
                4.0 * p - 1.0
            } else {
                3.0 - 4.0 * p
            }
        }
        2 => 2.0 * p - 1.0,
        3 => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        4 => s.to,
        _ => s.from + (s.to - s.from) * (0.5 - 0.5 * cos(core::f32::consts::PI * p)),
    };
}
pub fn time(kind: u32, v: i32) -> f32 {
    match kind {
        0 => 0.05 * exp2(v as f32 * 0.0764),
        1 => 8.0 * (v * v) as f32 * 0.0001,
        2 => 0.1 * exp2(v as f32 * 0.0691),
        3 => 6.0 * (v * v) as f32 * 0.0001,
        4 => 0.01 * exp2(v as f32 * 0.0864),
        _ => 0.05 * exp2(v as f32 * 0.0832),
    }
}
pub fn drone_envelope(s: &mut drone::State, frames: u32, attack: i32, decay: i32) {
    if s.env_up != 0 {
        s.env += frames as f32 / (time(4, attack) * SR);
        if s.env >= 1.0 {
            s.env = 1.0;
            s.env_up = 0;
        }
    } else {
        s.env *= exp(-6.9 * frames as f32 / (time(5, decay) * SR));
    }
}
fn mtof(note: f32, tuning: f32) -> f32 {
    440.0 * tuning * exp2((note - 69.0) * (1.0 / 12.0))
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Midi {
    pub status: u32,
    pub note: u32,
    pub velocity: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Events {
    pub len: u32,
    pub events: [Midi; 8],
}
impl Events {
    fn push(&mut self, status: u32, note: u32, velocity: u32) {
        self.events[self.len as usize] = Midi {
            status,
            note,
            velocity,
        };
        self.len += 1;
    }
}
#[repr(C)]
pub struct DrumConfig {
    pub decay: f32,
    pub tuning: f32,
    pub tune: f32,
    pub velocity: f32,
    pub kick_note: i32,
}
pub fn drum(states: &mut [Drum; 6], lane: usize, c: &DrumConfig, kits: [&data::Kit; 4]) {
    let tf = exp2(c.tune * (1.0 / 12.0));
    let v = &mut states[lane];
    v.fo = if v.on != 0 { v.xl } else { 0.0 };
    v.on = 1;
    v.amp = c.velocity;
    v.att = 0.0;
    v.t = 0.0;
    v.note = data::DRUM_GM[lane];
    let dec = c.decay;
    let [kk, ks, kh, kp] = kits;
    match lane {
        0 => {
            let f = mtof(c.kick_note as f32 + c.tune, c.tuning);
            v.inc = f * kk.k_start * (1.0 / SR);
            v.inc_end = f * (1.0 / SR);
            v.adec = exp(-6.9 / ((kk.k_dec0 + kk.k_dec1 * dec) * SR));
            v.ph = 0.0;
            v.idx = 1.0;
            v.x1 = kk.k_rumble;
            v.x2 = kk.k_rumble;
        }
        1 => {
            v.inc = ks.sn_f1 * tf * (1.0 / SR);
            v.inc2 = ks.sn_f2 * tf * (1.0 / SR);
            v.ph = 0.0;
            v.ph2 = 0.0;
            v.adec = exp(-6.9 / ((0.08 + 0.3 * dec) * ks.sn_dec * SR));
            v.hp = clamp(0.42 * tf, 0.15, 0.9);
        }
        2 => {
            v.inc = 185.0 * tf * (1.0 / SR);
            v.ph = 0.0;
            v.adec = exp(-6.9 / ((0.1 + 0.4 * dec) * ks.cl_dec * SR));
            v.inc_end = exp(-6.9 / (0.012 * SR));
            v.pend = c.velocity;
            v.hp = clamp(0.18 * tf, 0.08, 0.5);
        }
        3 => {
            v.adec = exp(-6.9 / ((0.015 + 0.1 * dec) * kh.h_dec * SR));
            v.hp = clamp(0.72 * tf, 0.4, 1.1);
            states[4].adec = exp(-6.9 / (0.02 * SR));
        }
        4 => {
            v.adec = exp(-6.9 / ((0.08 + 0.6 * dec) * kh.oh_dec * SR));
            v.hp = clamp(0.72 * tf, 0.4, 1.1);
        }
        _ => {
            let f = kp.p_freq * tf;
            v.inc = f * 2.0 * (1.0 / SR);
            v.inc_end = f * (1.0 / SR);
            v.inc2 = kp.p_ratio;
            v.idx = kp.p_idx;
            v.ph = 0.0;
            v.ph2 = 0.0;
            v.adec = exp(-6.9 / ((0.04 + 0.35 * dec) * SR));
        }
    }
}
pub fn pluck(s: &mut Pluck, note: i32, velocity: f32, decay: f32, tuning: f32) {
    s.inc = mtof(note as f32, tuning) * (1.0 / SR);
    s.ps = 0.0;
    s.env = 1.0;
    s.age = 0;
    s.vel = velocity;
    s.dm = exp(-6.9 / (decay * SR));
}
#[repr(C)]
pub struct AcidConfig {
    pub note: i32,
    pub slid_ok: u32,
    pub accent: u32,
    pub hat: u32,
    pub slide: u32,
    pub hat_amount: i32,
    pub decay: f32,
    pub step: f32,
    pub tuning: f32,
}
pub fn acid(s: &mut acid::State, c: &AcidConfig) -> Events {
    let slid = c.slid_ok != 0 && s.amp_t > 0.0;
    let mut events = Events::default();
    s.inc_t = mtof(c.note as f32, c.tuning) * (1.0 / SR);
    if !slid {
        s.inc = s.inc_t;
        s.env = 1.0;
        s.acc = if c.accent != 0 { 1.0 } else { 0.0 };
        if c.hat != 0 {
            s.acc = max(s.acc, c.hat_amount as f32 * 0.012);
        }
        s.amp_t = 1.0;
        s.edec = exp(-6.9 * 32.0 / (c.decay * if s.acc > 0.0 { 0.6 } else { 1.0 } * SR));
        if s.note != 0 {
            events.push(0x83, s.note as u32, 0);
        }
        events.push(0x93, c.note as u32, if s.acc > 0.0 { 120 } else { 90 });
        s.note = c.note as u8;
    }
    s.sliding = slid as u8;
    s.gate_left = if c.slide != 0 {
        -1
    } else {
        (0.55 * c.step) as i32
    };
    events
}
pub fn pad_handoff(states: &mut [pad::State; 4], oscillator: u8) {
    for v in states {
        if v.play != 0 {
            continue;
        }
        if v.amp > 0.002 && (v.midi_note as f32 != v.target || v.voices != oscillator) {
            v.hand = 1;
        } else if v.hand == 0 {
            v.note = v.target;
        }
    }
}
pub fn pad_leave(states: &mut [pad::State; 4], pcs: &[i32]) {
    for v in states {
        if v.play != 0 {
            continue;
        }
        let shared = pcs.contains(&((v.midi_note % 12) as i32));
        if v.amp > 0.002 && !shared && v.hand == 0 {
            v.hand = 1;
        } else if v.amp <= 0.002 && v.hand == 0 {
            v.note = v.target;
        }
    }
}
pub fn stab(
    states: &mut [pad::State; 4],
    oscillator: u8,
    level: f32,
    release: i32,
) -> (f32, Events) {
    let decay = exp(-6.9 * 32.0 / (0.08 * exp2(release as f32 * 0.04) * SR));
    let mut events = Events::default();
    for v in states {
        if v.play != 0 {
            continue;
        }
        if v.midi_note != 0 {
            events.push(0x80, v.midi_note as u32, 0);
        }
        v.note = v.target;
        v.hand = 0;
        v.hf = 1.0;
        v.voices = oscillator;
        v.midi_note = v.target as u8;
        events.push(0x90, v.midi_note as u32, (60.0 + 40.0 * level) as u32);
    }
    (decay, events)
}
fn fold(mut note: i32, lo: i32, hi: i32) -> i32 {
    while note < lo {
        note += 12;
    }
    while note > hi {
        note -= 12;
    }
    note
}
pub fn bass_to(s: &mut drone::State, pc: i32, octave: i32, line: bool, playing: bool) -> Events {
    let n = fold(36 + pc % 12 + 12 * octave, 24, 83);
    let mut events = Events::default();
    s.gtarget = n as f32;
    if s.play != 0 || line {
        return events;
    }
    s.target = n as f32;
    if playing && s.midi_note as i32 != n {
        events.push(0x82, s.midi_note as u32, 0);
        events.push(0x92, n as u32, 80);
    }
    s.midi_note = n as u8;
    events
}
pub fn bass_note(
    s: &mut drone::State,
    line: &mut drone::Line,
    note: i32,
    flags: i32,
    glide_knob: i32,
    length: i32,
    step: f32,
) -> (u8, Events) {
    let glide = flags & 2 != 0 && s.note >= 1.0 && line.env > 0.05;
    let n = fold(note, 24, 83);
    s.target = n as f32;
    if !glide && (glide_knob == 0 || s.note < 1.0) {
        s.note = n as f32;
    }
    let mut events = Events::default();
    events.push(0x82, s.midi_note as u32, 0);
    events.push(0x92, n as u32, if flags & 1 != 0 { 120 } else { 90 });
    s.midi_note = n as u8;
    s.velocity = if flags & 1 != 0 { 0.25 } else { 0.0 };
    if !glide {
        s.env_up = 1;
        line.up = 1;
    }
    line.gate = (0.25 + 0.0475 * length as f32) * step;
    (glide as u8, events)
}
pub fn pad_play(
    states: &mut [pad::State; 4],
    sequence: &mut u32,
    key: i32,
    on: bool,
    octave: i32,
    oscillator: u8,
) -> Events {
    let mut events = Events::default();
    if !on {
        for v in states {
            if v.play == 1 && v.pnote as i32 == key {
                v.play = 2;
                events.push(0x80, v.midi_note as u32, 0);
            }
        }
        return events;
    }
    let mut best = None;
    let mut bv = 1e9;
    for (i, v) in states.iter().enumerate() {
        if v.play == 0 && v.amp < bv {
            bv = v.amp;
            best = Some(i);
        }
    }
    if best.is_none() {
        bv = 1e9;
        for (i, v) in states.iter().enumerate() {
            if v.play == 2 && v.pamp < bv {
                bv = v.pamp;
                best = Some(i);
            }
        }
    }
    if best.is_none() {
        bv = 4e9;
        for (i, v) in states.iter().enumerate() {
            if (v.pord as f32) < bv {
                bv = v.pord as f32;
                best = Some(i);
            }
        }
    }
    // Valid engine states always have an order below4e9 after float conversion.
    let v = &mut states[best.expect("bounded pad allocation")];
    if v.midi_note != 0 {
        events.push(0x80, v.midi_note as u32, 0);
    }
    v.play = 1;
    v.pnote = key as u8;
    *sequence = sequence.wrapping_add(1);
    v.pord = *sequence;
    v.note = fold(key + 12 * octave, 24, 108) as f32;
    v.pamp = min(v.amp / 0.8, 1.0);
    v.hand = 0;
    v.hf = 1.0;
    v.voices = oscillator;
    v.midi_note = v.note as u8;
    events.push(0x90, v.midi_note as u32, 100);
    events
}
#[repr(C)]
pub struct KeyConfig {
    pub voice: i32,
    pub key: i32,
    pub velocity: i32,
    pub octaves: [i32; 3],
    pub oscillator: u32,
    pub playing: u32,
    pub bass_line: u32,
    pub acid_decay: i32,
    pub tuning: f32,
}
#[allow(clippy::too_many_arguments)]
pub fn key(
    pads: &mut [pad::State; 4],
    bass: &mut drone::State,
    acid: &mut acid::State,
    held: &mut [[u8; 8]; 3],
    counts: &mut [u8; 3],
    sequence: &mut u32,
    c: &KeyConfig,
) -> (u8, bool, Events) {
    let voice = c.voice as usize;
    let on = c.velocity > 0;
    let vg = c.velocity as f32 * 0.01;
    let n = &mut counts[voice];
    let h = &mut held[voice];
    if let Some(k) = h[..*n as usize].iter().position(|v| *v as i32 == c.key) {
        for j in k..*n as usize - 1 {
            h[j] = h[j + 1];
        }
        *n -= 1;
    }
    let legato = *n > 0;
    if on {
        if *n == 8 {
            h.copy_within(1..8, 0);
            *n = 7;
        }
        h[*n as usize] = c.key as u8;
        *n += 1;
    }
    let top = if *n != 0 {
        h[*n as usize - 1] as i32
    } else {
        -1
    };
    let mut events = Events::default();
    let mut glow = false;
    if voice == 0 {
        events = pad_play(pads, sequence, c.key, on, c.octaves[0], c.oscillator as u8);
    } else if voice == 1 {
        if top >= 0 {
            let m = fold(top - 24 + 12 * c.octaves[1], 24, 83);
            if on && !legato {
                bass.env_up = 1;
                bass.velocity = clamp(vg, 0.3, 1.2) - 1.0;
            }
            if bass.amp < 0.02 {
                bass.note = m as f32;
            }
            if bass.midi_note as i32 != m {
                events.push(0x82, bass.midi_note as u32, 0);
                events.push(
                    0x92,
                    m as u32,
                    if c.velocity != 0 {
                        c.velocity as u32
                    } else {
                        100
                    },
                );
            }
            bass.play = 1;
            bass.target = m as f32;
            bass.midi_note = m as u8;
        } else if bass.play != 0 {
            bass.play = 0;
            bass.velocity = 0.0;
            events = bass_to(
                bass,
                bass.gtarget as i32 % 12,
                c.octaves[1],
                c.bass_line != 0,
                c.playing != 0,
            );
        }
    } else if top >= 0 {
        let m = fold(top - 12 + 12 * c.octaves[2], 28, 96);
        acid.inc_t = mtof(m as f32, c.tuning) * (1.0 / SR);
        if legato && acid.play != 0 && acid.amp_t > 0.0 {
            acid.sliding = 1;
        } else {
            let dec = 0.06 * exp2(c.acid_decay as f32 * 0.05);
            acid.inc = acid.inc_t;
            acid.env = 1.0;
            acid.acc = if c.velocity >= 115 { 1.0 } else { 0.6 * vg };
            acid.edec = exp(-6.9 * 32.0 / (dec * SR));
            acid.sliding = 0;
        }
        if acid.note != 0 {
            events.push(0x83, acid.note as u32, 0);
        }
        events.push(
            0x93,
            m as u32,
            if c.velocity >= 115 {
                120
            } else {
                c.velocity as u32
            },
        );
        acid.note = m as u8;
        acid.amp_t = min(1.0, 0.3 + 0.7 * vg);
        acid.gate_left = -1;
        acid.play = 1;
        glow = true;
    } else if acid.play != 0 {
        acid.play = 0;
        acid.amp_t = 0.0;
        acid.gate_left = 0;
        acid.sliding = 0;
        if acid.note != 0 {
            events.push(0x83, acid.note as u32, 0);
        }
    }
    (counts[0] + counts[1] + counts[2], glow, events)
}
pub fn quiet(
    pads: &mut [pad::State; 4],
    bass: &mut drone::State,
    acid: &mut acid::State,
    drums: &mut [Drum; 6],
    counts: &mut [u8; 3],
) {
    *counts = [0; 3];
    for v in pads {
        v.play = 0;
        v.pamp = 0.0;
        v.amp = 0.0;
    }
    bass.play = 0;
    bass.amp = 0.0;
    acid.play = 0;
    acid.amp = 0.0;
    acid.amp_t = 0.0;
    for v in drums {
        v.on = 0;
        v.wait = 0;
        v.rn = 0;
    }
}
