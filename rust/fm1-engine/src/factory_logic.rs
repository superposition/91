// SPDX-License-Identifier: GPL-3.0-only
//! Procedural factory generation over compact donor/override tables.
use fm1_types::{
    GenreId, ParamId, PatternBank, PieceId, ScaleMask, StepMask, StepMod, PARAMETERS,
    PARAMETER_COUNT,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactoryPreset {
    pub parameters: [i16; PARAMETER_COUNT],
    pub pool: u16,
}
#[derive(Clone, Copy)]
#[repr(C)]
struct Override {
    parameter: ParamId,
    value: i16,
}
impl Override {
    const fn new(index: usize, value: i16) -> Self {
        match ParamId::new(index) {
            Ok(parameter) => Self { parameter, value },
            Err(_) => panic!("invalid factory parameter"),
        }
    }
}
#[repr(C)]
struct Palette {
    pad: [u8; 5],
    drone: [u8; 5],
    acid: [u8; 5],
    pluck: [u8; 5],
    kick: [u8; 3],
    snare: [u8; 3],
    hat: [u8; 3],
    perc: [u8; 3],
}
#[path = "factory_data.rs"]
mod data;
pub fn preset(genre: GenreId, piece: PieceId) -> FactoryPreset {
    let g = genre.index();
    let i = piece.index();
    let d = donor(genre) as usize - 4;
    let mut par = [0; PARAMETER_COUNT];
    for (value, info) in par.iter_mut().zip(PARAMETERS.iter()) {
        *value = info.default;
    }
    for change in data::BASE[d].iter().chain(data::OVERRIDES[g].iter()) {
        par[change.parameter.index()] = change.value;
    }
    macro_rules! p {
        ($name:ident) => {
            par[ParamId::$name.index()]
        };
    }
    let r = &data::ROWS[(d * 10 + i) * 7..(d * 10 + i + 1) * 7];
    let tr = &data::TEMPO_KEY_MODE[(g * 10 + i) * 3..(g * 10 + i + 1) * 3];
    p!(TEMPO) = tr[0] as i16;
    p!(ROOT) = tr[1] as i16;
    p!(MODE) = tr[2] as i16;
    p!(CHORD) = r[3] as i16;
    p!(PADTONE) = r[4] as i16;
    p!(DR_INST) = r[5] as i16;
    p!(AC_WAVE) = r[6] as i16;
    p!(PAD) = if p!(PAD) > 86 { 100 } else { p!(PAD) + 14 };
    p!(CHORD) = match p!(CHORD) {
        1 => 2,
        2 => 3,
        3 => 7,
        v => v,
    };
    p!(CM_LVL) = p!(CM_LVL) * 3 / 4;
    let palette = &data::PALETTES[d];
    let h = i / 5;
    let pw = d * 3 + i;
    p!(CM_SND) = palette.pluck[(i * 4 + d) % 5] as i16;
    p!(CM_STYLE) = ((i * 2 + d) % 6) as i16;
    p!(CM_RATE) = data::PLUCK_RATE[d] as i16;
    if d != 0 {
        p!(PADTONE) = palette.pad[(i + data::PAD_SHIFT[d] as usize * h) % 5] as i16;
        p!(DR_INST) = palette.drone[(i * 2 + d + 2 * h) % 5] as i16;
        p!(AC_WAVE) = palette.acid[(i * 3 + pw + 2 * h) % 5] as i16;
        p!(KIT_KICK) = palette.kick[(i + d) % 3] as i16;
        p!(KIT_SNARE) = palette.snare[(i * 2 + d + i / 3) % 3] as i16;
        p!(KIT_HAT) = palette.hat[(i + 2 * d + i / 9) % 3] as i16;
        p!(KIT_PERC) = palette.perc[(i * 2 + 2 * d) % 3] as i16;
        p!(PAN3) = 50 + if i & 1 != 0 { -14 } else { 14 };
        p!(PAN2) = 50 + if i & 1 != 0 { 8 } else { -8 };
    }
    p!(CM_SND) = data::PLUCK[g * 5 + (i * 4 + d) % 5] as i16;
    // C's RETRO pad assignment is overwritten by the final donor palette row.
    // Preserve the result without retaining a dead intermediate lookup table.
    p!(PAD_COMP) = (2 + i % 5) as i16;
    p!(PE_ATK) = 4;
    p!(PE_REL) = (58 + (i * 3) % 18) as i16;
    p!(BRIGHT) = if p!(BRIGHT) > 72 {
        100
    } else {
        p!(BRIGHT) + 14
    };
    p!(DETUNE) = 8;
    p!(VOICING) = 1;
    p!(CM_DEC) = (64 + (i % 3) * 8) as i16;
    p!(CM_STYLE) = match i % 3 {
        0 => 0,
        1 => 5,
        _ => 2,
    };
    p!(DRIFT) = 82;
    p!(SWING) = (p!(SWING) + 8).min(64);
    p!(HUMAN) = (p!(HUMAN) + 10).min(60);
    p!(PADTONE) = palette.pad[i % 5] as i16;
    if slow(g, i) {
        p!(BL_MODE) = 0;
        p!(CM_RATE) = 2;
        p!(AC_DENS) = 35;
        p!(ACID) = 40;
        p!(PAD) = 60;
        p!(REVERB) = 60;
        p!(SIZE) = 85;
        p!(ECHO) = 50;
        p!(DRIVE) = 15;
        p!(PULSE) = 70;
        p!(SWING) = 10;
        p!(EVOLVE) = 35;
        p!(BASS) = 68;
    }
    if g == 0 {
        p!(PE_ATK) = 14;
        p!(PE_REL) = 76;
        p!(REVERB) = 58;
        p!(SIZE) = 82;
        p!(RV_TYPE) = 1;
        p!(RV_X) = 30;
        p!(BRIGHT) = if p!(BRIGHT) > 30 { p!(BRIGHT) - 10 } else { 20 };
        p!(DRIFT) = 10;
        p!(BL_MODE) = 3;
        p!(BL_LEN) = 46;
        p!(BL_MOVE) = 14;
        p!(BL_GLIDE) = 8;
        p!(BASS) = 82;
        if p!(CHORD) == 7 {
            p!(CHORD) = 3;
        }
        p!(ACID) = 12;
        p!(AC_DENS) = 18;
    }
    if g == 7 {
        p!(BL_MODE) = 4;
        p!(BL_LEN) = 28;
        p!(BL_MOVE) = 34;
        p!(BL_GLIDE) = 0;
        p!(BASS) = 88;
        p!(PULSE) = 88;
        p!(DRIFT) = 45;
        p!(ACID) = 45;
        p!(AC_DENS) = 45;
    }
    p!(DR_INST) = [5, 10, 5, 9, 10][i % 5];
    p!(DR_CUT) = 42;
    p!(DR_RES) = 28;
    p!(DRIVE) = 42;
    p!(DETUNE) = 18;
    p!(BASS) = 86;
    p!(BASSOCT) = -1;
    let pool = data::MODE_POOLS[p!(MODE) as usize];
    FactoryPreset {
        parameters: par,
        pool,
    }
}
fn slow(g: usize, i: usize) -> bool {
    g == 17 && (i == 0 || i == 1 || i == 9)
}
pub fn patterns(genre: GenreId, piece: PieceId) -> PatternBank {
    let d = donor(genre) as usize - 4;
    let i = piece.index();
    let mut result = [[StepMask::from_bits(0); 6]; 6];
    for (pattern, lanes) in result.iter_mut().enumerate() {
        for (lane, mask) in lanes.iter_mut().enumerate() {
            let row = if (lane == 0 || lane == 5) && i >= 6 {
                (i + pattern + 2) % 6
            } else {
                (i + pattern) % 6
            };
            *mask = StepMask::from_bits(if slow(d + 4, i) {
                data::PSY_DOWN[row * 6 + lane]
            } else {
                data::PATTERNS[(d * 6 + row) * 6 + lane]
            });
        }
    }
    result
}
pub fn modifiers(genre: GenreId, piece: PieceId) -> [[StepMod; 16]; 6] {
    let g = donor(genre);
    let i = piece.index();
    let mut result = [[StepMod::from_bits(0); 16]; 6];
    let mut raw = [[0u8; 16]; 6];
    // Step-major rules affect several lanes; a row iterator is the wrong axis.
    for step in 0u8..16 {
        let s = usize::from(step);
        if g == 1 {
            raw[1][s] = 16;
            raw[2][s] = 16;
        }
        if g == 7 {
            raw[1][s] = 8;
        }
        if (g == 1 || g == 2 || g == 7 || (4..=6).contains(&g) || g == 9) && s & 1 != 0 {
            raw[3][s] = 128;
        }
        if (g == 2 || g == 8) && s & 3 == 2 {
            raw[3][s] = 192;
        }
        if (5..=6).contains(&g) && (s == 4 || s == 12) {
            raw[1][s] = 192;
        }
        if g == 4 {
            raw[1][s] = 8;
        }
        if g == 8 && s == 0 {
            raw[0][s] = 192;
        }
        if g == 10 && s & 3 == 2 {
            raw[3][s] = 192;
        }
        if ((11..=15).contains(&g) || g == 17) && s & 1 != 0 {
            raw[3][s] = 128;
        }
        if (g == 11 || g == 13) && (s == 4 || s == 12) {
            raw[1][s] = 192;
        }
        if g == 12 {
            raw[1][s] = 8;
        }
        if g == 15 {
            raw[1][s] = 16;
        }
        if g == 14 {
            raw[5][s] = 128;
        }
        if g == 18 && s & 3 == 2 {
            raw[5][s] = 192;
        }
    }
    if g == 18 {
        raw[5][7] = 2;
        raw[5][15] = 2;
        raw[1][12] = 5;
    }
    if g == 16 {
        raw[1][15] = 2;
        raw[5][13] = 1;
    }
    if g == 9 || g == 16 || g == 18 {
        let rolls = [(14, 3), (6, 2), (10, 1), (12, 4), (14, 7)];
        for index in [i % 5, (i + 2) % 5] {
            let (step, ratchet) = rolls[index];
            raw[3][step] = (raw[3][step] & !7) | ratchet;
        }
    }
    for lane in 0..6 {
        for step in 0..16 {
            result[lane][step] = StepMod::from_bits(raw[lane][step]);
        }
    }
    result
}
pub fn boot_patterns() -> &'static PatternBank {
    &data::BOOT_PATTERNS
}
pub fn mode_pools() -> &'static [u16; 8] {
    &data::MODE_POOLS
}
pub fn mode(index: usize) -> Option<ScaleMask> {
    data::MODE_POOLS
        .get(index)
        .copied()
        .map(ScaleMask::from_bits)
}
pub fn donor(genre: GenreId) -> u8 {
    data::DONORS[genre.index()]
}
pub fn arc(piece: PieceId) -> ([u8; 6], [u8; 3]) {
    data::ARCS[piece.index()]
}
pub fn names(category: usize) -> Option<&'static [&'static [u8]]> {
    data::NAMES.get(category).copied()
}
pub fn name(category: usize, index: usize) -> Option<&'static [u8]> {
    names(category)?.get(index).copied()
}
pub fn numeric_storage_bytes() -> usize {
    data::BASE
        .iter()
        .chain(data::OVERRIDES.iter())
        .map(|row| core::mem::size_of_val(*row))
        .sum::<usize>()
        + core::mem::size_of_val(&data::ROWS)
        + core::mem::size_of_val(&data::PATTERNS)
        + core::mem::size_of_val(&data::PSY_DOWN)
        + core::mem::size_of_val(&data::TEMPO_KEY_MODE)
        + core::mem::size_of_val(&data::PLUCK)
        + core::mem::size_of_val(&data::PLUCK_RATE)
        + core::mem::size_of_val(&data::PAD_SHIFT)
        + core::mem::size_of_val(&data::PALETTES)
        + 72
        + 16
        + 19
        + 90
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_compact_source_table_entry_matches_independent_c_including_unused_donors() {
        let c: serde_json::Value =
            serde_json::from_str(include_str!("../tests/factory/compact.json")).unwrap();
        for (key, table) in [("base", data::BASE), ("overrides", data::OVERRIDES)] {
            for (row, changes) in table.iter().enumerate() {
                let expected = c[key][row].as_array().unwrap();
                assert_eq!(expected.len(), changes.len() * 2 + 1);
                for (index, change) in changes.iter().enumerate() {
                    assert_eq!(
                        change.parameter.index() as u64,
                        expected[index * 2].as_u64().unwrap()
                    );
                    assert_eq!(
                        change.value as i64,
                        expected[index * 2 + 1].as_i64().unwrap()
                    );
                }
                assert_eq!(expected.last().unwrap().as_i64(), Some(-1));
            }
        }
        macro_rules! same {
            ($field:ident,$key:literal) => {
                for (index, value) in data::$field.iter().enumerate() {
                    assert_eq!(
                        *value as u64,
                        c[$key][index].as_u64().unwrap(),
                        "{}[{}]",
                        $key,
                        index
                    );
                }
            };
        }
        same!(ROWS, "rows");
        same!(PATTERNS, "patterns");
        same!(PSY_DOWN, "psy_down");
        same!(TEMPO_KEY_MODE, "tempo_key_mode");
        same!(PLUCK, "pluck");
        same!(PLUCK_RATE, "pluck_rate");
        same!(PAD_SHIFT, "pad_shift");
        for (genre, palette) in data::PALETTES.iter().enumerate() {
            let mut index = genre * 32;
            for field in [
                palette.pad.as_slice(),
                &palette.drone,
                &palette.acid,
                &palette.pluck,
                &palette.kick,
                &palette.snare,
                &palette.hat,
                &palette.perc,
            ] {
                for value in field {
                    assert_eq!(*value as u64, c["palettes"][index].as_u64().unwrap());
                    index += 1;
                }
            }
            assert_eq!(index, (genre + 1) * 32);
        }
    }
}
