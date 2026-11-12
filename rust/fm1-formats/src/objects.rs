// SPDX-License-Identifier: GPL-3.0-only
//! Explicit constructors for every retained flash-object schema. State and
//! persistence policy are caller responsibilities; no runtime struct is copied.
use crate::{storage, voice::PackedVoice, CodecError};
pub const STORE_MAX: usize = 3840;
fn prepare<'a>(
    out: &'a mut [u8],
    len: usize,
    base: Option<&[u8]>,
) -> Result<&'a mut [u8], CodecError> {
    if len > STORE_MAX {
        return Err(CodecError::Length);
    }
    if out.len() < len {
        return Err(CodecError::Capacity);
    }
    if base.is_some_and(|b| b.len() != len) {
        return Err(CodecError::Length);
    }
    let out = &mut out[..len];
    if let Some(base) = base {
        out.copy_from_slice(base);
    } else {
        out.fill(0);
    }
    Ok(out)
}
fn u16_at(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn u32_at(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn bank_header(
    genre: u8,
    parameters: u16,
    saved: u16,
    flags: [u8; 4],
    out: &mut [u8],
) -> Result<usize, CodecError> {
    if genre >= 19 {
        return Err(CodecError::Index);
    }
    if out.len() < 16 {
        return Err(CodecError::Capacity);
    }
    let out = &mut out[..16];
    out.fill(0);
    out[..4].copy_from_slice(b"HYP3");
    out[4] = 2;
    out[5] = genre;
    u16_at(out, 6, parameters);
    u16_at(out, 8, saved);
    u16_at(out, 10, 376);
    out[12..16].copy_from_slice(&flags);
    Ok(16)
}
pub fn extension_header(
    kind: ExtensionKind,
    genres: u8,
    reserved: [u8; 2],
    out: &mut [u8],
) -> Result<usize, CodecError> {
    if genres > 19 {
        return Err(CodecError::Index);
    }
    if out.len() < 8 {
        return Err(CodecError::Capacity);
    }
    out[..4].copy_from_slice(if kind == ExtensionKind::Extras {
        b"PCX1"
    } else {
        b"BSQ1"
    });
    out[4] = 16;
    out[5] = genres;
    out[6..8].copy_from_slice(&reserved);
    Ok(8)
}
pub struct GlobalParts<'a> {
    pub values: [i16; 4],
    pub list: Option<&'a [u8; 16]>,
    pub play: u8,
    pub reserved: [u8; 3],
}
impl GlobalParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        if self.list.is_some_and(|s| s.iter().any(|v| *v > 190)) {
            return Err(CodecError::Index);
        }
        self.encode_compatible(out)
    }
    /// Preserve unknown list entries from an already stored legacy object.
    pub fn encode_compatible(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        let n = if self.list.is_some() { 32 } else { 12 };
        let out = prepare(out, n, None)?;
        out[..4].copy_from_slice(b"PMGL");
        for (i, value) in self.values.iter().enumerate() {
            u16_at(out, 4 + i * 2, *value as u16);
        }
        if let Some(list) = self.list {
            out[12..28].copy_from_slice(list);
            out[28] = self.play;
            out[29..32].copy_from_slice(&self.reserved);
        }
        Ok(n)
    }
}
pub struct SettingsParts<'a> {
    pub effects: &'a [u8],
    pub performance: &'a [u8; 12],
    pub targets: &'a [u8; 4],
    pub pattern: u16,
    pub usb: u16,
}
impl SettingsParts<'_> {
    pub fn canonical_len(&self) -> Result<usize, CodecError> {
        let n = self.effects.len();
        if n > 65535 {
            return Err(CodecError::Length);
        }
        Ok(((24 + n).next_multiple_of(2) + 4).next_multiple_of(4))
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        self.encode_preserving(None, out)
    }
    pub fn encode_preserving(
        &self,
        base: Option<&[u8]>,
        out: &mut [u8],
    ) -> Result<usize, CodecError> {
        let canonical = self.canonical_len()?;
        let tail = (24 + self.effects.len()).next_multiple_of(2);
        let n = base.map_or(canonical, |b| b.len());
        if n < tail + 4 {
            return Err(CodecError::Length);
        }
        let out = prepare(out, n, base)?;
        out[..4].copy_from_slice(b"SET1");
        u16_at(out, 4, self.effects.len() as u16);
        u16_at(out, 6, 12);
        out[8..8 + self.effects.len()].copy_from_slice(self.effects);
        out[8 + self.effects.len()..20 + self.effects.len()].copy_from_slice(self.performance);
        out[20 + self.effects.len()..24 + self.effects.len()].copy_from_slice(self.targets);
        u16_at(out, tail, self.pattern);
        u16_at(out, tail + 2, self.usb);
        Ok(n)
    }
}
pub struct VoicesParts<'a> {
    pub voices: &'a [PackedVoice<'a>],
}
impl VoicesParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        self.encode_preserving(None, out)
    }
    pub fn encode_preserving(
        &self,
        base: Option<&[u8]>,
        out: &mut [u8],
    ) -> Result<usize, CodecError> {
        if self.voices.len() > 16 {
            return Err(CodecError::Index);
        }
        let out = prepare(out, 2056, base)?;
        out[..4].copy_from_slice(b"VOX1");
        u32_at(out, 4, self.voices.len() as u32);
        for (i, voice) in self.voices.iter().enumerate() {
            out[8 + i * 128..8 + (i + 1) * 128].copy_from_slice(voice.bytes());
        }
        Ok(2056)
    }
}
pub struct KindsParts<'a> {
    pub kinds: &'a [u8],
}
impl KindsParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        if !self.kinds.len().is_multiple_of(30) || self.kinds.len() > 570 {
            return Err(CodecError::Length);
        }
        let n = 4 + self.kinds.len();
        let out = prepare(out, n, None)?;
        out[..4].copy_from_slice(b"KND1");
        out[4..].copy_from_slice(self.kinds);
        Ok(n)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionKind {
    Extras,
    Bass,
}
pub struct ExtensionsParts<'a> {
    pub kind: ExtensionKind,
    pub genres: u8,
    pub reserved: [u8; 2],
    pub records: &'a [u8],
}
impl ExtensionsParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        if self.genres > 19 {
            return Err(CodecError::Index);
        }
        if self.records.len() != self.genres as usize * 160 {
            return Err(CodecError::Length);
        }
        let n = 8 + self.records.len();
        let out = prepare(out, n, None)?;
        extension_header(self.kind, self.genres, self.reserved, out)?;
        out[8..].copy_from_slice(self.records);
        Ok(n)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiEntry([u8; 3]);
