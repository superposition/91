// SPDX-License-Identifier: GPL-3.0-only
// Ported from jam.h and song_plan.inc. Candidate order and RNG calls are musical behavior.
use fm1_formats::jam::{self, JamPack};
pub use fm1_types::JamSettings as Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    Settings,
    Reference,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Beat {
    pub notes: [u8; 4],
    pub node: u8,
    pub passing: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Phrase {
    pub bars: u8,
    pub role: u8,
    pub motif: u8,
    pub section: u8,
    pub beats: [Beat; 64],
}

impl Phrase {
    pub const EMPTY: Self = Self {
        bars: 0,
        role: 0,
        motif: 0,
        section: 0,
        beats: [Beat {
            notes: [0; 4],
            node: 0,
            passing: 0,
        }; 64],
    };
    pub fn active_beats(&self) -> &[Beat] {
        &self.beats[..usize::from(self.bars.min(16)) * 4]
    }
}

const _: [(); 6] = [(); core::mem::size_of::<Beat>()];
const _: [(); 388] = [(); core::mem::size_of::<Phrase>()];
const _: [(); 10] = [(); core::mem::size_of::<Settings>()];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(u32);
impl Seed {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn state(self) -> u32 {
        self.0
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

fn choose(pack: JamPack<'_>, node: usize, seed: &mut Seed, role: u8) -> usize {
    let p = pack.bytes();
    let mut sum = 0;
    for i in 0..pack.node_count() {
        sum += p[jam::EDGE + node * 16 + i] as u32
            * (1 + ((p[jam::NODE + i * 8 + 5] >> role) & 1) as u32);
    }
    let mut r = (seed.next_u32() >> 8) % sum;
    for i in 0..pack.node_count() {
        let weight = p[jam::EDGE + node * 16 + i] as u32
            * (1 + ((p[jam::NODE + i * 8 + 5] >> role) & 1) as u32);
        if r < weight {
            return i;
        }
        r -= weight;
    }
    node
}

pub fn plan_into(
    pack: JamPack<'_>,
    settings: &Settings,
    section: u32,
    previous: Option<&Phrase>,
    out: &mut Phrase,
    seed: &mut Seed,
) -> Result<(), PlanError> {
    settings.validate().map_err(|_| PlanError::Settings)?;
    let p = pack.bytes();
    let form = &p[jam::FORM + settings.form as usize * 32..];
    out.bars = settings.bars;
    out.section = (section % form[16] as u32) as u8;
    out.role = form[17 + out.section as usize];
    if pack.version() == 1 {
        plan_v1(pack, settings, previous, out, seed);
    } else {
        plan_v2(pack, settings, section, previous, out, seed)?;
    }
    Ok(())
}

fn last_notes(previous: Option<&Phrase>, default: [i32; 4]) -> [i32; 4] {
    match previous {
        Some(prev) if (8..=16).contains(&prev.bars) => {
            prev.beats[prev.bars as usize * 4 - 1].notes.map(i32::from)
        }
        _ => default,
    }
}

fn plan_v1(
    pack: JamPack<'_>,
    s: &Settings,
    previous: Option<&Phrase>,
    out: &mut Phrase,
    seed: &mut Seed,
) {
    let p = pack.bytes();
    let base = if s.minor != 0 { 12 } else { 9 };
    let mut node = p[base] as usize;
    let mut rung = 0usize;
    let mut last = last_notes(previous, [48, 52, 55, 57]);
    out.motif = match previous {
        Some(prev) => prev.motif % p[6],
        None => ((seed.next_u32() >> 16) % p[6] as u32) as u8,
    };
    for bar in 0..s.bars as usize {
        if bar == s.bars as usize - 3 {
            node = p[base + 1] as usize;
        } else if bar == s.bars as usize - 2 {
            node = p[base + 2] as usize;
        } else if bar == s.bars as usize - 1 || bar == 0 {
            node = p[base] as usize;
        } else if s.bars == 8 || bar & 1 == 0 {
            node = choose(pack, node, seed, out.role);
        }
        for b in 0..4 {
            let a = &p[jam::NODE + node * 8..];
            let scale = &p[jam::SCALE + a[2] as usize * 8..];
            let movement =
                s.movement == 3 || (s.movement == 2 && b & 1 == 0) || (s.movement == 1 && b == 3);
            if b == 0 {
                rung = 0;
            } else if movement {
                rung = (rung
                    + if bar < (s.bars / 2) as usize || out.role != 2 {
                        1
                    } else {
                        7
                    })
                    % 8;
            }
            let mut best = 10_000;
            let mut chosen = [48, 52, 55, 57];
            for inv in 0..4 {
                for oct in 3..6 {
                    let mut notes = [0; 4];
                    let mut cost = 0;
                    for i in 0..4 {
                        let d = (rung & 1) + 2 * inv + 2 * i;
                        let v = s.key as i32
                            + a[1] as i32
                            + 12 * oct
                            + scale[d % 8] as i32
                            + 12 * (d / 8) as i32;
                        notes[i] = v;
                        cost += (v - last[i]).abs();
                        if !(48..=76).contains(&v) {
                            cost += 1000;
                        }
                    }
                    if inv != rung / 2 {
                        cost += 9;
                    }
                    if cost < best {
                        best = cost;
                        chosen = notes;
                    }
                }
            }
            last = chosen;
            out.beats[bar * 4 + b] = Beat {
                notes: chosen.map(|v| v as u8),
                node: node as u8,
                passing: (rung & 1) as u8,
            };
        }
    }
}

fn plan_v2(
    pack: JamPack<'_>,
    s: &Settings,
    section: u32,
    previous: Option<&Phrase>,
    out: &mut Phrase,
    seed: &mut Seed,
) -> Result<(), PlanError> {
    let p = pack.bytes();
    let head = &p[jam::V2_HEAD..];
    let mut last = last_notes(previous, [52, 55, 59, 62]);
    out.motif = (head[12] + u8::from(head[11] != 0 && section % 4 == 2)) % p[6];
    if head[11] >= 2 && section % 4 == 3 {
        out.motif = ((seed.next_u32() >> 16) % p[6] as u32) as u8;
    }
    for bar in 0..s.bars as usize {
        for b in 0..4 {
            let part = usize::from(out.role == 1 || out.role == 2);
            let offset = ((s.minor as usize * 2 + usize::from(s.bars == 16)) * 2 + part) * 32
                + bar * 2
                + b / 2;
            let mut node = p[jam::V2_CHART + offset] as usize;
            if part == 0 && out.section != 0 && bar == s.bars as usize - 1 && b >= 2 {
                node = p[if s.minor != 0 { 12 } else { 9 }] as usize;
            }
            let a = pack.node(node).ok_or(PlanError::Reference)?;
            let scale = &p[jam::SCALE + a[2] as usize * 8..];
            let mut borrow = 0u8;
            let mut diminished = 0u8;
            if s.movement != 0 && (b == 1 || (s.movement >= 2 && b == 3)) {
                borrow = a[6] & if bar & 1 != 0 { 1 } else { 8 };
                if head[11] == 0 {
                    borrow = 0;
                }
                if s.movement == 3 && bar % 4 == 3 && b == 3 && bar + 1 < s.bars as usize {
                    diminished = 1;
                    borrow = 0;
                }
            }
            let mut best = 100_000;
            let mut chosen = [48, 52, 55, 57];
            for inv in 0..4 {
                let mut shape = [0; 4];
                for (i, value) in shape.iter_mut().enumerate() {
                    let voice = (inv + i) & 3;
                    let d = voice * 2
                        + if diminished != 0 {
                            1
                        } else {
                            ((borrow >> voice) & 1) as usize
                        };
                    *value =
                        s.key as i32 + a[1] as i32 + scale[d] as i32 + 12 * ((inv + i) / 4) as i32;
                }
                let low = ((48 - shape[0] + 11) / 12).max(3);
                let high = ((72 - shape[3]) / 12).min(5);
                for oct in low..=high {
                    let mut notes = [0; 4];
                    let mut cost = 0;
                    for i in 0..4 {
                        notes[i] = shape[i] + 12 * oct;
                        cost += (notes[i] - last[i]).abs() * 3;
                    }
                    cost += (notes[3] - (64 + (s.key % 5) as i32 + if part != 0 { 2 } else { 0 }))
                        .abs();
                    if cost < best {
                        best = cost;
                        chosen = notes;
                    }
                }
            }
            last = chosen;
            out.beats[bar * 4 + b] = Beat {
                notes: chosen.map(|v| v as u8),
                node: node as u8,
                passing: borrow | diminished << 4,
            };
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BassEvent {
    pub note: i16,
    pub velocity: u8,
    pub gate: u8,
}

pub fn bass_event(
    pack: JamPack<'_>,
    s: &Settings,
    phrase: &Phrase,
    next: &Phrase,
    bar: usize,
    step: usize,
) -> Result<BassEvent, PlanError> {
    s.validate().map_err(|_| PlanError::Settings)?;
    if phrase.bars == 0 || phrase.bars > 16 || bar >= phrase.bars as usize || step >= 16 {
        return Err(PlanError::Reference);
    }
    let p = pack.bytes();
    let v2 = pack.version() == 2;
    let pattern =
        (phrase.motif as usize + s.bass as usize + usize::from(bar == phrase.bars as usize - 1))
            % p[6] as usize;
    let node = phrase.beats[bar * 4 + if v2 { step / 4 } else { 0 }].node as usize;
    let a = pack.node(node).ok_or(PlanError::Reference)?;
    let b = &p[jam::BASS + pattern * 36..];
    let interval = b[step] as i8;
    let root = (s.key as i32 + a[0] as i32) % 12;
    let mut note = 36 + root;
    if interval == -128 || b[16 + step] == 0 {
        return Ok(BassEvent {
            note: -1,
            velocity: b[16 + step],
            gate: b[32],
        });
    }
    if interval == -127 {
        let target = if v2 && step < 8 {
            phrase.beats[bar * 4 + 2].node
        } else if bar + 1 < phrase.bars as usize {
            phrase.beats[(bar + 1) * 4].node
        } else {
            next.beats[0].node
        };
        let target_node = pack.node(target as usize).ok_or(PlanError::Reference)?;
        note = 36 + (s.key as i32 + target_node[0] as i32) % 12 - 1;
        while note - (36 + root) > 6 {
            note -= 12;
        }
        while (36 + root) - note > 6 {
            note += 12;
        }
    } else if interval == -126 {
        note += if matches!(a[3], 1 | 2 | 4 | 5) { 3 } else { 4 };
    } else if interval == -125 {
        note += if matches!(a[3], 2 | 5) {
            6
        } else if v2 && a[3] == 7 {
            8
        } else {
            7
        };
    } else {
        note += interval as i32;
    }
    while note < if v2 { 28 } else { 31 } {
        note += 12;
    }
    while note > if v2 { 52 } else { 55 } {
        note -= 12;
    }
    Ok(BassEvent {
        note: note as i16,
        velocity: b[16 + step],
        gate: b[32],
    })
}
