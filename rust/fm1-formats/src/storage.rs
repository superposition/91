// SPDX-License-Identifier: GPL-3.0-only
//! Stored records are byte views, never Rust struct images. Legacy acceptance
//! is separate from application conversion/default policy.
use crate::{read_u16, read_u32, CodecError};
use fm1_types::{ParamId, PARAMETERS, PARAMETER_COUNT};
pub const RECORD_LEN: usize = 376;
pub const BANK_TAIL: usize = 16 + 10 * RECORD_LEN;
pub const BANK_LEN: usize = BANK_TAIL + 60;
pub fn scale_valid(pool: u16) -> bool {
    pool & 1 != 0 && pool & !0xfff == 0 && pool.count_ones() >= 3
}
pub struct RecordValues {
    pub parameters: [i16; 176],
    pub pool: u16,
    pub patterns: [u16; 36],
    pub modifiers: [u8; 96],
    pub acid: [u8; 18],
    pub pluck: [u8; 12],
}
pub struct DecodedRecord {
    pub values: RecordValues,
    pub valid: bool,
}
#[derive(Clone, Copy)]
pub struct Record<'a>(&'a [u8; RECORD_LEN]);
impl<'a> Record<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        Ok(Self(bytes.try_into().map_err(|_| CodecError::Length)?))
    }
    pub fn bytes(self) -> &'a [u8; RECORD_LEN] {
        self.0
    }
    /// Match project.c's unpack_slot: recipe bytes and absent settings retain
    /// the caller's recipe; the old chance-in-sync field becomes zero.
    pub fn decode_legacy(self, count: u16, recipe: &[i16; 176]) -> DecodedRecord {
        let mut result = RecordValues {
            parameters: *recipe,
            pool: read_u16(self.0, 0).unwrap(),
            patterns: [0; 36],
            modifiers: [0; 96],
            acid: [0; 18],
            pluck: [0; 12],
        };
        let mut valid = scale_valid(result.pool);
        for (i, value) in result.patterns.iter_mut().enumerate() {
            *value = read_u16(self.0, 2 + i * 2).unwrap();
        }
        result.modifiers.copy_from_slice(&self.0[280..376]);
        result.acid.copy_from_slice(&self.0[256..274]);
        result.pluck[..6].copy_from_slice(&self.0[250..256]);
        result.pluck[6..].copy_from_slice(&self.0[274..280]);
        for (p, info) in PARAMETERS
            .iter()
            .enumerate()
            .take((count as usize).min(176))
        {
            if self.0[74 + p] == 255 {
                continue;
            }
            let value = info.min + self.0[74 + p] as i16;
            if p == ParamId::STYLE.index() && value > 1 {
                result.parameters[p] = 0;
                continue;
            }
            valid &= value <= info.max;
            result.parameters[p] = value.clamp(info.min, info.max);
        }
        DecodedRecord {
            values: result,
            valid,
        }
    }
    pub fn decode(self, count: u16, recipe: &[i16; 176]) -> Result<RecordValues, CodecError> {
        if count > 176 {
            return Err(CodecError::Range);
        }
        let result = self.decode_legacy(count, recipe);
        if !scale_valid(result.values.pool) {
            return Err(CodecError::Range);
        }
        if !result.valid {
            return Err(CodecError::Range);
        }
        Ok(result.values)
    }
    pub fn parameter(self, index: usize, min: i16, count: u16) -> Result<Option<i16>, CodecError> {
        if index >= 176 {
            return Err(CodecError::Index);
        }
        Ok(if index >= count as usize || self.0[74 + index] == 255 {
            None
        } else {
            Some(
                min.checked_add(self.0[74 + index] as i16)
                    .ok_or(CodecError::Range)?,
            )
        })
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        if output.len() < 376 {
            return Err(CodecError::Capacity);
        }
        output[..376].copy_from_slice(self.0);
        Ok(376)
    }
}
impl RecordValues {
    /// Legacy new-save policy zeros spare bytes and clamps byte deltas, exactly
    /// like pack_slot. Use Record::encode for a lossless existing-record copy.
    pub fn encode_legacy(&self, output: &mut [u8]) -> Result<usize, CodecError> {
        if output.len() < 376 {
            return Err(CodecError::Capacity);
        }
        output[..376].fill(0);
        output[..2].copy_from_slice(&self.pool.to_le_bytes());
        for (i, value) in self.patterns.iter().enumerate() {
            output[2 + i * 2..4 + i * 2].copy_from_slice(&value.to_le_bytes());
        }
        for (p, info) in PARAMETERS.iter().enumerate() {
            output[74 + p] = (self.parameters[p] as i32 - info.min as i32).clamp(0, 255) as u8;
        }
        output[280..376].copy_from_slice(&self.modifiers);
        output[256..274].copy_from_slice(&self.acid);
        output[250..256].copy_from_slice(&self.pluck[..6]);
        output[274..280].copy_from_slice(&self.pluck[6..]);
        Ok(376)
    }
}