impl MidiEntry {
    pub fn parse(bytes: [u8; 3]) -> Result<Self, CodecError> {
        let [flags, number, target] = bytes;
        let valid = if flags & 128 != 0 {
            flags & 16 != 0 && target < 176
        } else {
            target < 14 || (32..59).contains(&target) || (64..71).contains(&target)
        };
        if !valid || number > 127 || flags >> 5 & 3 == 3 {
            return Err(CodecError::Range);
        }
        Ok(Self(bytes))
    }
    pub const fn bytes(self) -> [u8; 3] {
        self.0
    }
    pub const fn setting(self) -> bool {
        self.0[0] & 128 != 0
    }
}
pub struct MidiMapParts<'a> {
    pub version: Option<u8>,
    pub entries: &'a [MidiEntry],
    pub reserved: [u8; 3],
}
impl MidiMapParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        self.encode_preserving(None, out)
    }
    pub fn encode_preserving(
        &self,
        base: Option<&[u8]>,
        out: &mut [u8],
    ) -> Result<usize, CodecError> {
        if self.entries.len() > 48 {
            return Err(CodecError::Index);
        }
        let version = self
            .version
            .unwrap_or(if self.entries.iter().any(|v| v.setting()) {
                2
            } else {
                1
            });
        if version != 1 && version != 2 {
            return Err(CodecError::Version);
        }
        let out = prepare(out, 152, base)?;
        out[..4].copy_from_slice(if version == 1 { b"MAP1" } else { b"MAP2" });
        out[4] = self.entries.len() as u8;
        out[5..8].copy_from_slice(&self.reserved);
        for (i, entry) in self.entries.iter().enumerate() {
            out[8 + i * 3..11 + i * 3].copy_from_slice(&entry.bytes());
        }
        Ok(152)
    }
}
pub struct BankParts<'a> {
    pub genre: u8,
    pub parameters: u16,
    pub saved: u16,
    pub answer_encoding: u8,
    pub voice_numbering: u8,
    pub progression_flag: u8,
    pub reserved: u8,
    pub records: &'a [u8; 3760],
    pub progressions: Option<&'a [u8; 60]>,
    pub trailing: &'a [u8],
}
impl BankParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        if self.genre >= 19 {
            return Err(CodecError::Index);
        }
        let n = 3776 + if self.progressions.is_some() { 60 } else { 0 } + self.trailing.len();
        let out = prepare(out, n, None)?;
        bank_header(
            self.genre,
            self.parameters,
            self.saved,
            [
                self.answer_encoding,
                self.voice_numbering,
                self.progression_flag,
                self.reserved,
            ],
            out,
        )?;
        out[16..3776].copy_from_slice(self.records);
        let mut at = 3776;
        if let Some(progressions) = self.progressions {
            out[at..at + 60].copy_from_slice(progressions);
            at += 60;
        }
        out[at..].copy_from_slice(self.trailing);
        Ok(n)
    }
}
pub struct LegacyKitParts<'a> {
    pub format: u32,
    pub patterns: &'a [u16; 36],
    pub lanes: &'a [i16; 18],
    pub theme: u8,
    pub reserved: &'a [u8; 15],
}
impl LegacyKitParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        let out = prepare(out, 132, None)?;
        out[..4].copy_from_slice(b"HYP1");
        u32_at(out, 4, self.format);
        for (i, mask) in self.patterns.iter().enumerate() {
            u16_at(out, 8 + i * 2, *mask);
        }
        for (i, value) in self.lanes.iter().enumerate() {
            u16_at(out, 80 + i * 2, *value as u16);
        }
        out[116] = self.theme;
        out[117..132].copy_from_slice(self.reserved);
        Ok(132)
    }
}
pub struct LegacyRecordParts<'a> {
    pub parameters: &'a [i16],
    pub pool: u16,
    pub patterns: &'a [u16; 36],
}
impl LegacyRecordParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        let count = self.parameters.len();
        if !(1..=176).contains(&count) {
            return Err(CodecError::Range);
        }
        let n = count * 2 + 74;
        let out = prepare(out, n, None)?;
        for (i, value) in self.parameters.iter().enumerate() {
            u16_at(out, i * 2, *value as u16);
        }
        u16_at(out, count * 2, self.pool);
        for (i, mask) in self.patterns.iter().enumerate() {
            u16_at(out, count * 2 + 2 + i * 2, *mask);
        }
        Ok(n)
    }
}
pub struct LegacyBankParts<'a> {
    pub parameters: u16,
    pub saved: u16,
    pub globals: [i16; 3],
    pub revision: u16,
    pub records: &'a [LegacyRecordParts<'a>; 10],
}
impl LegacyBankParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        let count = self.parameters as usize;
        if !(1..=176).contains(&count) || self.records.iter().any(|r| r.parameters.len() != count) {
            return Err(CodecError::Length);
        }
        let stride = count * 2 + 74;
        let n = 20 + 10 * stride;
        let out = prepare(out, n, None)?;
        out[..4].copy_from_slice(b"HYP2");
        u32_at(out, 4, 1);
        u16_at(out, 8, self.parameters);
        u16_at(out, 10, self.saved);
        for (i, value) in self.globals.iter().enumerate() {
            u16_at(out, 12 + i * 2, *value as u16);
        }
        u16_at(out, 18, self.revision);
        for (i, record) in self.records.iter().enumerate() {
            record
                .encode(&mut out[20 + i * stride..20 + (i + 1) * stride])
                .unwrap();
        }
        Ok(n)
    }
}

