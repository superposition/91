// SPDX-License-Identifier: GPL-3.0-only
//! Runtime pieces and saved banks have separate representations. Platform I/O
//! owns no project pointer; all state and scratch is fixed caller storage.
use crate::factory;
use fm1_formats::{
    objects::{bank_header, GlobalParts},
    storage::{LegacyRecord, Record, RecordValues, BANK_LEN, BANK_TAIL},
};
use fm1_types::{GenreId, ParamId as P, PieceId, PARAMETERS};
pub const PIECES: usize = 10;
pub const GENRES: usize = 19;
pub const EXT_LEN: usize = 8 + GENRES * PIECES * 16;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub parameters: [i16; 176],
    pub pool: u16,
    pub patterns: [[u16; 6]; 6],
    pub modifiers: [[u8; 16]; 6],
    pub acid: [u8; 18],
    pub pluck: [u8; 12],
    pub progression: [u8; 6],
    pub kinds: [u8; 3],
    pub bass: [u8; 16],
}
impl Slot {
    pub const ZERO: Self = Self {
        parameters: [0; 176],
        pool: 0,
        patterns: [[0; 6]; 6],
        modifiers: [[0; 16]; 6],
        acid: [0; 18],
        pluck: [0; 12],
        progression: [0; 6],
        kinds: [0; 3],
        bass: [0; 16],
    };
    pub fn recipe(genre: usize, piece: usize) -> Self {
        let (g, p) = (GenreId::new(genre).unwrap(), PieceId::new(piece).unwrap());
        let preset = factory::preset(g, p);
        let pats = factory::patterns(g, p);
        let mods = factory::modifiers(g, p);
        Self {
            parameters: preset.parameters,
            pool: preset.pool,
            patterns: pats.map(|row| row.map(|v| v.bits())),
            modifiers: mods.map(|row| row.map(|v| v.bits())),
            ..Self::ZERO
        }
    }
    fn values(&self) -> RecordValues {
        let mut patterns = [0; 36];
        let mut modifiers = [0; 96];
        for (i, v) in patterns.iter_mut().enumerate() {
            *v = self.patterns[i / 6][i % 6];
        }
        for (i, v) in modifiers.iter_mut().enumerate() {
            *v = self.modifiers[i / 16][i % 16];
        }
        RecordValues {
            parameters: self.parameters,
            pool: self.pool,
            patterns,
            modifiers,
            acid: self.acid,
            pluck: self.pluck,
        }
    }
    fn decoded(&mut self, v: RecordValues) {
        self.parameters = v.parameters;
        self.pool = v.pool;
        for (i, n) in v.patterns.iter().enumerate() {
            self.patterns[i / 6][i % 6] = *n;
        }
        for (i, n) in v.modifiers.iter().enumerate() {
            self.modifiers[i / 16][i % 16] = *n;
        }
        self.acid = v.acid;
        self.pluck = v.pluck;
    }
    fn answer_old(&mut self) {
        let n = self.parameters[P::AC_RESP.index()];
        self.parameters[P::AC_RESP.index()] = [0, 8, 13, 22, 37, 6, 11]
            .get(n as usize)
            .copied()
            .unwrap_or(0);
        self.parameters[P::STYLE.index()] = 0;
    }
    fn pluck_old(&mut self) {
        let v = &mut self.parameters[P::CM_SND.index()];
        if *v >= 34 {
            *v = v.wrapping_add(8);
        }
    }
    fn style_old(&mut self) {
        let s = self.parameters[P::STYLE.index()];
        if !(1..5).contains(&s) {
            return;
        }
        for p in [P::KIT_KICK, P::KIT_SNARE, P::KIT_HAT, P::KIT_PERC] {
            self.parameters[p.index()] = s;
        }
        let pad = [-1, 6, 7, -1, -1][s as usize];
        if pad >= 0 {
            self.parameters[P::PADTONE.index()] = pad;
        }
        self.parameters[P::DR_INST.index()] = [0, 1, 2, 0, 3][s as usize];
        if s == 3 {
            self.parameters[P::AC_WAVE.index()] = 2;
        }
        self.parameters[P::STYLE.index()] = 0;
    }
}
#[repr(C)]
pub struct Runtime {
    pub parameters: [i16; 176],
    pub pool: u16,
    pub acid: [u8; 18],
    pub pluck: [u8; 12],
    pub progression: [u8; 6],
    pub kinds: [u8; 3],
    pub bass: [u8; 16],
    pub leds: u8,
    pub piece: u8,
    pub genre: u8,
    pub edited: u16,
    pub slots: [Slot; 10],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Globals {
    pub magic: u32,
    pub values: [i16; 4],
    pub list: [u8; 16],
    pub play: u8,
    pub reserved: [u8; 3],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Patterns {
    pub patterns: [[u16; 6]; 6],
    pub modifiers: [[u8; 16]; 6],
}
impl Patterns {
    pub const ZERO: Self = Self {
        patterns: [[0; 6]; 6],
        modifiers: [[0; 16]; 6],
    };
}
pub trait Objects {
    fn load(&mut self, object: u32, output: &mut [u8]) -> i32;
    fn save(&mut self, object: u32, input: &[u8]) -> i32;
}
pub struct Project<'a> {
    /// Engine application effect, replayed only after the exclusive state borrow.
    pub recipe_effect: Option<u8>,
    pub runtime: &'a mut Runtime,
    pub image: &'a mut [u8; 3840],
    pub saved: &'a mut u16,
    pub pending: &'a mut u8,
    pub pending_globals: &'a mut u8,
    pub globals: &'a mut Globals,
    pub from_old: &'a mut i32,
    pub extras: &'a mut [[u8; 16]; 10],
    pub live_extras: &'a mut [[u8; 8]; 10],
    pub pending_extras: &'a mut u8,
    pub bass_dirty: &'a mut u16,
}
fn u16at(b: &[u8], n: usize) -> u16 {
    u16::from_le_bytes([b[n], b[n + 1]])
}
fn magic(b: &[u8], m: &[u8; 4]) -> bool {
    b.len() >= 4 && &b[..4] == m
}
pub fn object(genre: u8) -> u32 {
    if genre == 0 {
        0
    } else {
        4 + u32::from(genre)
    }
}
pub fn normalize(value: i32, count: i32) -> u8 {
    if !(1..=256).contains(&count) {
        return 0;
    }
    value.rem_euclid(count) as u8
}
pub fn list_find(list: &[u8; 16], position: u8, genre: u8, piece: u8) -> u8 {
    let e = genre as usize * 10 + piece as usize + 1;
    if (position as usize) < 16 && list[position as usize] as usize == e {
        position
    } else {
        list.iter()
            .position(|v| *v as usize == e)
            .map_or(255, |n| n as u8)
    }
}
pub fn list_step(list: &[u8; 16], from: i32, direction: i32) -> i32 {
    if ![-1, 1].contains(&direction) || from < 0 {
        return -1;
    }
    let mut at = if from < 16 {
        from
    } else if direction > 0 {
        15
    } else {
        0
    };
    for _ in 0..16 {
        at = (at + direction + 16) % 16;
        if at < 0 {
            return -1;
        }
        if list[at as usize] != 0 {
            return at;
        }
    }
    -1
}
impl Project<'_> {
    pub fn switch_genre(&mut self, io: &mut impl Objects, genre: i32, patterns: &mut Patterns) {
        let genre = normalize(genre, 19);
        if genre == self.runtime.genre {
            return;
        }
        if *self.pending != 0 || *self.pending_extras != 0 || *self.bass_dirty != 0 {
            self.flush(io);
        }
        self.runtime.genre = genre;
        self.image_load(io);
        self.runtime.edited = 0;
        self.take(patterns);
    }
    pub fn place_set(&mut self, place: u32) {
        self.globals.values[3] =
            ((self.globals.values[3] as u16 & 256) | (place as u16 & 255)) as i16;
        self.globals_sync();
    }
    pub fn random_set(&mut self, on: bool) {
        self.globals.values[3] =
            ((self.globals.values[3] as u16 & 255) | if on { 256 } else { 0 }) as i16;
        self.globals_sync();
    }
    pub fn apply_lines(&self) -> u32 {
        u32::from(
            self.runtime.parameters[P::AC_LOCK.index()] != 0
                && self.runtime.acid.iter().any(|v| *v != 0),
        ) | (u32::from(
            self.runtime.parameters[P::CM_LOCK.index()] != 0
                && self.runtime.pluck.iter().any(|v| *v != 0),
        ) << 1)
    }
    pub fn keep(&mut self, patterns: &Patterns) {
        let p = &mut self.runtime;
        let s = &mut p.slots[p.piece as usize];
        s.parameters = p.parameters;
        s.pool = p.pool;
        s.patterns = patterns.patterns;
        s.modifiers = patterns.modifiers;
        s.progression = p.progression;
        s.kinds = p.kinds;
    }
    pub fn take(&mut self, patterns: &mut Patterns) {
        let p = &mut self.runtime;
        let s = &p.slots[p.piece as usize];
        for (i, v) in p.parameters.iter_mut().enumerate() {
            if !(73..=75).contains(&i) {
                *v = s.parameters[i];
            }
        }
        p.pool = s.pool;
        patterns.patterns = s.patterns;
        patterns.modifiers = s.modifiers;
        p.acid = s.acid;
        p.pluck = s.pluck;
        p.progression = s.progression;
        p.kinds = s.kinds;
        p.bass = s.bass;
    }
    pub fn header(&mut self) {
        bank_header(
            self.runtime.genre,
            176,
            *self.saved,
            [1, 1, 1, 0],
            self.image,
        )
        .unwrap();
    }
    pub fn pack(&mut self, piece: usize) {
        let s = &self.runtime.slots[piece];
        self.image[BANK_TAIL + piece * 6..BANK_TAIL + piece * 6 + 6]
            .copy_from_slice(&s.progression);
        for i in 0..3 {
            if self.extras[piece][i] != s.kinds[i] {
                self.extras[piece][i] = s.kinds[i];
                *self.pending_extras = 1;
            }
        }
        s.values()
            .encode_legacy(&mut self.image[16 + piece * 376..16 + (piece + 1) * 376])
            .unwrap();
    }
    pub fn extras_load(&mut self, io: &mut impl Objects) {
        let mut b = [0; EXT_LEN];
        let n = io.load(23, &mut b).max(0) as usize;
        self.extras.fill([0; 16]);
        self.live_extras.fill([0; 8]);
        let g = self.runtime.genre as usize;
        if n >= 8 && magic(&b, b"PCX1") && b[4] == 16 {
            let at = 8 + g * 160;
            if n >= at + 160 {
                for (i, v) in self.extras.iter_mut().enumerate() {
                    v.copy_from_slice(&b[at + i * 16..at + (i + 1) * 16]);
                }
            }
        } else if n >= 4 && magic(&b, b"KND1") {
            for (i, v) in self.extras.iter_mut().enumerate() {
                let at = 4 + (g * 10 + i) * 3;
                if n >= at + 3 {
                    v[..3].copy_from_slice(&b[at..at + 3]);
                }
            }
        }
    }
    pub fn extras_save(&mut self, io: &mut impl Objects) -> i32 {
        let mut b = [0; EXT_LEN];
        let n = io.load(23, &mut b).max(0) as usize;
        if !(n >= 8 && magic(&b, b"PCX1") && b[4] == 16) {
            let kn = if n >= 4 && magic(&b, b"KND1") {
                (n - 4) / 3
            } else {
                0
            };
            for k in (0..190).rev() {
                let mut t = [0; 3];
                if k < kn {
                    t.copy_from_slice(&b[4 + k * 3..7 + k * 3]);
                }
                b[8 + k * 16..8 + (k + 1) * 16].fill(0);
                b[8 + k * 16..11 + k * 16].copy_from_slice(&t);
            }
        } else if n < EXT_LEN {
            b[n..].fill(0);
        }
        b[..8].copy_from_slice(b"PCX1\x10\x13\0\0");
        let at = 8 + self.runtime.genre as usize * 160;
        for (i, v) in self.extras.iter().enumerate() {
            b[at + i * 16..at + (i + 1) * 16].copy_from_slice(v);
        }
        io.save(23, &b)
    }
    pub fn bass_load(&mut self, io: &mut impl Objects) {
        let mut b = [0; EXT_LEN];
        let n = io.load(25, &mut b).max(0) as usize;
        *self.bass_dirty = 0;
        if n < 8 || !magic(&b, b"BSQ1") || b[4] != 16 {
            return;
        }
        for (i, s) in self.runtime.slots.iter_mut().enumerate() {
            let at = 8 + (self.runtime.genre as usize * 10 + i) * 16;
            if *self.saved >> i & 1 != 0 && n >= at + 16 {
                s.bass.copy_from_slice(&b[at..at + 16]);
            }
        }
    }
    pub fn bass_save(&mut self, io: &mut impl Objects) -> i32 {
        let mut b = [0; EXT_LEN];
        let n = io.load(25, &mut b).max(0) as usize;
        let mut diff = false;
        if !(n >= 8 && magic(&b, b"BSQ1") && b[4] == 16) {
            b.fill(0);
        } else if n < EXT_LEN {
            b[n..].fill(0);
            diff = true;
        }
        b[..8].copy_from_slice(b"BSQ1\x10\x13\0\0");
        for i in 0..10 {
            if *self.bass_dirty >> i & 1 != 0 {
                let at = 8 + (self.runtime.genre as usize * 10 + i) * 16;
                for k in 0..16 {
                    let v = if *self.saved >> i & 1 != 0 {
                        self.runtime.slots[i].bass[k]
                    } else {
                        0
                    };
                    diff |= b[at + k] != v;
                    b[at + k] = v;
                }
            }
        }
        *self.bass_dirty = 0;
        if diff {
            io.save(25, &b)
        } else {
            0
        }
    }
    fn legacy_load(&mut self, n: usize) {
        let b = &self.image[..n];
        let np = if n >= 20 { u16at(b, 8) as usize } else { 0 };
        let stride = np * 2 + 74;
        if n >= 20
            && magic(b, b"HYP2")
            && b[4..8] == [1, 0, 0, 0]
            && (1..=176).contains(&np)
            && n >= 20 + 10 * stride
        {
            let mut saved = u16at(b, 10);
            let ver = u16at(b, 18);
            self.globals.values[..3].copy_from_slice(&[
                u16at(b, 12) as i16,
                u16at(b, 14) as i16,
                u16at(b, 16) as i16,
            ]);
            *self.from_old = 1;
            if ver < 2 {
                let theme = &mut self.globals.values[2];
                *theme = if *theme >= 5 {
                    0
                } else {
                    theme.wrapping_add(1)
                };
            }
            for (i, s) in self.runtime.slots.iter_mut().enumerate() {
                if saved >> i & 1 == 0 {
                    continue;
                }
                let v = LegacyRecord::parse(&b[20 + i * stride..20 + (i + 1) * stride], np as u16)
                    .unwrap()
                    .decode(ver < 3, ver == 0);
                s.parameters = v.parameters;
                s.pool = v.pool;
                for (j, v) in v.patterns.iter().enumerate() {
                    s.patterns[j / 6][j % 6] = *v;
                }
                s.style_old();
                s.answer_old();
                s.pluck_old();
                if !v.valid {
                    saved &= !(1 << i);
                }
            }
            *self.saved = saved;
        } else if n >= 132 && magic(b, b"HYP1") {
            let info = P::THEME.info();
            self.globals.values[2] = (b[116] as i16).clamp(info.min, info.max);
            *self.from_old = 1;
        }
    }
    pub fn image_load(&mut self, io: &mut impl Objects) {
        self.extras_load(io);
        *self.saved = 0;
        let g = self.runtime.genre as usize;
        for (i, s) in self.runtime.slots.iter_mut().enumerate() {
            *s = Slot::recipe(g, i);
        }
        if g == 0 {
            self.recipe_effect = Some(9);
        }
        let n = io.load(object(g as u8), self.image).max(0) as usize;
        if n >= BANK_TAIL
            && magic(self.image, b"HYP3")
            && self.image[4] == 2
            && self.image[5] == g as u8
            && u16at(self.image, 10) == 376
        {
            let tail = self.image[14] == 1 && n >= BANK_LEN;
            let count = u16at(self.image, 6);
            let mask = u16at(self.image, 8);
            let old_answer = self.image[12] == 0;
            let old_pluck = self.image[13] == 0;
            for i in 0..10 {
                if mask >> i & 1 == 0 {
                    continue;
                }
                let s = &mut self.runtime.slots[i];
                let v = Record::parse(&self.image[16 + i * 376..16 + (i + 1) * 376])
                    .unwrap()
                    .decode_legacy(count, &s.parameters);
                s.decoded(v.values);
                if tail {
                    s.progression
                        .copy_from_slice(&self.image[BANK_TAIL + i * 6..BANK_TAIL + i * 6 + 6]);
                }
                s.kinds.copy_from_slice(&self.extras[i][..3]);
                if v.valid {
                    *self.saved |= 1 << i;
                    if self.extras[i][3] & 1 == 0 {
                        let range = P::DD_CHANGE.index()..=P::DD_MUTATE.index();
                        for (value, info) in s.parameters[range.clone()]
                            .iter_mut()
                            .zip(&PARAMETERS[range])
                        {
                            *value = info.default;
                        }
                    }
                    if old_answer {
                        s.answer_old();
                    }
                    if old_pluck {
                        s.pluck_old();
                    }
                } else {
                    *s = Slot::recipe(g, i);
                    if g == 0 {
                        self.recipe_effect = Some(i as u8);
                    }
                }
            }
        } else {
            if g == 0 && n > 0 {
                self.legacy_load(n);
            }
            self.image.fill(0);
        }
        self.image[BANK_TAIL..BANK_LEN].fill(0);
        for i in 0..10 {
            if *self.saved >> i & 1 != 0 {
                self.pack(i);
            }
        }
        self.header();
        self.bass_load(io);
    }
    pub fn globals_load(&mut self, io: &mut impl Objects) {
        let mut b = [0; 32];
        let n = io.load(4, &mut b);
        if (n == 32 || n == 12) && magic(&b, b"PMGL") {
            if n == 12 {
                b[12..].fill(0);
            }
            self.globals.magic = 0x4c474d50;
            for (i, v) in self.globals.values.iter_mut().enumerate() {
                *v = u16at(&b, 4 + i * 2) as i16;
            }
            self.globals.list.copy_from_slice(&b[12..28]);
            self.globals.play = b[28];
            self.globals.reserved.copy_from_slice(&b[29..32]);
        } else if *self.from_old == 0 {
            for (i, p) in [P::TUNE, P::MIDI, P::THEME].iter().enumerate() {
                self.globals.values[i] = p.info().default;
            }
        }
        self.globals.magic = 0x4c474d50;
    }
    pub fn globals_sync(&mut self) {
        for (i, p) in [P::TUNE, P::MIDI, P::THEME].iter().enumerate() {
            self.globals.values[i] = self.runtime.parameters[p.index()];
        }
        *self.pending_globals = 1;
    }
    pub fn flush_globals(&mut self, io: &mut impl Objects) -> i32 {
        if *self.pending_globals == 0 {
            return 0;
        }
        self.globals.magic = 0x4c474d50;
        let mut b = [0; 32];
        GlobalParts {
            values: self.globals.values,
            list: Some(&self.globals.list),
            play: self.globals.play,
            reserved: self.globals.reserved,
        }
        .encode_compatible(&mut b)
        .unwrap();
        if io.save(4, &b) != 0 {
            return -1;
        }
        *self.pending_globals = 0;
        0
    }
    pub fn flush(&mut self, io: &mut impl Objects) -> i32 {
        if *self.pending != 0 {
            self.header();
            if io.save(object(self.runtime.genre), &self.image[..BANK_LEN]) != 0 {
                return -1;
            }
            *self.pending = 0;
        }
        if *self.pending_extras != 0 {
            if self.extras_save(io) != 0 {
                return -1;
            }
            *self.pending_extras = 0;
        }
        if *self.bass_dirty != 0 && self.bass_save(io) != 0 {
            return -1;
        }
        self.flush_globals(io)
    }
    pub fn save_piece(&mut self) {
        let i = self.runtime.piece as usize;
        self.pack(i);
        let same = self.extras[i][3] & 1 != 0 && self.extras[i][4..11] == self.live_extras[i][1..8];
        if !same {
            self.extras[i][4..11].copy_from_slice(&self.live_extras[i][1..8]);
            self.extras[i][3] |= 1;
            *self.pending_extras = 1;
        }
        *self.saved |= 1 << i;
        *self.bass_dirty |= 1 << i;
        self.runtime.edited &= !(1 << i);
        *self.pending = 1;
    }
    pub fn factory_piece(&mut self, patterns: &mut Patterns) {
        let i = self.runtime.piece as usize;
        self.live_extras[i][0] = 0;
        self.runtime.slots[i] = Slot::recipe(self.runtime.genre as usize, i);
        if self.runtime.genre == 0 {
            self.recipe_effect = Some(i as u8);
        }
        self.take(patterns);
        if *self.saved >> i & 1 != 0 {
            *self.saved &= !(1 << i);
            self.image[16 + i * 376..16 + (i + 1) * 376].fill(0);
            self.image[BANK_TAIL + i * 6..BANK_TAIL + (i + 1) * 6].fill(0);
            self.extras[i].fill(0);
            *self.pending_extras = 1;
            *self.bass_dirty |= 1 << i;
            *self.pending = 1;
        }
        self.runtime.edited &= !(1 << i);
    }
    /// Return whether C must capture the engine's frozen lines and UI effects.
    /// Engine seeding/genre publication occurs only after this borrow ends.
    pub fn boot(&mut self, io: &mut impl Objects, seed: u32, patterns: &mut Patterns) -> bool {
        self.runtime.parameters.fill(0);
        self.runtime.pool = 0;
        self.runtime.acid.fill(0);
        self.runtime.pluck.fill(0);
        self.runtime.progression.fill(0);
        self.runtime.kinds.fill(0);
        self.runtime.bass.fill(0);
        self.runtime.leds = 0;
        self.runtime.piece = 0;
        self.runtime.genre = 0;
        self.runtime.edited = 0;
        self.runtime.slots.fill(Slot::ZERO);
        self.globals_load(io);
        let place = self.globals.values[3] as u16;
        let last = if seed != 0 && place & 256 == 0 && (1..=190).contains(&(place & 255)) {
            Some((place as u8 - 1) as usize)
        } else {
            None
        };
        self.runtime.genre = last.map_or(0, |n| (n / 10) as u8);
        self.image_load(io);
        for (i, v) in self.runtime.parameters.iter_mut().enumerate() {
            *v = PARAMETERS[i].default;
        }
        for (i, p) in [P::TUNE, P::MIDI, P::THEME].iter().enumerate() {
            self.runtime.parameters[p.index()] =
                self.globals.values[i].clamp(p.info().min, p.info().max);
        }
        self.runtime.leds = 1;
        let mut random = seed;
        fn next(r: &mut u32) -> u32 {
            *r ^= *r << 13;
            *r ^= *r >> 17;
            *r ^= *r << 5;
            *r
        }
        self.runtime.piece = last.map_or_else(
            || {
                if seed != 0 {
                    (next(&mut random) % 10) as u8
                } else {
                    0
                }
            },
            |n| (n % 10) as u8,
        );
        self.take(patterns);
        let nudge = seed != 0 && last.is_none() && *self.saved >> self.runtime.piece & 1 == 0;
        if nudge {
            for p in [
                P::PAD,
                P::ACID,
                P::BASS,
                P::PULSE,
                P::NOISE,
                P::EVOLVE,
                P::DRIFT,
                P::MOTION,
                P::BRIGHT,
                P::DETUNE,
                P::DR_CUT,
                P::DR_RES,
                P::DE_AMT,
                P::DL_DEPTH,
                P::AC_CUT,
                P::AC_RES,
                P::AC_ENV,
                P::AC_DEC,
                P::AC_DENS,
                P::AC_MUT,
                P::ECHO,
                P::REVERB,
                P::SIZE,
                P::DRIVE,
            ] {
                let v = &mut self.runtime.parameters[p.index()];
                if *v == 0 && p.index() <= P::NOISE.index() {
                    continue;
                }
                *v = (*v as i32 + (next(&mut random) % 17) as i32 - 8)
                    .clamp(p.info().min as i32, p.info().max as i32) as i16;
            }
            let p = P::TEMPO;
            self.runtime.parameters[p.index()] =
                (self.runtime.parameters[p.index()] + (next(&mut random) % 7) as i16 - 3)
                    .clamp(p.info().min, p.info().max);
        }
        nudge
    }
}