/// The HYP2 record predates compact parameter bytes. Musical style/answer/voice
/// renumbering is still application policy; this view only reproduces byte
/// decoding, the historical rate/tone fields and the original validity result.
pub struct LegacyRecord<'a> {
    bytes: &'a [u8],
    count: usize,
}
pub struct LegacyValues {
    pub parameters: [i16; 176],
    pub pool: u16,
    pub patterns: [u16; 36],
    pub valid: bool,
}
impl<'a> LegacyRecord<'a> {
    pub fn parse(bytes: &'a [u8], count: u16) -> Result<Self, CodecError> {
        let count = count as usize;
        if !(1..=176).contains(&count) {
            return Err(CodecError::Range);
        }
        if bytes.len() != count * 2 + 74 {
            return Err(CodecError::Length);
        }
        Ok(Self { bytes, count })
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn decode(&self, old_tone: bool, old_rate: bool) -> LegacyValues {
        let mut parameters = [0; 176];
        let mut valid = true;
        for (p, info) in PARAMETERS.iter().enumerate() {
            let mut value = if p < self.count {
                read_u16(self.bytes, p * 2).unwrap() as i16
            } else {
                info.default
            };
            if p == ParamId::AC_EVERY.index() && old_tone {
                value = 0;
            }
            if p == ParamId::CM_RATE.index() && old_rate {
                value = ((value as i32 + 2) & 3) as i16;
            }
            valid &= p == ParamId::STYLE.index() || (info.min..=info.max).contains(&value);
            parameters[p] = value;
        }
        let pool = read_u16(self.bytes, self.count * 2).unwrap();
        valid &= scale_valid(pool);
        let mut patterns = [0; 36];
        for (i, mask) in patterns.iter_mut().enumerate() {
            *mask = read_u16(self.bytes, self.count * 2 + 2 + i * 2).unwrap();
        }
        LegacyValues {
            parameters,
            pool,
            patterns,
            valid,
        }
    }
}
#[derive(Clone, Copy)]
pub struct LegacyKit<'a>(&'a [u8]);
impl<'a> LegacyKit<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 132 {
            return Err(CodecError::Length);
        }
        if &bytes[..4] != b"HYP1" {
            return Err(CodecError::Magic);
        }
        Ok(Self(bytes))
    }
    pub fn bytes(self) -> &'a [u8] {
        self.0
    }
    pub fn theme(self) -> u8 {
        self.0[116]
    }
}
#[derive(Clone, Copy)]
pub struct Bank<'a> {
    bytes: &'a [u8],
    legacy_count: Option<usize>,
}
impl<'a> Bank<'a> {
    pub fn parse(bytes: &'a [u8], genre: u8) -> Result<Self, CodecError> {
        if genre >= 19 {
            return Err(CodecError::Index);
        }
        if bytes.len() < 20 {
            return Err(CodecError::Length);
        }
        let legacy_count = match &bytes[..4] {
            b"HYP3" => {
                if bytes[4] != 2 {
                    return Err(CodecError::Version);
                }
                if bytes[5] != genre {
                    return Err(CodecError::Index);
                }
                if read_u16(bytes, 10) != Some(376) || bytes.len() < BANK_TAIL {
                    return Err(CodecError::Length);
                }
                None
            }
            b"HYP2" if genre == 0 => {
                if read_u32(bytes, 4) != Some(1) {
                    return Err(CodecError::Version);
                }
                let count = read_u16(bytes, 8).unwrap() as usize;
                if !(1..=176).contains(&count) || bytes.len() < 20 + 10 * (count * 2 + 74) {
                    return Err(CodecError::Length);
                }
                Some(count)
            }
            _ => return Err(CodecError::Magic),
        };
        Ok(Self {
            bytes,
            legacy_count,
        })
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn parameter_count(self) -> u16 {
        self.legacy_count
            .map_or_else(|| read_u16(self.bytes, 6).unwrap(), |n| n as u16)
    }
    pub fn saved(self) -> u16 {
        read_u16(self.bytes, if self.legacy_count.is_some() { 10 } else { 8 }).unwrap()
    }
    pub fn record(self, index: usize) -> Result<Record<'a>, CodecError> {
        if index >= 10 {
            return Err(CodecError::Index);
        }
        if self.legacy_count.is_some() {
            return Err(CodecError::Version);
        }
        Record::parse(&self.bytes[16 + index * 376..16 + (index + 1) * 376])
    }
    pub fn legacy_record(self, index: usize) -> Result<&'a [u8], CodecError> {
        if index >= 10 {
            return Err(CodecError::Index);
        }
        let count = self.legacy_count.ok_or(CodecError::Version)?;
        let n = count * 2 + 74;
        Ok(&self.bytes[20 + index * n..20 + (index + 1) * n])
    }
    pub fn progression(self, index: usize) -> Result<Option<&'a [u8; 6]>, CodecError> {
        if index >= 10 {
            return Err(CodecError::Index);
        }
        if self.legacy_count.is_some() || self.bytes[14] != 1 || self.bytes.len() < BANK_LEN {
            return Ok(None);
        }
        let at = BANK_TAIL + index * 6;
        Ok(Some(self.bytes[at..at + 6].try_into().unwrap()))
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        if output.len() < self.bytes.len() {
            return Err(CodecError::Capacity);
        }
        output[..self.bytes.len()].copy_from_slice(self.bytes);
        Ok(self.bytes.len())
    }
}
#[derive(Clone, Copy)]
pub struct RestoreLimits {
    pub effects: u16,
    pub settings_len: usize,
}
/// Exact ui_restore_valid acceptance for legacy object IDs0..25, excluding the
/// device's own panel calibration. Jam/session validation remains in their modules.
pub fn restore_valid(object: u32, bytes: &[u8], limits: RestoreLimits) -> bool {
    let n = bytes.len();
    let magic = read_u32(bytes, 0);
    let word = |at| read_u16(bytes, at);
    if object == 0 || (5..=22).contains(&object) {
        let genre = if object == 0 { 0 } else { object - 4 };
        if n < 20 {
            return false;
        }
        if magic == Some(0x33505948) {
            return n >= BANK_TAIL
                && bytes[4] == 2
                && bytes[5] as u32 == genre
                && word(10) == Some(376);
        }
        return object == 0 && (magic == Some(0x32505948) || magic == Some(0x31505948));
    }
    match object {
        4 => (n == 32 || n == 12) && magic == Some(0x4c474d50),
        23 => {
            ((4..=574).contains(&n) && (n - 4).is_multiple_of(30) && magic == Some(0x31444e4b))
                || ((8..=3048).contains(&n)
                    && bytes[4] == 16
                    && (n - 8).is_multiple_of(160)
                    && magic == Some(0x31584350))
        }
        25 => {
            (8..=3048).contains(&n)
                && bytes[4] == 16
                && (n - 8).is_multiple_of(160)
                && magic == Some(0x31515342)
        }
        24 => midi_map_valid(bytes),
        3 => n == 2056 && magic == Some(0x31584f56) && read_u32(bytes, 4).is_some_and(|v| v <= 16),
        2 => {
            if n < 8 || magic != Some(0x31544553) || word(6) != Some(12) {
                return false;
            }
            let nf = word(4).unwrap();
            let at = (24 + nf as usize).next_multiple_of(2);
            (nf == limits.effects && n == limits.settings_len)
                || (nf < limits.effects && n < limits.settings_len && n >= at + 4)
        }
        _ => false,
    }
}
pub fn midi_map_valid(bytes: &[u8]) -> bool {
    if bytes.len() != 152 || (&bytes[..4] != b"MAP1" && &bytes[..4] != b"MAP2") || bytes[4] > 48 {
        return false;
    }
    for e in bytes[8..8 + bytes[4] as usize * 3].chunks_exact(3) {
        let t = e[2];
        if if e[0] & 128 != 0 {
            e[0] & 16 == 0 || t as usize >= PARAMETER_COUNT
        } else {
            !(t < 14 || (32..59).contains(&t) || (64..71).contains(&t))
        } {
            return false;
        }
        if e[1] > 127 || e[0] >> 5 & 3 == 3 {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy)]
pub struct ObjectView<'a> {
    object: u32,
    bytes: &'a [u8],
}
impl<'a> ObjectView<'a> {
    pub fn parse(object: u32, bytes: &'a [u8], limits: RestoreLimits) -> Result<Self, CodecError> {
        if !restore_valid(object, bytes, limits) {
            return Err(CodecError::Range);
        }
        Ok(Self { object, bytes })
    }
    pub fn object(self) -> u32 {
        self.object
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        if output.len() < self.bytes.len() {
            return Err(CodecError::Capacity);
        }
        output[..self.bytes.len()].copy_from_slice(self.bytes);
        Ok(self.bytes.len())
    }
    pub fn voice(self, index: usize) -> Result<&'a [u8; 128], CodecError> {
        if self.object != 3 {
            return Err(CodecError::Version);
        }
        let count = read_u32(self.bytes, 4).unwrap() as usize;
        if index >= count {
            return Err(CodecError::Index);
        }
        Ok(self.bytes[8 + index * 128..8 + (index + 1) * 128]
            .try_into()
            .unwrap())
    }
    pub fn extension(self, genre: usize, piece: usize) -> Result<Option<&'a [u8]>, CodecError> {
        if genre >= 19 || piece >= 10 {
            return Err(CodecError::Index);
        }
        if self.object != 23 && self.object != 25 {
            return Err(CodecError::Version);
        }
        let (header, size) = if &self.bytes[..4] == b"KND1" {
            (4, 3)
        } else {
            (8, 16)
        };
        let at = header + (genre * 10 + piece) * size;
        Ok(self.bytes.get(at..at + size))
    }
    pub fn set_list(self) -> Result<Option<&'a [u8; 16]>, CodecError> {
        if self.object != 4 {
            return Err(CodecError::Version);
        }
        Ok(if self.bytes.len() == 12 {
            None
        } else {
            Some(self.bytes[12..28].try_into().unwrap())
        })
    }
    pub fn midi_entries(self) -> Result<core::slice::ChunksExact<'a, u8>, CodecError> {
        if self.object != 24 {
            return Err(CodecError::Version);
        }
        Ok(self.bytes[8..8 + self.bytes[4] as usize * 3].chunks_exact(3))
    }
}
pub struct Settings<'a> {
    view: ObjectView<'a>,
    count: usize,
    tail: usize,
}
impl<'a> Settings<'a> {
    pub fn parse(bytes: &'a [u8], limits: RestoreLimits) -> Result<Self, CodecError> {
        let view = ObjectView::parse(2, bytes, limits)?;
        let count = read_u16(bytes, 4).unwrap() as usize;
        let tail = (24 + count).next_multiple_of(2);
        if bytes.len() < tail + 4 {
            return Err(CodecError::Length);
        }
        Ok(Self { view, count, tail })
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.view.bytes
    }
    pub fn effects(&self) -> &'a [u8] {
        &self.view.bytes[8..8 + self.count]
    }
    pub fn performance(&self) -> &'a [u8; 12] {
        self.view.bytes[8 + self.count..20 + self.count]
            .try_into()
            .unwrap()
    }
    pub fn targets_and_visualizer(&self) -> &'a [u8; 4] {
        self.view.bytes[20 + self.count..24 + self.count]
            .try_into()
            .unwrap()
    }
    pub fn pattern(&self) -> u16 {
        read_u16(self.view.bytes, self.tail).unwrap()
    }
    pub fn usb_flags(&self) -> u16 {
        read_u16(self.view.bytes, self.tail + 2).unwrap()
    }
}