/// Update a bank field only after validating its byte span and index. The
/// caller decides when an edited bank is committed to storage (#7).
pub fn put_record(
    bank: &mut [u8],
    genre: u8,
    index: usize,
    record: &[u8; 376],
    progression: Option<&[u8; 6]>,
) -> Result<(), CodecError> {
    if index >= 10 {
        return Err(CodecError::Index);
    }
    storage::Bank::parse(bank, genre)?;
    if bank.len() < 3776 || &bank[..4] != b"HYP3" {
        return Err(CodecError::Version);
    }
    if progression.is_some() && bank.len() < 3836 {
        return Err(CodecError::Length);
    }
    bank[16 + index * 376..16 + (index + 1) * 376].copy_from_slice(record);
    if let Some(progression) = progression {
        bank[3776 + index * 6..3782 + index * 6].copy_from_slice(progression);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportResult {
    pub dropped: u16,
    pub filled: u16,
    pub replaced: bool,
}
/// Browser legacy import policy: parameter deltas unknown to either side are
/// filled/dropped in the original byte spans; no persistence decisions occur.
#[allow(clippy::too_many_arguments)] // explicit legacy field boundary
pub fn import_piece(
    previous: &[u8],
    genre: u8,
    index: usize,
    parameters: u16,
    piece_parameters: u16,
    record: &[u8; 376],
    progression: &[u8; 6],
    out: &mut [u8],
) -> Result<ImportResult, CodecError> {
    if genre >= 19 || index >= 10 {
        return Err(CodecError::Index);
    }
    if parameters > 176 {
        return Err(CodecError::Range);
    }
    let old = if previous.is_empty() {
        None
    } else {
        Some(storage::Bank::parse(previous, genre)?)
    };
    if old.is_some_and(|b| !b.modern() || b.encoding_flags().is_none_or(|v| v[0] != 1 || v[1] != 1))
    {
        return Err(CodecError::Version);
    }
    let mut records = [0u8; 3760];
    let mut progressions = [0u8; 60];
    let mut flags = [0u8; 4];
    let saved = old.map_or(0, |b| b.saved());
    let old_parameters = old.map_or(parameters, |b| b.parameter_count());
    if let Some(old) = old {
        records.copy_from_slice(&old.bytes()[16..3776]);
        flags = old.encoding_flags().unwrap();
        for i in 0..10 {
            if let Some(prg) = old.progression(i)? {
                progressions[i * 6..(i + 1) * 6].copy_from_slice(prg);
            }
        }
    }
    if old_parameters < parameters {
        for i in 0..10 {
            if i != index && saved >> i & 1 != 0 {
                let start = (74 + old_parameters as usize).min(376);
                let end = (74 + parameters as usize).min(376);
                records[i * 376 + start..i * 376 + end].fill(255);
            }
        }
    }
    let mut imported = *record;
    let start = 74 + parameters.min(piece_parameters) as usize;
    let end = (74 + parameters.max(piece_parameters) as usize).min(376);
    if start < end {
        imported[start..end].fill(if piece_parameters < parameters {
            255
        } else {
            0
        });
    }
    records[index * 376..(index + 1) * 376].copy_from_slice(&imported);
    progressions[index * 6..(index + 1) * 6].copy_from_slice(progression);
    BankParts {
        genre,
        parameters,
        saved: saved | 1 << index,
        answer_encoding: 1,
        voice_numbering: 1,
        progression_flag: 1,
        reserved: flags[3],
        records: &records,
        progressions: Some(&progressions),
        trailing: &[],
    }
    .encode(out)?;
    Ok(ImportResult {
        dropped: piece_parameters.saturating_sub(parameters),
        filled: parameters.saturating_sub(piece_parameters),
        replaced: saved >> index & 1 != 0,
    })
}

fn extension_at(bytes: &[u8], genre: usize, piece: usize) -> Result<(usize, usize), CodecError> {
    if genre >= 19 || piece >= 10 {
        return Err(CodecError::Index);
    }
    if bytes.len() >= 8 && (&bytes[..4] == b"PCX1" || &bytes[..4] == b"BSQ1") && bytes[4] == 16 {
        Ok((8 + (genre * 10 + piece) * 16, 16))
    } else if bytes.len() >= 4 && &bytes[..4] == b"KND1" {
        Ok((4 + (genre * 10 + piece) * 3, 3))
    } else {
        Err(CodecError::Magic)
    }
}
/// Read one bounded extension field. Missing/all-zero fields are represented by
/// zero output bytes exactly as the legacy sharing tools represented `null`.
pub fn extension_get(
    bytes: &[u8],
    genre: usize,
    piece: usize,
    field: u32,
    out: &mut [u8],
) -> Result<usize, CodecError> {
    let (at, width) = match extension_at(bytes, genre, piece) {
        Ok(v) => v,
        Err(CodecError::Magic) => return Ok(0),
        Err(e) => return Err(e),
    };
    let Some(record) = bytes.get(at..at + width) else {
        return Ok(0);
    };
    let data = match field {
        0 if &bytes[..4] != b"BSQ1" => &record[..3],
        1 if width == 16 && &bytes[..4] == b"PCX1" && record[3] & 1 != 0 => &record[4..11],
        2 if width == 16 && &bytes[..4] == b"BSQ1" => record,
        _ => return Ok(0),
    };
    if field != 1 && data.iter().all(|v| *v == 0) {
        return Ok(0);
    }
    if out.len() < data.len() {
        return Err(CodecError::Capacity);
    }
    out[..data.len()].copy_from_slice(data);
    Ok(data.len())
}
/// Format migration/editing for the shared piece extension objects. `kind` is
/// 0 KND1,1 PCX1,2 BSQ1; absence clears only the selected piece's field/record.
#[allow(clippy::too_many_arguments)] // explicit optional serialized fields
pub fn extension_update(
    previous: &[u8],
    genre: usize,
    piece: usize,
    genres: u8,
    kind: u32,
    kinds: Option<&[u8; 3]>,
    fx: Option<&[u8; 7]>,
    bass: Option<&[u8; 16]>,
    out: &mut [u8],
) -> Result<usize, CodecError> {
    if genre >= 19 || piece >= 10 || genres > 19 || genre >= genres as usize {
        return Err(CodecError::Index);
    }
    if kind > 2 {
        return Err(CodecError::Version);
    }
    let (width, header) = if kind == 0 { (3, 4) } else { (16, 8) };
    let n = header + genres as usize * 10 * width;
    let out = prepare(out, n, None)?;
    let desired = if kind == 0 {
        b"KND1"
    } else if kind == 1 {
        b"PCX1"
    } else {
        b"BSQ1"
    };
    if previous.len() >= header && &previous[..4] == desired && (kind == 0 || previous[4] == 16) {
        let length = previous.len().min(n);
        out[..length].copy_from_slice(&previous[..length]);
    } else if kind == 1 && previous.len() >= 4 && &previous[..4] == b"KND1" {
        let count = ((previous.len() - 4) / 3).min(genres as usize * 10);
        for i in 0..count {
            out[8 + i * 16..11 + i * 16].copy_from_slice(&previous[4 + i * 3..7 + i * 3]);
        }
    }
    out[..4].copy_from_slice(desired);
    if kind != 0 {
        out[4] = 16;
        out[5] = genres;
        out[6] = 0;
        out[7] = 0;
    }
    let at = header + (genre * 10 + piece) * width;
    out[at..at + width].fill(0);
    if kind == 2 {
        if let Some(bass) = bass {
            out[at..at + 16].copy_from_slice(bass);
        }
    } else {
        if let Some(kinds) = kinds {
            out[at..at + 3].copy_from_slice(kinds);
        }
        if kind == 1 {
            if let Some(fx) = fx {
                out[at + 3] = 1;
                out[at + 4..at + 11].copy_from_slice(fx);
            }
        }
    }
    Ok(n)
}
