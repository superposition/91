// SPDX-License-Identifier: GPL-3.0-only
//! Immutable numeric scheduling recipes. Hardware's selectable C0 tables remain
//! independent; the emulator reads these values without building a cache.

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Section {
    pub kick: u8,
    pub hat: u8,
    pub build: u8,
    pub pad: u8,
    pub kret: u8,
    pub snare: u8,
    pub bass: u8,
    pub ft: u8,
    pub fl: u8,
}

const fn section(fields: [u8; 9]) -> Section {
    let [kick, hat, build, pad, kret, snare, bass, ft, fl] = fields;
    Section {
        kick,
        hat,
        build,
        pad,
        kret,
        snare,
        bass,
        ft,
        fl,
    }
}

// The bridge instantiates one exported immutable array for each C ABI view.
// These constants otherwise have no storage in the production core.
pub const SECTIONS: [Section; 19] = [
    section([1, 0, 4, 90, 1, 0, 0, 0, 64]),
    section([0, 0, 2, 60, 0, 0, 0, 0, 16]),
    section([1, 0, 8, 90, 1, 0, 0, 0, 8]),
    section([0, 0, 8, 80, 0, 0, 0, 8, 1 | 2 | 8]),
    section([1, 1, 4, 40, 0, 1, 1, 0, 16]),
    section([0, 1, 8, 90, 0, 0, 0, 0, 1 | 8]),
    section([1, 1, 4, 0, 0, 1, 0, 0, 0]),
    section([0, 0, 4, 80, 0, 0, 0, 0, 32]),
    section([0, 0, 4, 90, 1, 0, 0, 0, 0]),
    section([0, 0, 4, 40, 0, 0, 0, 0, 1 | 4 | 8]),
    section([0, 0, 4, 90, 1, 0, 0, 0, 8]),
    section([1, 1, 4, 0, 0, 1, 0, 0, 0]),
    section([0, 0, 4, 60, 0, 0, 0, 0, 16]),
    section([0, 0, 4, 90, 1, 0, 0, 0, 8]),
    section([0, 0, 4, 80, 0, 0, 0, 0, 16]),
    section([0, 0, 2, 60, 0, 0, 0, 0, 16]),
    section([0, 1, 4, 50, 1, 0, 0, 0, 1 | 4 | 8]),
    section([0, 0, 8, 90, 0, 0, 0, 0, 1 | 8]),
    section([0, 1, 4, 70, 1, 0, 0, 0, 1 | 4 | 8]),
];
pub const FILL_OK: [u8; 19] = [
    0x3f, 0x16, 0x07, 0x49, 0x14, 0x7f, 0x7e, 0x5a, 0x3f, 0x64, 0x87, 0x9e, 0x14, 0x87, 0x12, 0x16,
    0x70, 0x47, 0x7e,
];

#[derive(Clone, Copy, Debug)]
pub struct BassFigure {
    pub mask: u16,
    pub intervals: [i8; 4],
}
const fn figure(mask: u16, intervals: [i8; 4]) -> BassFigure {
    BassFigure { mask, intervals }
}
const BASS_ROOT: [u16; 19] = [
    0x1111, 0x0421, 0x1111, 0x0401, 0x0501, 0x0441, 0x1121, 0x0449, 0x5555, 0x2401, 0x5555, 0x4529,
    0x0149, 0x5555, 0x2449, 0x0401, 0x0405, 0xeeee, 0xeeee,
];
const BASS_RIFF: [BassFigure; 19] = [
    figure(0x5555, [0, 0, 12, 0]),
    figure(0x0421, [0, 7, 5, 0]),
    figure(0x4444, [0, 0, 12, 0]),
    figure(0x0401, [0, 0, -2, 0]),
    figure(0x0501, [0, 0, 7, 5]),
    figure(0x0441, [0, 0, 7, 5]),
    figure(0x1121, [0, 0, 12, 7]),
    figure(0x0449, [0, 7, 0, 12]),
    figure(0x5555, [0, 0, 12, 0]),
    figure(0x2401, [0, 0, 12, 7]),
    figure(0x5555, [0, 12, 0, 12]),
    figure(0x0509, [0, 12, 7, 0]),
    figure(0x0149, [0, 0, 7, 5]),
    figure(0x5555, [0, 0, 12, 0]),
    figure(0x2449, [0, 0, 7, 0]),
    figure(0x0401, [0, 0, -2, 0]),
    figure(0x1249, [0, 1, 0, -2]),
    figure(0xeeee, [0, 0, 0, 12]),
    figure(0xeeee, [0, 0, 1, 0]),
];
const BASS_FIGURES: [BassFigure; 9] = [
    figure(0x5555, [0, 12, 0, 12]),
    figure(0x0441, [0, 0, 12, 0]),
    figure(0x0149, [0, 0, 7, 5]),
    figure(0x2449, [0, 0, 7, 0]),
    figure(0x0449, [0, 7, 0, 12]),
    figure(0x0509, [0, 12, 7, 0]),
    figure(0x1121, [0, 0, 12, 7]),
    figure(0x0501, [0, 0, 7, 5]),
    figure(0x7777, [0, 0, 0, 12]),
];
/// Genre is already mapped by the existing C adapter; do not map it twice here.
/// Mode is the validated firmware value 0..16.
pub fn bass_recipe(genre: usize, mode: i32) -> (u16, BassFigure) {
    let selected = if mode == 6 {
        BASS_RIFF[genre]
    } else {
        BASS_FIGURES[(if mode > 6 { mode - 7 } else { 0 }) as usize % 9]
    };
    (BASS_ROOT[genre], selected)
}

const COMP_MASK: [u16; 7] = [0x4444, 0x1111, 0x4949, 0x0041, 0x2448, 0x0401, 0x5555];
const COMP_GENRE: [u16; 19] = [
    0x4444, 0x0041, 0x4949, 0x0401, 0x4444, 0x0408, 0x0a48, 0x2448, 0x2222, 0x0401, 0x4545, 0x2448,
    0x4444, 0x0401, 0x2448, 0x0401, 0x1249, 0x0101, 0x0001,
];
pub fn comp_mask(jam: bool, comp: i32, genre: usize) -> u16 {
    if jam {
        1
    } else if comp >= 2 {
        COMP_MASK[(comp - 2) as usize]
    } else {
        COMP_GENRE[genre]
    }
}
