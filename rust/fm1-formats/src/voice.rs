// SPDX-License-Identifier: GPL-3.0-only
//! DX7 VMEM/VCED conversion, matching the firmware's masking and sanitization.
//! Raw borrowed views retain every byte; conversion deliberately canonicalizes.
use crate::CodecError;
pub const PACKED_LEN: usize = 128;
pub const VOICE_LEN: usize = 155;

#[derive(Clone, Copy)]
pub struct PackedVoice<'a>(&'a [u8; PACKED_LEN]);
impl<'a> PackedVoice<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        Ok(Self(bytes.try_into().map_err(|_| CodecError::Length)?))
    }
    pub fn bytes(self) -> &'a [u8; PACKED_LEN] {
        self.0
    }
    pub fn unpack(self) -> [u8; VOICE_LEN] {
        unpack(self.0)
    }
}
#[derive(Clone, Copy)]
pub struct Voice<'a>(&'a [u8; VOICE_LEN]);
impl<'a> Voice<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        Ok(Self(bytes.try_into().map_err(|_| CodecError::Length)?))
    }
    pub fn bytes(self) -> &'a [u8; VOICE_LEN] {
        self.0
    }
    pub fn pack(self) -> [u8; PACKED_LEN] {
        pack(self.0)
    }
}
pub fn unpack(b: &[u8; PACKED_LEN]) -> [u8; VOICE_LEN] {
    let mut v = [0; VOICE_LEN];
    for k in 0..6 {
        let o = &b[k * 17..k * 17 + 17];
        let d = &mut v[k * 21..k * 21 + 21];
        for i in 0..11 {
            d[i] = o[i] & 127;
        }
        d[11] = o[11] & 3;
        d[12] = (o[11] >> 2) & 3;
        d[13] = o[12] & 7;
        d[20] = (o[12] >> 3) & 15;
        d[14] = o[13] & 3;
        d[15] = (o[13] >> 2) & 7;
        d[16] = o[14] & 127;
        d[17] = o[15] & 1;
        d[18] = (o[15] >> 1) & 31;
        d[19] = o[16] & 127;
    }
    for i in 0..9 {
        v[126 + i] = b[102 + i] & 127;
    }
    v[134] &= 31;
    v[135] = b[111] & 7;
    v[136] = (b[111] >> 3) & 1;
    for i in 0..4 {
        v[137 + i] = b[112 + i] & 127;
    }
    v[141] = b[116] & 1;
    v[142] = (b[116] >> 1) & 7;
    v[143] = (b[116] >> 4) & 7;
    v[144] = b[117] & 127;
    for i in 0..10 {
        v[145 + i] = b[118 + i] & 127;
    }
    sanitize(&mut v);
    v
}
/// Checked shared limits for a canonical155-byte voice (including name bytes).
pub const fn maximum(index: usize) -> Option<u8> {
    const OP_MAX: [u8; 21] = [
        99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 3, 3, 7, 3, 7, 99, 1, 31, 99, 14,
    ];
    const GLOBAL_MAX: [u8; 19] = [
        99, 99, 99, 99, 99, 99, 99, 99, 31, 7, 1, 99, 99, 99, 99, 1, 5, 7, 48,
    ];
    if index < 126 {
        Some(OP_MAX[index % 21])
    } else if index < 145 {
        Some(GLOBAL_MAX[index - 126])
    } else if index < 155 {
        Some(126)
    } else {
        None
    }
}
pub fn sanitize(v: &mut [u8; 155]) {
    for (i, value) in v.iter_mut().enumerate() {
        if i >= 145 {
            if !(32..=126).contains(value) {
                *value = b' ';
            }
        } else {
            *value = (*value).min(maximum(i).unwrap());
        }
    }
}
pub fn pack(v: &[u8; VOICE_LEN]) -> [u8; PACKED_LEN] {
    let mut b = [0; PACKED_LEN];
    for k in 0..6 {
        let d = &v[k * 21..k * 21 + 21];
        let o = &mut b[k * 17..k * 17 + 17];
        for i in 0..11 {
            o[i] = d[i] & 127;
        }
        o[11] = (d[11] & 3) | ((d[12] & 3) << 2);
        o[12] = (d[13] & 7) | ((d[20] & 15) << 3);
        o[13] = (d[14] & 3) | ((d[15] & 7) << 2);
        o[14] = d[16] & 127;
        o[15] = (d[17] & 1) | ((d[18] & 31) << 1);
        o[16] = d[19] & 127;
    }
    for i in 0..9 {
        b[102 + i] = v[126 + i] & 127;
    }
    b[111] = (v[135] & 7) | ((v[136] & 1) << 3);
    for i in 0..4 {
        b[112 + i] = v[137 + i] & 127;
    }
    b[116] = (v[141] & 1) | ((v[142] & 7) << 1) | ((v[143] & 7) << 4);
    b[117] = v[144] & 127;
    for i in 0..10 {
        b[118 + i] = v[145 + i] & 127;
    }
    b
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IncomingKind {
    Bulk = 1,
    Single = 2,
    Slot = 3,
    Clear = 4,
    BackupChunk = 5,
    Jam = 6,
}
/// Original USB collector classification, before checksum/service policy. Input
/// is the bytes between F0/F7; seven-bit filtering belongs to the collector.
pub fn classify_legacy(m: &[u8]) -> Option<IncomingKind> {
    let n = m.len();
    let mut kind = None;
    if n >= 5 && m[0] == 0x43 && m[1] & 0xf0 == 0 {
        if m[2..5] == [9, 32, 0] && n == 4102 {
            kind = Some(IncomingKind::Bulk);
        } else if m[2..5] == [0, 1, 27] && n == 161 {
            kind = Some(IncomingKind::Single);
        }
    } else if n >= 4 && m[..3] == [0x7d, 0x50, 0x56] {
        if n == 133 && m[3] < 16 {
            kind = Some(IncomingKind::Slot);
        } else if n == 4 && m[3] == 127 {
            kind = Some(IncomingKind::Clear);
        }
    } else if n >= 12 && m[..4] == [0x7d, 0x50, 0x42, 3] {
        kind = Some(IncomingKind::BackupChunk);
    }
    if n >= 6 && m[..3] == [0x7d, 0x50, 0x60] {
        kind = Some(IncomingKind::Jam);
    }
    kind
}
#[derive(Clone, Copy)]
pub struct DxMessage<'a> {
    bytes: &'a [u8],
    kind: IncomingKind,
}
impl<'a> DxMessage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 6 || bytes[0] != 0xf0 || bytes[bytes.len() - 1] != 0xf7 {
            return Err(CodecError::Magic);
        }
        let inner = &bytes[1..bytes.len() - 1];
        if inner.iter().any(|v| *v > 127) {
            return Err(CodecError::Range);
        }
        let kind = classify_legacy(inner).ok_or(CodecError::Length)?;
        if kind == IncomingKind::BackupChunk || kind == IncomingKind::Jam {
            return Err(CodecError::Version);
        }
        if kind != IncomingKind::Clear {
            let start = if kind == IncomingKind::Slot { 4 } else { 5 };
            if crate::wire::sum7(&inner[start..inner.len() - 1]) != inner[inner.len() - 1] {
                return Err(CodecError::Checksum);
            }
        }
        Ok(Self { bytes, kind })
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn kind(self) -> IncomingKind {
        self.kind
    }
    pub fn slot(self) -> Option<u8> {
        if self.kind == IncomingKind::Slot {
            Some(self.bytes[4])
        } else {
            None
        }
    }
    pub fn channel(self) -> Option<u8> {
        if matches!(self.kind, IncomingKind::Bulk | IncomingKind::Single) {
            Some(self.bytes[2])
        } else {
            None
        }
    }
    pub fn single(self) -> Option<&'a [u8; 155]> {
        if self.kind == IncomingKind::Single {
            Some(self.bytes[6..161].try_into().unwrap())
        } else {
            None
        }
    }
    pub fn packed(self, index: usize) -> Result<&'a [u8; 128], CodecError> {
        let (start, count) = match self.kind {
            IncomingKind::Bulk => (6, 32),
            IncomingKind::Slot => (5, 1),
            _ => return Err(CodecError::Version),
        };
        if index >= count {
            return Err(CodecError::Index);
        }
        let at = start + index * 128;
        Ok(self.bytes[at..at + 128].try_into().unwrap())
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        if output.len() < self.bytes.len() {
            return Err(CodecError::Capacity);
        }
        output[..self.bytes.len()].copy_from_slice(self.bytes);
        Ok(self.bytes.len())
    }
}
pub fn dx7_single(channel: u8, voice: &[u8; 155], output: &mut [u8]) -> Result<usize, CodecError> {
    dx7_yamaha(channel, false, voice, output)
}
pub fn dx7_bulk(channel: u8, voices: &[u8; 4096], output: &mut [u8]) -> Result<usize, CodecError> {
    dx7_yamaha(channel, true, voices, output)
}
fn dx7_yamaha(
    channel: u8,
    bulk: bool,
    payload: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if channel > 15 || payload.iter().any(|v| *v > 127) {
        return Err(CodecError::Range);
    }
    let n = payload.len() + 8;
    if output.len() < n {
        return Err(CodecError::Capacity);
    }
    output[..6].copy_from_slice(&[
        0xf0,
        0x43,
        channel,
        if bulk { 9 } else { 0 },
        if bulk { 32 } else { 1 },
        if bulk { 0 } else { 27 },
    ]);
    output[6..n - 2].copy_from_slice(payload);
    output[n - 2] = crate::wire::sum7(payload);
    output[n - 1] = 0xf7;
    Ok(n)
}
pub fn slot_voice(
    slot: u8,
    voice: Option<&[u8; 128]>,
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if voice.is_some_and(|v| slot >= 16 || v.iter().any(|v| *v > 127)) {
        return Err(CodecError::Range);
    }
    let n = if voice.is_some() { 135 } else { 6 };
    if output.len() < n {
        return Err(CodecError::Capacity);
    }
    output[..5].copy_from_slice(&[
        0xf0,
        0x7d,
        0x50,
        0x56,
        if voice.is_some() { slot } else { 127 },
    ]);
    if let Some(v) = voice {
        output[5..133].copy_from_slice(v);
        output[133] = crate::wire::sum7(v);
    }
    output[n - 1] = 0xf7;
    Ok(n)
}
