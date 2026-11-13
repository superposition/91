// SPDX-License-Identifier: GPL-3.0-only
use fm1_types::{ParamId, STORE_MAX};

pub const NODE: usize = 64;
pub const EDGE: usize = 192;
pub const FORM: usize = 448;
pub const BASS: usize = 576;
pub const DRUM: usize = 864;
pub const SOUND: usize = 992;
pub const SCALE: usize = 40;
pub const MIN_V1: usize = 1120;
pub const V2_HEAD: usize = 1120;
pub const V2_EXTRA: usize = 1152;
pub const V2_CHART: usize = 1280;
pub const V2_VOICE: usize = 1536;
pub const MIN_V2: usize = 2560;
pub const V3_QUALITIES: u8 = 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackError {
    Length,
    Header,
    Destination,
    Scale,
    Node,
    Edge,
    Form,
    Bass,
    Drum,
    Song,
    Chart,
    Voice,
    Sound,
}

/// A validated, immutable view; no copy, allocation or expanded graph is needed.
#[derive(Clone, Copy, Debug)]
pub struct JamPack<'a> {
    bytes: &'a [u8],
}

impl<'a> JamPack<'a> {
    /// Copy a validated pack without changing unknown metadata/reserved bytes.
    pub fn encode_into(self, output: &mut [u8]) -> Result<usize, crate::CodecError> {
        if output.len() < self.bytes.len() {
            return Err(crate::CodecError::Capacity);
        }
        output[..self.bytes.len()].copy_from_slice(self.bytes);
        Ok(self.bytes.len())
    }
    pub fn parse(p: &'a [u8]) -> Result<Self, PackError> {
        let pack = Self::parse_structural(p)?;
        for i in 0..p[8] as usize {
            let offset = SOUND + i * 4;
            let id = u16::from_le_bytes([p[offset], p[offset + 1]]);
            let value = i16::from_le_bytes([p[offset + 2], p[offset + 3]]);
            let parameter = ParamId::new(id as usize).map_err(|_| PackError::Sound)?;
            if parameter.is_global()
                || parameter == ParamId::TEMPO
                || parameter == ParamId::ROOT
                || parameter == ParamId::MODE
                || !parameter.info().contains(value)
            {
                return Err(PackError::Sound);
            }
        }
        Ok(pack)
    }

