// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-state melodic scheduling. Pitch/velocity decisions are values; voice
//! callbacks, MIDI emission and parameter-to-DSP coefficients stay at the seam.
use crate::sequencer::Rng;

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Harmony {
    pub steps: [i32; 12],
    pub pcs: [i32; 4],
    pub nsteps: i32,
    pub chord_count: i32,
    pub degree: i32,
    pub root: i32,
    pub song_octave: i32,
    pub pool: u32,
    pub pedal: u32,
}
pub fn fold(mut note: i32, low: i32, high: i32) -> i32 {
    while note > high {
        note -= 12;
    }
    while note < low {
        note += 12;
    }
    note
}
pub fn avoid(note: i32, h: &Harmony) -> i32 {
    if h.chord_count < 2 {
        return note;
    }
    let mut major = false;
    let mut minor = false;
    for k in 1..h.chord_count as usize {
        let d = (h.pcs[k] - h.pcs[0] + 12) % 12;
        major |= d == 4;
        minor |= d == 3;
    }
    if major && !minor && (note - h.pcs[0]).rem_euclid(12) == 5 && note + 2 <= 96 {
        note + 2
    } else {
        note
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Acid {
    pub offsets: [i8; 16],
    pub octaves: [i8; 16],
    pub gate: u32,
    pub accent: u32,
    pub slide: u32,
    pub position: i32,
    pub loops: u32,
    pub origin: u32,
    pub resync: u32,
    pub answer: u32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AcidConfig {
    pub length: i32,
    pub density: i32,
    pub range: i32,
    pub slide: i32,
    pub mutation: i32,
    pub change: i32,
    pub locked: i32,
    pub chance: i32,
    pub response: i32,
    pub every: i32,
    pub drift_change: i32,
    pub octave: i32,
    pub manual: i32,
    pub evolve: f32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AcidNote {
    pub action: u32,
    pub position: u32,
    pub note: i32,
    pub slid_ok: u32,
    pub accent: u32,
    pub slide: u32,
    pub beat: u32,
}
pub fn acid_roll(s: &mut Acid, c: &AcidConfig, h: &Harmony, step: usize, what: i32, rng: &mut Rng) {
    let density = c.density as f32 * 0.01;
    let mask = 1u32 << step;
    if what == 0 || what == 2 {
        if rng.fraction()
            < if step == 0 {
                (density + 0.3).min(1.0)
            } else {
                density
            }
        {
            s.gate |= mask;
        } else {
            s.gate &= !mask;
        }
    }
    if what <= 1 {
        s.offsets[step] = if rng.fraction() < 0.8 {
            2 * rng.index(h.chord_count as u32)
        } else {
            rng.index(h.nsteps as u32)
        } as i8;
        s.octaves[step] = if rng.fraction() < 0.55 {
            0
        } else {
            rng.index(c.range as u32)
        } as i8;
    }
    if what == 0 || what == 3 {
        if rng.fraction() < 0.3 {
            s.accent |= mask;
        } else {
            s.accent &= !mask;
        }
        if rng.fraction() < c.slide as f32 * 0.007 {
            s.slide |= mask;
        } else {
            s.slide &= !mask;
        }
    }
}
pub fn acid_mutate(s: &mut Acid, c: &AcidConfig, h: &Harmony, rng: &mut Rng) {
    if c.locked != 0 {
        return;
    }
    let k = c.mutation as f32 * 0.01;
    let amount = k * k
        + if c.mutation != 0 {
            0.05 * c.evolve
        } else {
            0.0
        };
    for step in 0..c.length as usize {
        if rng.fraction() < amount {
            acid_roll(
                s,
                c,
                h,
                step,
                if c.change > 0 && c.change < 4 {
                    c.change
                } else {
                    0
                },
                rng,
            );
        }
    }
}
pub fn acid_raw(s: &Acid, c: &AcidConfig, h: &Harmony, step: usize, shift: i32) -> i32 {
    let d = h.degree + s.offsets[step] as i32 + shift + h.nsteps * 8;
    let octave = d / h.nsteps - 8;
    fold(
        36 + h.root
            + h.steps[(d % h.nsteps) as usize]
            + 12 * (octave + s.octaves[step] as i32 + c.octave + h.song_octave),
        28,
        96,
    )
}
pub fn acid_source(step: i32, length: i32, kind: i32) -> i32 {
    if kind == 5 {
        length - 1 - step
    } else if kind == 6 {
        (step - 4).rem_euclid(length)
    } else {
        step
    }
}
pub fn acid_gate(s: &Acid, step: i32, length: i32, value: i32) -> bool {
    let kind = value / 5;
    let source = acid_source(step, length, kind) as u32;
    s.gate >> source & 1 != 0
        && !(kind == 7 && step & 1 != 0)
        && !(kind == 8 && s.accent >> source & 1 == 0)
}
pub fn acid_note(s: &Acid, c: &AcidConfig, h: &Harmony, step: usize, value: i32) -> i32 {
    let kind = value / 5;
    let direction = value % 5 - 2;
    let mut shift = 0;
    let mut note = if kind == 3 {
        acid_raw(s, c, h, step, 2 * direction)
    } else {
        acid_raw(s, c, h, step, 0)
    };
    if kind == 0 {
        return note;
    }
    if kind != 3 {
        if kind == 4 {
            let mut low = 127;
            let mut high = 0;
            for index in 0..c.length.clamp(1, 16) as usize {
                if s.gate >> index & 1 != 0 {
                    let n = acid_raw(s, c, h, index, 0);
                    low = low.min(n);
                    high = high.max(n);
                }
            }
            let centre = if high >= low { (low + high) / 2 } else { 60 };
            note = fold(2 * centre - note, 28, 96);
        }
        shift = if kind == 1 {
            12 * direction
        } else if kind == 2 {
            7 * direction
        } else if kind >= 4 {
            12 * direction
        } else {
            0
        };
    }
    if shift != 0 {
        let mut to = note + shift;
        if !(28..=96).contains(&to) {
            to = note - shift;
        }
        note = fold(to, 28, 96);
    }
    note
}
pub fn answer_due(
    response: i32,
    every: i32,
    count: u32,
    origin: u32,
    fill: bool,
    drift_change: i32,
) -> bool {
    response >= 5
        && if every >= 7 {
            fill || (drift_change == 0 && count.wrapping_sub(origin) % 8 == 7)
        } else {
            count.wrapping_sub(origin) % (every as u32 + 2) == every as u32 + 1
        }
}
pub fn acid_step(
    s: &mut Acid,
    c: &AcidConfig,
    h: &Harmony,
    bar_answer: u32,
    fill: bool,
    played: bool,
    held_gate: bool,
) -> AcidNote {
    let step = s.position;
    if step == 0 && s.resync != 0 {
        s.origin = s.loops;
        s.resync = 0;
    }
    if step == 0 && c.chance != 0 {
        s.answer = u32::from(answer_due(
            c.response,
            c.every,
            s.loops,
            s.origin,
            fill,
            c.drift_change,
        ));
    }
    let value = if if c.chance != 0 { s.answer } else { bar_answer } != 0 {
        c.response
    } else {
        0
    };
    let kind = value / 5;
    let source = acid_source(step, c.length, kind);
    let previous = if kind == 5 {
        (source + 1) % c.length
    } else {
        (source + c.length - 1) % c.length
    };
    let mut result = AcidNote {
        position: step as u32,
        ..AcidNote::default()
    };
    if !played && c.manual == 0 {
        if acid_gate(s, step, c.length, value) {
            result.action = 1;
            result.note = acid_note(s, c, h, source as usize, value);
            result.beat = u32::from(step & 3 == 0);
            result.slid_ok = (s.slide >> previous & 1) & (s.gate >> previous & 1);
            result.accent = s.accent >> source & 1;
            result.slide = s.slide >> source & 1;
        } else if held_gate {
            result.action = 2;
        }
    }
    result
}
pub fn acid_advance(s: &mut Acid, length: i32) -> bool {
    s.position += 1;
    if s.position >= length {
        s.position = 0;
        s.loops = s.loops.wrapping_add(1);
        true
    } else {
        false
    }
}
pub fn acid_capture(c: &AcidConfig, h: &Harmony, note: i32) -> (i32, i32) {
    let mut best = 999;
    let mut bo = 0;
    let mut bc = 0;
    for oct in 0..4 {
        for index in 0..12.min(2 * h.nsteps) {
            let d = h.degree + index + h.nsteps * 8;
            let n = 36
                + h.root
                + h.steps[(d % h.nsteps) as usize]
                + 12 * (d / h.nsteps - 8 + oct + c.octave + h.song_octave);
            let distance = (n - note).abs();
            if distance < best {
                best = distance;
                bo = index;
                bc = oct;
            }
        }
    }
    (bo, bc)
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Pluck {
    pub found: [u8; 32],
    pub gate: [u8; 32],
    pub index: [u8; 32],
    pub velocity: [u8; 32],
    pub intervals: [i8; 16],
    pub cursor: i32,
    pub slot: u32,
    pub sync: u32,
    pub motif: u32,
    pub last: i32,
    pub valid: u32,
    pub recording: u32,
    pub on: u32,
    pub inverted: u32,
    pub alternate: u32,
    pub restate: u32,
    pub first: u32,
    pub first_index: u32,
    pub previous: u32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PluckConfig {
    pub rate: i32,
    pub level: i32,
    pub manual: i32,
    pub range: i32,
    pub octave: i32,
    pub loop_size: i32,
    pub deja_vu: i32,
    pub density: i32,
    pub motif: i32,
    pub style: i32,
    pub leap: i32,
    pub locked: i32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PluckNote {
    pub fire: u32,
    pub note: i32,
    pub velocity: f32,
    pub slot: i32,
    pub publish: u32,
    pub rungs: [u8; 16],
    pub levels: [u8; 16],
}
pub fn ladder(h: &Harmony, range: i32, octave: i32) -> ([i32; 12], usize) {
    let mut arp = [0; 12];
    let mut n = 0;
    for o in 0..range.clamp(1, 3) {
        for k in 0..h.chord_count as usize {
            if n < 12 {
                arp[n] = 60 + 12 * (h.song_octave + octave) + h.pcs[k] + 12 * o;
                n += 1;
            }
        }
    }
    for k in 1..n {
        let t = arp[k];
        let mut o = k;
        while o > 0 && arp[o - 1] > t {
            arp[o] = arp[o - 1];
            o -= 1;
        }
        arp[o] = t;
    }
    (arp, n)
}
pub fn pluck_capture(s: &mut Pluck, c: &PluckConfig, h: &Harmony, slot: usize, note: i32) {
    s.found[slot] = 1;
    s.gate[slot] = u8::from(note >= 0);
    s.sync = 1;
    if note < 0 || h.chord_count < 1 {
        return;
    }
    let (arp, n) = ladder(h, c.range, c.octave);
    let mut best = 0;
    let mut leap = 0;
    let mut distance = 999;
    for (k, &a) in arp[..n].iter().enumerate() {
        for l in -1..=1 {
            let d = (a + 12 * l - note).abs();
            if d < distance {
                distance = d;
                best = k;
                leap = l;
            }
        }
    }
    s.index[slot] = (best
        | if leap > 0 {
            32
        } else if leap < 0 {
            64
        } else {
            0
        }
        | 128) as u8;
    s.velocity[slot] = 230;
}
pub fn pluck_step(
    s: &mut Pluck,
    c: &PluckConfig,
    h: &Harmony,
    step: i32,
    bars: u32,
    section: i32,
    rng: &mut Rng,
) -> PluckNote {
    let mut result = PluckNote {
        slot: -1,
        ..PluckNote::default()
    };
    let mut density = c.density as f32 * 0.01;
    if step == 0 && s.sync != 0 {
        s.slot = 0;
        s.cursor = 0;
        s.sync = 0;
    }
    if step == 0 {
        result.publish = 1;
        for k in 0..16 {
            let on = s.found[k] != 0 && s.gate[k] != 0;
            result.rungs[k] = s.index[k] & 15;
            result.levels[k] = if !on {
                0
            } else if s.velocity[k] < 140 {
                1
            } else if s.velocity[k] < 220 {
                2
            } else {
                3
            };
        }
        let phrase = bars & 7;
        if s.recording != 0 {
            s.valid = u32::from(s.motif != 0);
            s.recording = 0;
        }
        if c.motif == 0 {
            s.on = 0;
        } else if phrase == 0 {
            if s.valid == 0 || rng.fraction() * 100.0 < 70.0 {
                s.motif = 0;
                s.last = -1;
                s.recording = 1;
                s.on = 0;
            } else {
                s.on = 1;
                s.inverted = 0;
                s.alternate = 0;
                s.restate = 1;
            }
        } else {
            s.restate = u32::from(phrase == 4);
            s.on = u32::from(
                s.valid != 0 && (s.restate != 0 || rng.fraction() * 100.0 < c.motif as f32),
            );
            s.inverted = u32::from(s.on != 0 && s.restate == 0 && rng.fraction() < 0.2);
            s.alternate = u32::from(s.on != 0 && s.restate == 0 && rng.fraction() < 0.3);
        }
        s.first = 1;
    }
    let rate = [4, 3, 2, 1][c.rate.clamp(0, 3) as usize];
    if if rate == 3 {
        step % 3 != 0 || step > 12
    } else {
        step % rate != 0
    } {
        return result;
    }
    if c.level == 0 || c.manual != 0 || h.chord_count < 1 {
        return result;
    }
    let slot = (s.slot % (4u32 << c.loop_size.clamp(0, 3))) as usize;
    s.slot = s.slot.wrapping_add(1);
    result.slot = (slot & 15) as i32;
    let replay = c.deja_vu > 0 && s.found[slot] != 0 && rng.fraction() * 100.0 < c.deja_vu as f32;
    if replay {
        if s.gate[slot] == 0 || (section == 1 && rng.fraction() >= 0.4) {
            return result;
        }
    } else {
        if section == 1 {
            density *= 0.4;
        }
        let sound = if s.on != 0 && s.recording == 0 {
            s.motif >> step & 1 != 0
        } else {
            rng.fraction()
                < if s.cursor == 0 {
                    (density + 0.3).min(1.0)
                } else {
                    density
                }
        };
        s.found[slot] = 1;
        s.gate[slot] = u8::from(sound);
        if !sound {
            return result;
        }
    }
    let (arp, n) = ladder(h, c.range, c.octave);
    let n = n as i32;
    let mut leap = 0;
    let mut index;
    let velocity;
    if replay {
        index = (s.index[slot] & 15) as i32 % n;
        leap = if s.index[slot] & 32 != 0 {
            1
        } else if s.index[slot] & 64 != 0 {
            -1
        } else {
            0
        };
        velocity = s.velocity[slot] as f32 * (1.0 / 255.0);
    } else if s.on != 0 && s.recording == 0 {
        let mut interval = s.intervals[step as usize] as i32;
        if s.inverted != 0 {
            interval = -interval;
        }
        if s.alternate != 0 && step == s.last {
            interval = rng.index(5) as i32 - 2;
        }
        index = if s.first != 0 {
            if s.restate != 0 {
                s.first_index as i32
            } else {
                s.previous as i32
            }
        } else {
            s.previous as i32 + interval
        };
        index = index.rem_euclid(n);
        s.first = 0;
        velocity = 0.75 + 0.25 * rng.fraction();
        s.index[slot] = (index | 128) as u8;
        s.velocity[slot] = (velocity * 255.0) as u8;
    } else {
        index = match c.style {
            1 => {
                let q = n - 1 - s.cursor % n;
                s.cursor = s.cursor.wrapping_add(1);
                q
            }
            2 => {
                let period = if n > 1 { 2 * n - 2 } else { 1 };
                let q = s.cursor % period;
                s.cursor = s.cursor.wrapping_add(1);
                if q < n {
                    q
                } else {
                    period - q
                }
            }
            3 => {
                let q = rng.index(n as u32) as i32;
                s.cursor = s.cursor.wrapping_add(1);
                q
            }
            4 => {
                let q = s.cursor % n;
                s.cursor = s.cursor.wrapping_add(2);
                q
            }
            5 => {
                let q = s.cursor % n;
                s.cursor = s.cursor.wrapping_add(3);
                q
            }
            _ => {
                let q = s.cursor % n;
                s.cursor = s.cursor.wrapping_add(1);
                q
            }
        };
        if c.deja_vu > 0 && s.index[slot] & 128 != 0 && rng.fraction() < 0.5 {
            index = ((s.index[slot] & 15) as i32 % n
                + if rng.fraction() < 0.5 { 1 } else { n - 1 })
                % n;
        }
        if c.leap != 0 && rng.fraction() * 100.0 < c.leap as f32 {
            leap = if rng.fraction() < 0.7 { 1 } else { -1 };
        }
        velocity = 0.7 + 0.3 * rng.fraction();
        s.index[slot] = (index
            | if leap > 0 {
                32
            } else if leap < 0 {
                64
            } else {
                0
            }
            | 128) as u8;
        s.velocity[slot] = (velocity * 255.0) as u8;
    }
    if s.recording != 0 {
        s.intervals[step as usize] = if s.motif != 0 {
            index - s.previous as i32
        } else {
            0
        } as i8;
        if s.motif == 0 {
            s.first_index = index as u32;
        }
        s.motif |= 1u32 << step;
        s.last = step;
    }
    if bars & 7 == 7 && step >= 12 && c.locked == 0 {
        let mut best = index;
        let mut distance = 99;
        for k in 0..n {
            if (arp[k as usize] - h.pcs[0]).rem_euclid(12) == 0 {
                let d = (k - index).abs();
                if d < distance {
                    distance = d;
                    best = k;
                }
            }
        }
        index = best;
        s.index[slot] = (s.index[slot] & 0xf0) | index as u8;
    }
    s.previous = index as u32;
    result.fire = 1;
    result.note = fold(arp[index as usize] + 12 * leap, 36, 100);
    result.velocity = velocity;
    result
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Bass {
    pub steps: [u8; 16],
    pub history: [u8; 16],
    pub set: u32,
    pub beat: u32,
    pub position: i32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BassConfig {
    pub mode: i32,
    pub manual: i32,
    pub played: i32,
    pub playing: i32,
    pub section: i32,
    pub keep: i32,
    pub root_mask: u32,
    pub riff_mask: u32,
    pub intervals: [i32; 4],
    pub movement: i32,
    pub octave: i32,
    pub drop: i32,
    pub kick: i32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BassNote {
    pub fire: u32,
    pub note: i32,
    pub flags: u32,
    pub publish: u32,
}
pub fn bass_semis(k: i32, h: &Harmony) -> i32 {
    let n = if h.nsteps > 0 { h.nsteps } else { 7 };
    let root = if h.pedal & 1 != 0 { 0 } else { h.degree };
    let d = root + k;
    h.steps[(d % n) as usize] + 12 * (d / n) - (h.steps[(root % n) as usize] + 12 * (root / n))
}
pub fn bass_capture(s: &mut Bass) {
    s.steps = s.history;
    if !s.steps.iter().any(|n| *n != 0) {
        s.steps[0] = 1 | 1 << 6;
    }
    s.set = 1;
}
pub fn bass_record(s: &mut Bass, step: i32, note: i32, root: i32, h: &Harmony) {
    let mut interval = note - root;
    let mut octave = 1;
    while interval < 0 && octave > 0 {
        interval += 12;
        octave -= 1;
    }
    while interval >= 12 && octave < 2 {
        interval -= 12;
        octave += 1;
    }
    let mut best = 0;
    let mut distance = 99;
    for k in 0..8 {
        let d = (bass_semis(k, h) - interval).abs();
        if d < distance {
            distance = d;
            best = k;
        }
    }
    s.history[(step & 15) as usize] = (1 | best << 3 | octave << 6) as u8;
}
pub fn bass_below(root: i32, h: &Harmony) -> i32 {
    for d in 1..=2 {
        if h.pool >> ((root - d - h.root).rem_euclid(12)) & 1 != 0 {
            return root - d;
        }
    }
    root - 1
}
pub fn bass_fifth(h: &Harmony) -> i32 {
    for k in 1..h.chord_count as usize {
        let d = (h.pcs[k] - h.pcs[0] + 12) % 12;
        if d == 7 {
            return 7;
        }
        if d == 6 || d == 8 {
            return d;
        }
    }
    if h.pool >> ((h.pcs[0] + 7 - h.root).rem_euclid(12)) & 1 != 0 {
        7
    } else {
        6
    }
}
fn natural_degree(k: i32, h: &Harmony) -> i32 {
    h.steps[(k % 7) as usize]
}
pub fn bass_step(s: &mut Bass, c: &BassConfig, h: &Harmony, step: i32, rng: &mut Rng) -> BassNote {
    let mut result = BassNote::default();
    let mode = c.mode;
    if mode == 0 || c.played != 0 || c.manual != 0 || c.playing == 0 || h.nsteps == 0 {
        return result;
    }
    if step == 0 {
        s.beat = 0;
    }
    if c.section == 1 && c.keep == 0 {
        return result;
    }
    let pc = if h.pedal & 1 != 0 {
        h.root % 12
    } else {
        h.pcs[0]
    };
    let root = fold(36 + pc % 12 + 12 * (c.octave + h.song_octave), 24, 83);
    if mode == 16 {
        if s.set == 0 {
            bass_capture(s);
            result.publish = 1;
        }
        s.position = step;
        let value = s.steps[(step & 15) as usize] as i32;
        let kind = value & 7;
        if kind == 0 || kind > 4 {
            return result;
        }
        result.fire = 1;
        result.note = root + bass_semis(value >> 3 & 7, h) + 12 * ((value >> 6 & 3) - 1);
        result.flags = u32::from(kind == 2 || kind == 4) | (u32::from(kind >= 3) << 1);
        return result;
    }
    s.position = -1;
    let mut hit = match mode {
        1 => c.root_mask >> step & 1 != 0,
        2 => c.kick != 0 && c.section != 2,
        3 => step & 1 == 0,
        4 => step & 3 == 2,
        5 => step & 3 == 0,
        _ => c.riff_mask >> step & 1 != 0,
    };
    if c.drop != 0 && step == 0 {
        hit = true;
    }
    if !hit {
        s.history[(step & 15) as usize] = 0;
        return result;
    }
    let mut note;
    if mode == 5 {
        let three = if h.pedal & 1 != 0 {
            root + natural_degree(2, h)
        } else if h.chord_count > 1 {
            root + (h.pcs[1] - h.pcs[0] + 12) % 12
        } else {
            root + 3
        };
        let five = if h.pedal & 1 != 0 {
            root + natural_degree(4, h)
        } else {
            root + bass_fifth(h)
        };
        note = root;
        if step >> 2 == 1 {
            note = if rng.fraction() < 0.5 { three } else { five };
        } else if step >> 2 == 2 {
            note = five;
        } else if step >> 2 == 3 && rng.fraction() * 100.0 < c.movement as f32 + 20.0 {
            note = if rng.fraction() < 0.5 {
                root - 1
            } else {
                bass_below(root, h)
            };
        }
    } else {
        let mut interval = if mode >= 6 {
            c.intervals[(s.beat & 3) as usize]
        } else {
            0
        };
        if rng.fraction() * 100.0 < c.movement as f32 {
            let r = rng.fraction();
            interval = if r < 0.5 {
                12
            } else if r < 0.8 {
                bass_fifth(h)
            } else {
                bass_below(root, h) - root
            };
        }
        note = root + interval;
    }
    bass_record(s, step, note, root, h);
    result.fire = 1;
    result.note = note;
    s.beat = (s.beat + 1) & 255;
    result
}

pub fn source(request: i32, current: i32) -> u32 {
    if request != 0 {
        (request - 1) as u32
    } else {
        current as u32
    }
}
pub fn build_due(section: i32, bars: &mut i32) -> bool {
    if section != 2 {
        return false;
    }
    *bars -= 1;
    *bars <= 0
}
pub fn groove_bar(section: i32, bars: u32) -> u32 {
    if section == 0 && bars < 255 {
        bars + 1
    } else {
        bars
    }
}
pub fn wobble(flags: u32, section: i32, groove: u32, bars: u32) -> i32 {
    if flags & 2 != 0 && section == 0 && (1..=8).contains(&groove) {
        [8, 10, 9, 1][(bars.wrapping_mul(3).wrapping_add(groove) % 4) as usize]
    } else {
        -1
    }
}
#[allow(clippy::too_many_arguments)]
pub fn dj_action(
    sync: bool,
    section: i32,
    playing: bool,
    frozen: bool,
    pending: bool,
    harmonic_bars: i32,
    left: i32,
    total: f32,
    remaining: f32,
    step_length: f32,
) -> u32 {
    if !sync || section != 0 || !playing || frozen {
        0
    } else if pending {
        1
    } else if if harmonic_bars != 0 {
        left == 1
    } else {
        total > 0.0 && remaining <= step_length * 16.0
    } {
        2
    } else {
        0
    }
}
pub fn snare_rise(section: i32, length: i32, left: i32, step: i32) -> f32 {
    if section == 2 {
        7.0 * ((length - left) * 16 + step) as f32 / (length * 16) as f32
    } else {
        0.0
    }
}
pub fn perc_fall(hold: bool, fill: bool, drift: bool, section: i32, kind: u32, step: i32) -> f32 {
    if (hold || fill || (drift && section == 0)) && kind == 7 && step >= 8 {
        -0.9 * (step - 8) as f32
    } else {
        0.0
    }
}
#[allow(clippy::too_many_arguments)]
pub fn pad_step(
    jam: bool,
    stabs: bool,
    manual: bool,
    playing: bool,
    mask: u32,
    step: i32,
    section: i32,
    comp: i32,
    evolve: f32,
    rng: &mut Rng,
) -> f32 {
    if jam || !stabs || manual || !playing {
        return 0.0;
    }
    if mask >> step & 1 != 0
        || (section != 1
            && mask == 0x4444
            && comp == 0
            && (step == 7 || step == 15)
            && rng.fraction() < 0.35 * evolve)
    {
        (if step & 1 != 0 { 0.7 } else { 1.0 }) * (if section == 1 { 0.8 } else { 1.0 })
    } else {
        0.0
    }
}
pub fn acid_link(jam: bool, linked: bool, perc: bool, section: i32, hit: bool) -> bool {
    !jam && (!(linked && perc && section == 0) || hit)
}
pub fn harmonic_due(jam: bool, step: i32, pending: bool, frozen: bool) -> bool {
    !jam && step == 0 && pending && !frozen
}
pub fn drift_arm(every: i32, change: i32, mutation: i32, countdown: i32) -> i32 {
    if every == 0 && (change != 0 || mutation != 0) && countdown < 0 {
        2
    } else {
        countdown
    }
}
#[allow(clippy::too_many_arguments)]
pub fn harmonic_advance(
    jam: bool,
    step: i32,
    playing: bool,
    frozen: bool,
    length: i32,
    left: &mut i32,
    step_length: f32,
) -> Option<(bool, f32, f32)> {
    if jam || step != 0 || !playing || frozen || length == 0 {
        return None;
    }
    if *left > 0 {
        *left -= 1;
    }
    Some((
        *left == 0,
        length as f32 * 16.0 * step_length,
        (*left + 1) as f32 * 16.0 * step_length,
    ))
}

/// Advance the sample-based drift clock. Bar-based harmony clamps at zero and
/// waits for the bar callback; free drift publishes one pending change.
pub fn harmonic_samples(pending: bool, harmonic: bool, left: &mut f32, frames: u32) -> bool {
    if pending && !harmonic {
        return true;
    }
    *left -= frames as f32;
    if *left <= 0.0 {
        if harmonic {
            *left = 0.0;
        } else {
            return true;
        }
    }
    pending
}

pub fn acid_regrow(s: &mut Acid, c: &AcidConfig, h: &Harmony, rng: &mut Rng) {
    for step in 0..16 {
        acid_roll(s, c, h, step, 0, rng);
    }
}
#[allow(clippy::too_many_arguments)]
pub fn acid_edit(
    s: &mut Acid,
    step: usize,
    offset: i32,
    octave: i32,
    gate: bool,
    accent: bool,
    slide: bool,
) {
    let mask = 1u32 << step;
    s.offsets[step] = offset as i8;
    s.octaves[step] = octave as i8;
    s.gate = if gate { s.gate | mask } else { s.gate & !mask };
    s.accent = if accent {
        s.accent | mask
    } else {
        s.accent & !mask
    };
    s.slide = if slide {
        s.slide | mask
    } else {
        s.slide & !mask
    };
}
pub fn pluck_edit(s: &mut Pluck, step: usize, rung: i32, level: i32) {
    s.found[step] = 1;
    s.gate[step] = u8::from(level > 0);
    s.index[step] = (rung.min(11) | 128) as u8;
    s.velocity[step] = if level <= 1 {
        110
    } else if level == 2 {
        190
    } else {
        255
    };
}
pub fn midi_slot(notes: &[u8; 8], waits: &[i32; 8], note: i32) -> u32 {
    let mut off = 0u32;
    let mut slot = None;
    let mut oldest = None;
    for k in 0..8 {
        let active = notes[k] != 0 && notes[k] as i32 != note;
        if notes[k] as i32 == note {
            off |= 1 << k;
        }
        if !active {
            if slot.is_none() {
                slot = Some(k);
            }
        } else if oldest.is_none_or(|previous| waits[k] < waits[previous]) {
            oldest = Some(k);
        }
    }
    let slot = slot.unwrap_or_else(|| {
        let oldest = oldest.expect("eight occupied gates have an oldest");
        off |= 1 << oldest;
        oldest
    });
    off | ((slot as u32) << 8)
}
pub fn midi_due(present: bool, wait: &mut i32, frames: i32) -> bool {
    if !present {
        return false;
    }
    *wait -= frames;
    *wait <= 0
}
pub fn clear_pending(waits: &mut [i32; 16], ratchets: &mut [u32; 6]) {
    waits.fill(0);
    ratchets.fill(0);
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Start {
    pub song_position: u32,
    pub sixteenth: u32,
    pub bars: u32,
    pub acid_position: i32,
    pub chain: u32,
    pub slice: u32,
    pub pattern: i32,
}
pub fn start(s: &mut Start, next: i32) {
    s.song_position = 0;
    s.sixteenth = 0;
    s.bars = 0;
    s.acid_position = 0;
    s.chain = 0;
    s.slice = 0;
    s.pattern = next;
}
pub fn transport_message(external: bool, from_top: bool, stop: bool) -> u32 {
    if external {
        0
    } else if stop {
        0xfc
    } else if from_top {
        0xfa
    } else {
        0xfb
    }
}

/// Queued grid actions are emitted in the established order. The C callbacks
/// consume their own effect state; none changes the remaining arm flags.
pub fn tick_actions(scatter: u32, reverse: u32, exit: u32, repeat: u32) -> [u32; 5] {
    [
        if scatter != 0 { 1 } else { 0 },
        if reverse != 0 { 2 } else { 0 },
        3,
        if exit != 0 { 4 } else { 0 },
        if repeat != 0 {
            5 | ((repeat - 1) << 8)
        } else {
            6
        },
    ]
}
pub fn bass_edit(s: &mut Bass, command: u32, value: u32) -> bool {
    if command & 0x20 != 0 {
        if s.set == 0 {
            bass_capture(s);
            return true;
        }
        return false;
    }
    s.steps[(command & 15) as usize] = value as u8;
    if command & 0x10 == 0 {
        s.set = 1;
    } else if value & 0x100 != 0 && !s.steps.iter().any(|n| *n != 0) {
        s.set = 0;
    }
    true
}
pub fn pluck_parameter(s: &mut Pluck, operation: u32, value: i32) {
    if operation == 0 {
        if value != 0 {
            s.sync = 1;
        } else {
            s.found.fill(0);
            s.slot = 0;
            s.valid = 0;
            s.on = 0;
            s.recording = 0;
        }
    } else if operation == 1 && value != 0 {
        s.sync = 1;
    }
}
pub fn acid_length(s: &mut Acid, length: i32) {
    if s.position >= length {
        s.position = 0;
    }
}
pub fn answer_origin(count: u32, origin: u32, resync: bool) -> u32 {
    if resync {
        count
    } else {
        origin
    }
}
pub fn acid_preview(s: &Acid, c: &AcidConfig, h: &Harmony, step: i32, value: i32) -> i32 {
    let mut c = *c;
    c.length = c.length.clamp(1, 16);
    let step = if (0..16).contains(&step) { step } else { 0 };
    if step >= c.length || h.nsteps < 1 || !acid_gate(s, step, c.length, value) {
        -1
    } else {
        acid_note(
            s,
            &c,
            h,
            acid_source(step, c.length, value / 5) as usize,
            value,
        )
    }
}
