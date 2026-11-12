// SPDX-License-Identifier: GPL-3.0-only
//! Four procedural DJ/snippet voices over caller-owned state and mix buffers.
use super::{blep_saw, sine, sine_any, Mix};
use crate::{
    effects::math::{abs, clamp, exp, exp2, flush, max, min, tanh},
    sequencer::Rng,
};
const SR: f32 = 44_100.0;
const HORN: u8 = 0;
const SIREN: u8 = 1;
const LASER: u8 = 2;
const ZAP: u8 = 3;
const BEEP: u8 = 4;
const DOUBLE: u8 = 5;
const WHOOP: u8 = 6;
const DUB: u8 = 7;
const DROP: u8 = 8;
const RISER: u8 = 9;
const REWIND: u8 = 10;
const CRASH: u8 = 11;
const STAB: u8 = 12;
const BELL: u8 = 13;
const ROLL: u8 = 14;
const BLAST: u8 = 15;
const RAVE: u8 = 16;
const RAVE_STAB: u8 = 17;
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct State {
    pub on: u8,
    pub held: u8,
    pub key: u8,
    pub kind: u8,
    pub hold_mode: u8,
    pub padding: [u8; 3],
    pub hz: f32,
    pub aux: f32,
    pub time: f32,
    pub phase: f32,
    pub phase2: f32,
    pub phase3: f32,
    pub envelope: f32,
    pub z: f32,
    pub z2: f32,
    pub age: u32,
}
#[derive(Clone, Copy)]
pub struct Recipe {
    pub kind: u8,
    pub hz: f32,
    pub aux: f32,
}
const fn recipe(kind: u8, hz: f32, aux: f32) -> Recipe {
    Recipe { kind, hz, aux }
}
pub const RECIPES: [Recipe; 27] = [
    recipe(HORN, 466.0, 0.0),
    recipe(BLAST, 466.0, 0.0),
    recipe(HORN, 466.0, 1.0),
    recipe(RAVE, 233.0, 0.0),
    recipe(SIREN, 900.0, 1.2),
    recipe(SIREN, 1100.0, 3.0),
    recipe(SIREN, 700.0, 0.4),
    recipe(LASER, 2400.0, 0.06),
    recipe(LASER, 1400.0, 0.1),
    recipe(ZAP, 3000.0, 0.15),
    recipe(ZAP, 6000.0, 0.08),
    recipe(BEEP, 1000.0, 0.1),
    recipe(BEEP, 600.0, 0.12),
    recipe(BEEP, 1700.0, 0.08),
    recipe(DOUBLE, 1200.0, 0.07),
    recipe(WHOOP, 300.0, 0.35),
    recipe(WHOOP, 800.0, 0.6),
    recipe(DUB, 1400.0, 0.9),
    recipe(DUB, 900.0, 1.6),
    recipe(DROP, 140.0, 1.2),
    recipe(DROP, 100.0, 2.4),
    recipe(RISER, 0.0, 3.0),
    recipe(REWIND, 2200.0, 0.7),
    recipe(CRASH, 0.0, 0.9),
    recipe(RAVE_STAB, 233.0, 0.0),
    recipe(BELL, 880.0, 0.0),
    recipe(ROLL, 0.0, 2.5),
];
const NAME_BANK:&[u8]=b"AIR HORN\0AIR BLAST\0HORN MIN\0RAVE HORN\0SIREN\0SIREN 2\0SIREN 3\0LASER\0LASER LO\0ZAP\0ZAP HI\0BEEP\0BEEP LO\0BEEP HI\x002 BEEP\0WHOOP\0WHOOP 2\0DUB SIREN\0DUB LOW\0DROP\0DROP LONG\0RISER\0REWIND\0CRASH\0RAVE STAB\0BELL\0SNR ROLL\0";
const fn offsets() -> [u16; 27] {
    let mut out = [0; 27];
    let mut i = 0;
    let mut n = 1;
    while i < NAME_BANK.len() {
        if NAME_BANK[i] == 0 && n < 27 {
            out[n] = (i + 1) as u16;
            n += 1;
        }
        i += 1;
    }
    out
}
const NAME_OFFSETS: [u16; 27] = offsets();
pub fn name(key: i32) -> &'static [u8] {
    let i = if (key as u32) < 27 { key as usize } else { 0 };
    let start = NAME_OFFSETS[i] as usize;
    let end = if i < 26 {
        NAME_OFFSETS[i + 1] as usize
    } else {
        NAME_BANK.len()
    };
    &NAME_BANK[start..end]
}
/// Returns the slot for accepted note-ons; note-offs only release held flags.
pub fn note(
    states: &mut [State; 4],
    age: &mut u32,
    key: i32,
    on: bool,
    octave: i32,
    hold: bool,
) -> Option<usize> {
    if !(0..27).contains(&key) {
        return None;
    }
    if !on {
        for s in states {
            if s.on != 0 && s.key as i32 == key {
                s.held = 0;
            }
        }
        return None;
    }
    let mut slot = states.iter().position(|s| s.on == 0);
    if slot.is_none() {
        let mut best = 0;
        for i in 1..4 {
            if states[i].age < states[best].age {
                best = i;
            }
        }
        slot = Some(best);
    }
    let slot = slot.unwrap();
    let s = &mut states[slot];
    let r = RECIPES[key as usize];
    s.on = 1;
    s.held = 1;
    s.key = key as u8;
    s.kind = r.kind;
    s.hold_mode = hold as u8;
    s.hz = r.hz
        * if octave == 1 {
            2.0
        } else if octave == 2 {
            0.5
        } else {
            1.0
        };
    s.aux = r.aux;
    s.time = 0.0;
    s.phase = 0.0;
    s.phase2 = 0.0;
    s.phase3 = 0.0;
    s.envelope = 0.0;
    s.z = 0.0;
    s.z2 = 0.0;
    *age = age.wrapping_add(1);
    s.age = *age;
    Some(slot)
}
fn wrap(p: f32) -> f32 {
    p - p as i32 as f32
}
pub fn sample(v: &mut State, rng: &mut Rng, table: &[f32; 1025]) -> (f32, f32) {
    let t = v.time * (1.0 / SR);
    let inc = v.hz * (1.0 / SR);
    let n = rng.fraction() * 2.0 - 1.0;
    let y;
    let mut a = 1.0;
    match v.kind {
        HORN | BLAST => {
            let blast = v.kind == BLAST;
            let r2 = if v.aux > 0.5 { 1.189 } else { 1.26 };
            let e1 = exp(-t * 30.0);
            let tl = if t > 0.30 { (t - 0.30) * 4.0 } else { 0.0 };
            let tail = if tl > 1.0 { 1.0 } else { tl };
            let mut scale = (1.0 - 0.06 * e1) * (1.0 + 0.0015 * sine(table, wrap(t * 6.3)));
            scale *= if blast {
                1.0 - 0.05 * tail * tail
            } else {
                1.0 - 0.012 * (1.0 - exp(-t * 0.7))
            };
            let i1 = inc * scale * 0.9965;
            let i2 = inc * r2 * scale;
            let i3 = inc * 1.498 * scale * 1.0035;
            let mut x = (blep_saw(v.phase, i1) - blep_saw(wrap(v.phase + 0.4), i1))
                + (blep_saw(v.phase2, i2) - blep_saw(wrap(v.phase2 + 0.4), i2)) * 0.9
                + (blep_saw(v.phase3, i3) - blep_saw(wrap(v.phase3 + 0.4), i3)) * 0.85
                + blep_saw(wrap(v.phase * 2.0), 2.0 * i1) * 0.25;
            v.phase = wrap(v.phase + i1);
            v.phase2 = wrap(v.phase2 + i2);
            v.phase3 = wrap(v.phase3 + i3);
            x += n * (0.25 * exp(-t * 45.0) + 0.02);
            x *= 0.33;
            let u = 2.4 * x + 0.25;
            x = u / (1.0 + abs(u)) - 0.2;
            let f = 0.26 + 0.06 * (1.0 - e1) - if blast { 0.14 * tail } else { 0.0 };
            v.z += f * v.z2;
            let hp = x - v.z - 0.45 * v.z2;
            v.z2 += f * hp;
            y = 0.35 * x + 0.45 * v.z + 1.1 * v.z2;
            if blast {
                a = (if t < 0.006 { t * 166.0 } else { 1.0 })
                    * (if t < 0.32 {
                        1.0
                    } else {
                        exp(-(t - 0.32) * 12.0)
                    });
                if t > 0.9 {
                    v.on = 0;
                }
            } else {
                a = if t * 80.0 < 1.0 { t * 80.0 } else { 1.0 };
            }
        }
        RAVE | RAVE_STAB => {
            let stab = v.kind == RAVE_STAB;
            let e1 = exp(-t * if stab { 28.0 } else { 14.0 });
            let scale = 1.0 + if stab { 0.35 } else { 0.50 } * e1;
            let rate = if stab { 7.0 } else { 4.6 };
            let i1 = inc * scale * 1.0105;
            let i2 = inc * scale * 0.9895;
            let w1 = 0.50 + 0.28 * sine(table, wrap(t * rate));
            let w2 = 0.50 - 0.28 * sine(table, wrap(t * rate + 0.25));
            let f = if stab {
                0.08 + 0.45 * exp(-t * 18.0)
            } else {
                0.10 + 0.40 * exp(-t * 9.0)
            };
            let mut x = (blep_saw(v.phase, i1) - blep_saw(wrap(v.phase + w1), i1))
                + (blep_saw(v.phase2, i2) - blep_saw(wrap(v.phase2 + w2), i2))
                + 0.7 * (blep_saw(v.phase3, i1 * 0.5) - blep_saw(wrap(v.phase3 + 0.5), i1 * 0.5));
            v.phase = wrap(v.phase + i1);
            v.phase2 = wrap(v.phase2 + i2);
            v.phase3 = wrap(v.phase3 + i1 * 0.5);
            x *= 0.4;
            v.z += f * v.z2;
            let hp = x - v.z - 0.5 * v.z2;
            v.z2 += f * hp;
            let u = 3.0 * v.z;
            y = u / (1.0 + abs(u)) * 1.4;
            if stab {
                a = (if t < 0.003 { t * 333.0 } else { 1.0 }) * exp(-t * 9.0);
                if t > 0.45 {
                    v.on = 0;
                }
            } else {
                a = if t * 200.0 < 1.0 { t * 200.0 } else { 1.0 };
            }
        }
        SIREN => {
            let lf = wrap(t * v.aux);
            let tri = if lf < 0.5 {
                lf * 4.0 - 1.0
            } else {
                3.0 - lf * 4.0
            };
            v.phase = wrap(v.phase + inc * (1.0 + 0.5 * tri));
            y = 0.6 * sine(table, v.phase) + 0.25 * sine(table, wrap(v.phase * 2.0));
        }
        LASER => {
            v.phase = wrap(v.phase + (v.hz * exp(-t / v.aux) + 150.0) * (1.0 / SR));
            y = 0.5 * tanh(3.0 * sine(table, v.phase));
            a = exp(-t * 4.0);
            if t > 0.5 {
                v.on = 0;
            }
        }
        ZAP => {
            let k = 0.04 + 0.6 * exp(-t / v.aux);
            v.z += k * (n - v.z);
            v.z2 += k * (v.z - v.z2);
            y = (v.z - v.z2) * 3.0;
            a = exp(-t * 7.0 / v.aux * 0.5);
            if t > 0.5 {
                v.on = 0;
            }
        }
        BEEP | DOUBLE => {
            let len = v.aux;
            let g1 = if t < len { 1.0 } else { 0.0 };
            let g2 = if v.kind == DOUBLE && t > len * 1.9 && t < len * 2.9 {
                1.0
            } else {
                0.0
            };
            v.phase = wrap(v.phase + inc);
            y = sine(table, v.phase) * 0.7;
            a = g1 + g2;
            if t > len * 3.2 {
                v.on = 0;
            }
        }
        WHOOP => {
            let u = min(t / v.aux, 1.0);
            v.phase = wrap(v.phase + inc * exp2(2.585 * u));
            y = 0.55 * sine(table, v.phase) + 0.2 * sine(table, wrap(v.phase * 2.0));
            a = if u < 1.0 {
                1.0
            } else {
                max(0.0, 1.0 - (t - v.aux) / 0.15)
            };
            if t > v.aux + 0.16 {
                v.on = 0;
            }
        }
        DUB => {
            let cycle = t / v.aux;
            let fraction = cycle - cycle as i32 as f32;
            v.phase = wrap(v.phase + inc * exp2(-2.5 * fraction));
            let x = if v.phase < 0.5 { 0.4 } else { -0.4 };
            v.z += 0.35 * (x - v.z);
            y = v.z;
        }
        DROP => {
            v.phase = wrap(v.phase + (v.hz * exp(-t * 3.0) + 28.0) * (1.0 / SR));
            y = 0.9 * sine(table, v.phase);
            a = exp(-t / (v.aux * 0.5));
            if t > v.aux * 2.0 {
                v.on = 0;
            }
        }
        RISER => {
            let u = min(t / v.aux, 1.0);
            v.z += (0.03 + 0.9 * u * u) * (n - v.z);
            y = v.z * 3.0;
            a = 0.08 + 0.9 * u * u;
        }
        REWIND => {
            let u = min(t / v.aux, 1.0);
            let f = v.hz * exp2(-5.0 * u) + 40.0;
            v.phase = wrap(v.phase + f * (1.0 / SR));
            let x = blep_saw(v.phase, f * (1.0 / SR)) * 0.5;
            v.z += 0.3 * (x - v.z);
            y = v.z;
            a = 1.0 - u * u;
            if t > v.aux {
                v.on = 0;
            }
        }
        CRASH => {
            v.z += 0.7 * (n - v.z);
            y = (n - v.z) * 0.9;
            a = exp(-t / (v.aux * 0.35));
            if t > v.aux * 2.0 {
                v.on = 0;
            }
        }
        STAB => {
            let k = 0.5 * exp(-t * 6.0) + 0.04;
            let x = (blep_saw(v.phase, inc)
                + blep_saw(v.phase2, inc * 1.189)
                + blep_saw(v.phase3, inc * 1.498))
                * 0.4;
            v.phase = wrap(v.phase + inc);
            v.phase2 = wrap(v.phase2 + inc * 1.189);
            v.phase3 = wrap(v.phase3 + inc * 1.498);
            v.z += k * (x - v.z);
            y = v.z;
            a = exp(-t * 7.0);
            if t > 0.7 {
                v.on = 0;
            }
        }
        BELL => {
            v.phase = wrap(v.phase + inc);
            v.phase2 = wrap(v.phase2 + inc * 1.4);
            y = 0.7 * sine_any(table, v.phase + 2.0 * exp(-t * 3.0) * sine(table, v.phase2));
            a = exp(-t * 2.2);
            if t > 3.0 {
                v.on = 0;
            }
        }
        _ => {
            let rate = 7.0 + 18.0 * min(t / v.aux, 1.0);
            v.phase3 = wrap(v.phase3 + rate * (1.0 / SR));
            let beat = v.phase3;
            v.z += 0.25 * (n - v.z);
            y = (n - v.z) * 0.9 + 0.3 * sine(table, wrap(v.phase + 0.0));
            v.phase = wrap(v.phase + 190.0 * (1.0 / SR));
            a = exp(-beat * 7.0);
        }
    }
    v.time += 1.0;
    (y, a)
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Config {
    pub mix: i32,
    pub echo: i32,
    pub gain0: f32,
    pub gain1: f32,
    pub inverse: f32,
}
pub struct Result {
    pub active: u32,
    pub samples: u32,
}
pub fn process(
    states: &mut [State; 4],
    c: &Config,
    rng: &mut Rng,
    table: &[f32; 1025],
    b: Mix<'_>,
) -> Result {
    let sends = [[0.0, 0.0], [0.25, 0.35], [0.6, 0.9]];
    let ec = c.echo.clamp(0, 2) as usize;
    let gain = 0.01 * clamp(c.mix as f32, 0.0, 100.0) * 0.5;
    let mut active = 0;
    let mut samples = 0;
    for v in states {
        if v.on == 0 {
            continue;
        }
        for j in 0..b.left.len() {
            if v.on == 0 {
                break;
            }
            let (mut y, a) = sample(v, rng, table);
            samples += 1;
            let sustained = matches!(v.kind, HORN | RAVE | SIREN | DUB | RISER | ROLL);
            let gate = !sustained
                || (v.held != 0 && v.hold_mode != 0)
                || (v.hold_mode == 0 && v.time < 1.2 * SR);
            v.envelope +=
                ((if gate { 1.0 } else { 0.0 }) - v.envelope) * if gate { 0.01 } else { 0.00025 };
            y = y * a * v.envelope * gain * (c.gain0 + (c.gain1 - c.gain0) * j as f32 * c.inverse);
            b.left[j] += y;
            b.right[j] += y;
            b.reverb[j] += y * sends[ec][0];
            b.echo[j] += y * sends[ec][1];
            if sustained && !gate && v.envelope < 0.003 {
                v.on = 0;
            }
            v.z = flush(v.z);
            v.z2 = flush(v.z2);
        }
        active += v.on as u32;
    }
    Result { active, samples }
}