    /// Validate layout and musical references, matching the legacy `jp_valid`.
    /// Sound-parameter policy is additionally checked by [`Self::parse`].
    pub fn parse_structural(p: &'a [u8]) -> Result<Self, PackError> {
        if p.len() < MIN_V1 || p.len() > STORE_MAX {
            return Err(PackError::Length);
        }
        if &p[..3] != b"JMP"
            || !(1..=3).contains(&p[4])
            || p[3] != b'0' + p[4]
            || p[5] == 0
            || p[5] > if p[4] >= 2 { 32 } else { 16 }
            || p[6] == 0
            || p[6] > 8
            || p[7] == 0
            || p[7] > 8
            || p[8] > 32
            || p[39] != 0
        {
            return Err(PackError::Header);
        }
        let v2 = p[4] >= 2;
        let v3 = p[4] == 3;
        if v2 && p.len() < MIN_V2 {
            return Err(PackError::Length);
        }
        let pack = Self { bytes: p };
        for (index, &destination) in p[9..15].iter().enumerate() {
            let node = pack
                .node(destination as usize)
                .ok_or(PackError::Destination)?;
            if !v3 && node[4] != u8::from(index >= 3) {
                return Err(PackError::Destination);
            }
        }
        for mode in 0..2 {
            for j in 0..8 {
                let value = p[SCALE + mode * 8 + j];
                if value > 11
                    || (j == 0 && value != 0)
                    || (j != 0 && value <= p[SCALE + mode * 8 + j - 1])
                {
                    return Err(PackError::Scale);
                }
            }
        }
        for i in 0..p[5] as usize {
            let node = pack.node(i).ok_or(PackError::Node)?;
            if node[0] > 11
                || node[1] > 11
                || node[2] > 1
                || node[3] > if v2 { 7 } else { 5 }
                || node[4] > 1
                || node[5] > 15
                || (v2 && node[6] > 15)
                || (v3 && node[7] >= V3_QUALITIES)
            {
                return Err(PackError::Node);
            }
            if !v2 {
                let mut sum = 0u32;
                for j in 0..16 {
                    let weight = p[EDGE + i * 16 + j];
                    if weight != 0 {
                        let destination = pack.node(j).ok_or(PackError::Edge)?;
                        if destination[4] != node[4] {
                            return Err(PackError::Edge);
                        }
                    }
                    sum += weight as u32;
                }
                if sum == 0 {
                    return Err(PackError::Edge);
                }
            }
        }
        for i in 0..4 {
            let form = &p[FORM + i * 32..FORM + (i + 1) * 32];
            if form[15] != 0 || form[16] == 0 || form[16] > 8 {
                return Err(PackError::Form);
            }
            for &role in &form[17..17 + form[16] as usize] {
                if role > 3 {
                    return Err(PackError::Form);
                }
            }
        }
        for i in 0..p[6] as usize {
            let bass = &p[BASS + i * 36..BASS + (i + 1) * 36];
            if bass[32] == 0 || bass[32] > 100 {
                return Err(PackError::Bass);
            }
            for j in 0..16 {
                let note = bass[j] as i8;
                if (!(-12..=12).contains(&note) && !(-128..=-125).contains(&note))
                    || bass[16 + j] > 127
                {
                    return Err(PackError::Bass);
                }
            }
        }
        for i in 0..p[7] as usize {
            if p[DRUM + i * 16 + 14] > 100 {
                return Err(PackError::Drum);
            }
        }
        if v2 {
            let head = &p[V2_HEAD..];
            if &head[..4] != if v3 { b"SNG3" } else { b"SNG2" }
                || head[4] == 0
                || head[4] > 8
                || head[5] >= head[4]
                || head[6] >= head[4]
                || head[7] >= 24
                || head[11] > 3
                || head[12] > 2
                || head[16] == 0
                || head[16] > 100
                || head[17] == 0
                || head[17] > 100
                || (v3 && (head[21] > 1 || head[22] > 3))
                || (v3
                    && (!(-2..=2).contains(&(head[23] as i8))
                        || !(-2..=2).contains(&(head[24] as i8))))
            {
                return Err(PackError::Song);
            }
            for &value in &head[8..=10] {
                if value > 100 {
                    return Err(PackError::Song);
                }
            }
            for &value in &head[18..=20] {
                if value > 100 {
                    return Err(PackError::Song);
                }
            }
            for i in 0..256 {
                let node = p[V2_CHART + i] as usize;
                let destination = pack.node(node).ok_or(PackError::Chart)?;
                if !v3 && destination[4] != (i / 128) as u8 {
                    return Err(PackError::Chart);
                }
            }
            for &value in &p[V2_VOICE..V2_VOICE + head[4] as usize * 128] {
                if value > 127 {
                    return Err(PackError::Voice);
                }
            }
        }
        Ok(pack)
    }

    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub const fn version(self) -> u8 {
        self.bytes[4]
    }
    pub const fn node_count(self) -> usize {
        self.bytes[5] as usize
    }
    pub fn name(self) -> &'a [u8] {
        let name = &self.bytes[16..40];
        &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())]
    }

    /// Return a borrowed node only when the index belongs to this pack.
    pub fn node(self, index: usize) -> Option<&'a [u8]> {
        if index >= self.node_count() {
            return None;
        }
        let offset = if index < 16 {
            NODE + index * 8
        } else {
            V2_EXTRA + (index - 16) * 8
        };
        Some(&self.bytes[offset..offset + 8])
    }
}
