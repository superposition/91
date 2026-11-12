// SPDX-License-Identifier: GPL-3.0-only
//! Seven-bit, nibble and editor/backup packets; buffers belong to the caller.
use crate::CodecError;
pub const BACKUP_MAX: usize = 3840;
pub const BACKUP_CHUNK: usize = 256;
pub const BACKUP_FRAME: usize = 64;
pub fn sum7(bytes: &[u8]) -> u8 {
    0u8.wrapping_sub(bytes.iter().fold(0u8, |sum, &v| sum.wrapping_add(v))) & 127
}
fn space(output: &[u8], needed: usize) -> Result<(), CodecError> {
    if output.len() < needed {
        Err(CodecError::Capacity)
    } else {
        Ok(())
    }
}
fn frame(bytes: &[u8], kind: u8) -> Result<(), CodecError> {
    if bytes.len() < 5 || bytes[..4] != [0xf0, 0x7d, 0x50, kind] || bytes[bytes.len() - 1] != 0xf7 {
        return Err(CodecError::Magic);
    }
    if bytes[1..bytes.len() - 1].iter().any(|b| *b > 127) {
        return Err(CodecError::Range);
    }
    Ok(())
}
pub fn decode_nibbles(input: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    if !input.len().is_multiple_of(2) {
        return Err(CodecError::Length);
    }
    space(output, input.len() / 2)?;
    if input.iter().any(|v| *v > 15) {
        return Err(CodecError::Nibble);
    }
    for (o, pair) in output.iter_mut().zip(input.chunks_exact(2)) {
        *o = pair[0] << 4 | pair[1];
    }
    Ok(input.len() / 2)
}
pub fn encode_nibbles(input: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    let n = input.len().checked_mul(2).ok_or(CodecError::Length)?;
    space(output, n)?;
    for (pair, &v) in output[..n].chunks_exact_mut(2).zip(input) {
        pair[0] = v >> 4;
        pair[1] = v & 15;
    }
    Ok(n)
}
pub fn pack7(input: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    let n = input
        .len()
        .checked_mul(8)
        .and_then(|v| v.checked_add(6))
        .ok_or(CodecError::Length)?
        / 7;
    space(output, n)?;
    let (mut acc, mut bits, mut at) = (0u32, 0, 0);
    for &v in input {
        acc |= (v as u32) << bits;
        bits += 8;
        while bits >= 7 {
            output[at] = (acc & 127) as u8;
            at += 1;
            acc >>= 7;
            bits -= 7;
        }
    }
    if bits != 0 {
        output[at] = acc as u8;
        at += 1;
    }
    Ok(at)
}
/// Compatibility decoder masks bit7 like the original OTA decoder; strict
/// transport validation can reject high bits before calling it.
pub fn unpack7(input: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    let n = input.len().checked_mul(7).ok_or(CodecError::Length)? / 8;
    space(output, n)?;
    let (mut acc, mut bits, mut at) = (0u32, 0, 0);
    for &v in input {
        acc |= ((v & 127) as u32) << bits;
        bits += 7;
        if bits >= 8 {
            output[at] = acc as u8;
            at += 1;
            acc >>= 8;
            bits -= 8;
        }
    }
    Ok(at)
}
#[derive(Clone, Copy, Debug)]
pub struct BackupData<'a> {
    pub object: u8,
    pub index: u16,
    pub total: u16,
    nibbles: &'a [u8],
}
impl<'a> BackupData<'a> {
    pub fn parse_frame(bytes: &'a [u8]) -> Result<Self, CodecError> {
        frame(bytes, 0x43)?;
        if bytes.len() < 13 {
            return Err(CodecError::Length);
        }
        let n = ((bytes[9] as usize) << 7) | bytes[10] as usize;
        if n > BACKUP_FRAME || bytes.len() != 13 + 2 * n {
            return Err(CodecError::Length);
        }
        Self::body(
            bytes[4],
            &bytes[5..11],
            &bytes[11..11 + 2 * n],
            bytes[11 + 2 * n],
        )
    }
    /// The firmware's raw chunk parser sees bytes between F0/F7. Preserve its
    /// checksum-before-nibble error precedence; USB owns seven-bit filtering.
    pub fn parse_chunk_legacy(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 12 || bytes[..4] != [0x7d, 0x50, 0x42, 3] {
            return Err(CodecError::Length);
        }
        let n = ((bytes[9] as usize) << 7) | bytes[10] as usize;
        if n == 0 || n > BACKUP_CHUNK || bytes.len() != 12 + 2 * n {
            return Err(CodecError::Length);
        }
        Self::body(
            bytes[4],
            &bytes[5..11],
            &bytes[11..11 + 2 * n],
            bytes[11 + 2 * n],
        )
    }
    pub fn parse_chunk(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.iter().any(|b| *b > 127) {
            return Err(CodecError::Range);
        }
        Self::parse_chunk_legacy(bytes)
    }
    fn body(object: u8, fields: &[u8], data: &'a [u8], checksum: u8) -> Result<Self, CodecError> {
        let s = fields
            .iter()
            .chain(data)
            .fold(object, |a, &b| a.wrapping_add(b));
        if 0u8.wrapping_sub(s) & 127 != checksum {
            return Err(CodecError::Checksum);
        }
        if data.iter().any(|b| *b > 15) {
            return Err(CodecError::Nibble);
        }
        Ok(Self {
            object,
            index: (fields[0] as u16) << 7 | fields[1] as u16,
            total: (fields[2] as u16) << 7 | fields[3] as u16,
            nibbles: data,
        })
    }
    pub fn len(self) -> usize {
        self.nibbles.len() / 2
    }
    pub fn is_empty(self) -> bool {
        self.nibbles.is_empty()
    }
    pub fn decode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        decode_nibbles(self.nibbles, output)
    }
}
pub fn backup_frame(
    object: u8,
    index: u16,
    total: u16,
    data: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    backup_data(false, object, index, total, data, output)
}
pub fn backup_chunk(
    object: u8,
    index: u16,
    total: u16,
    data: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    backup_data(true, object, index, total, data, output)
}
fn backup_data(
    chunk: bool,
    object: u8,
    index: u16,
    total: u16,
    data: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if object > 127
        || index > 16383
        || total > 16383
        || data.len() > if chunk { 256 } else { 64 }
        || (chunk && data.is_empty())
    {
        return Err(CodecError::Range);
    }
    let n = 13 + usize::from(chunk) + data.len() * 2;
    space(output, n)?;
    output[..4].copy_from_slice(&[0xf0, 0x7d, 0x50, if chunk { 0x42 } else { 0x43 }]);
    let at = if chunk {
        output[4] = 3;
        5
    } else {
        4
    };
    output[at] = object;
    for (i, v) in [index, total, data.len() as u16].into_iter().enumerate() {
        output[at + 1 + i * 2] = (v >> 7) as u8;
        output[at + 2 + i * 2] = (v & 127) as u8;
    }
    encode_nibbles(data, &mut output[at + 7..n - 2])?;
    output[n - 2] = sum7(&output[at..n - 2]);
    output[n - 1] = 0xf7;
    Ok(n)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackupSession {
    object: u8,
    length: u16,
    mask: u16,
}
impl BackupSession {
    pub fn accept(&mut self, chunk: BackupData<'_>, output: &mut [u8]) -> Result<(), CodecError> {
        let off = chunk.index as usize * 256;
        let length = chunk.total as usize;
        if length == 0 || length > BACKUP_MAX || chunk.index >= 16 || off + chunk.len() > length {
            return Err(CodecError::Range);
        }
        if chunk.len() != 256.min(length - off) {
            return Err(CodecError::Length);
        }
        if chunk.index != 0
            && (self.mask == 0 || self.object != chunk.object || self.length != chunk.total)
        {
            return Err(CodecError::State);
        }
        space(output, length)?;
        chunk.decode(&mut output[off..off + chunk.len()])?;
        if chunk.index == 0 {
            self.object = chunk.object;
            self.length = chunk.total;
            self.mask = 0;
        }
        self.mask |= 1 << chunk.index;
        Ok(())
    }
    pub fn complete(self, object: u8, length: u16) -> bool {
        length != 0
            && length as usize <= BACKUP_MAX
            && self.object == object
            && self.length == length
            && self.mask == ((1u16 << (length as usize).div_ceil(256)) - 1)
    }
}
pub fn editor_dump(first: u16, values: &[i32], output: &mut [u8]) -> Result<usize, CodecError> {
    if first > 16383 || values.len() > 24 {
        return Err(CodecError::Range);
    }
    let n = 8 + values.len() * 2;
    space(output, n)?;
    output[..7].copy_from_slice(&[
        0xf0,
        0x7d,
        0x50,
        0x51,
        (first >> 7) as u8,
        (first & 127) as u8,
        values.len() as u8,
    ]);
    for (i, value) in values.iter().enumerate() {
        let v = value.wrapping_add(8192) as u32 & 16383;
        output[7 + i * 2] = (v >> 7) as u8;
        output[8 + i * 2] = (v & 127) as u8;
    }
    output[n - 1] = 0xf7;
    Ok(n)
}
pub fn editor_voice(
    slot: u8,
    loaded: u8,
    voice: Option<&[u8; 128]>,
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if slot > 127 || loaded > 127 {
        return Err(CodecError::Range);
    }
    let n = if voice.is_some() { 136 } else { 7 };
    space(output, n)?;
    output[..6].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x58, slot, loaded]);
    if let Some(v) = voice {
        for (i, b) in v.iter().enumerate() {
            output[6 + i] = b & 127;
        }
        output[134] = sum7(&output[6..134]);
    }
    output[n - 1] = 0xf7;
    Ok(n)
}
pub fn backup_ack(
    object: u8,
    index: u8,
    status: u8,
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if object > 127 || index > 127 || status > 127 {
        return Err(CodecError::Range);
    }
    space(output, 8)?;
    output[..8].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x44, object, index, status, 0xf7]);
    Ok(8)
}
pub fn about(
    version: u8,
    parameters: u16,
    genres: u8,
    voices: u8,
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if version > 127 || parameters > 16383 || genres > 127 || voices > 127 {
        return Err(CodecError::Range);
    }
    space(output, 10)?;
    output[..10].copy_from_slice(&[
        0xf0,
        0x7d,
        0x50,
        0x45,
        version,
        (parameters >> 7) as u8,
        (parameters & 127) as u8,
        genres,
        voices,
        0xf7,
    ]);
    Ok(10)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlReply {
    BackupAck {
        object: u8,
        index: u8,
        status: u8,
    },
    About {
        version: u8,
        parameters: u16,
        genres: u8,
        voices: u8,
    },
    VoiceAck {
        kind: u8,
        count: u8,
    },
}
impl ControlReply {
    pub fn parse(bytes: &[u8]) -> Result<Self, CodecError> {
        if bytes.len() < 5 {
            return Err(CodecError::Length);
        }
        frame(bytes, bytes[3])?;
        match (bytes[3], bytes.len()) {
            (0x44, 8) => Ok(Self::BackupAck {
                object: bytes[4],
                index: bytes[5],
                status: bytes[6],
            }),
            (0x45, 10) => Ok(Self::About {
                version: bytes[4],
                parameters: (bytes[5] as u16) << 7 | bytes[6] as u16,
                genres: bytes[7],
                voices: bytes[8],
            }),
            (0x57, 7) => Ok(Self::VoiceAck {
                kind: bytes[4],
                count: bytes[5],
            }),
            _ => Err(CodecError::Length),
        }
    }
}
#[derive(Clone, Copy)]
pub struct EditorDump<'a>(&'a [u8]);
impl<'a> EditorDump<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        frame(bytes, 0x51)?;
        if bytes.len() < 8 || bytes[6] > 24 || bytes.len() != 8 + bytes[6] as usize * 2 {
            return Err(CodecError::Length);
        }
        Ok(Self(bytes))
    }
    pub fn first(self) -> u16 {
        (self.0[4] as u16) << 7 | self.0[5] as u16
    }
    pub fn len(self) -> usize {
        self.0[6] as usize
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn value(self, index: usize) -> Result<i16, CodecError> {
        if index >= self.len() {
            return Err(CodecError::Index);
        }
        Ok((((self.0[7 + index * 2] as u16) << 7) | self.0[8 + index * 2] as u16) as i16 - 8192)
    }
}
#[derive(Clone, Copy)]
pub struct VoiceReply<'a>(&'a [u8]);
impl<'a> VoiceReply<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        frame(bytes, 0x58)?;
        if bytes.len() != 7 && bytes.len() != 136 {
            return Err(CodecError::Length);
        }
        if bytes.len() == 136 && sum7(&bytes[6..134]) != bytes[134] {
            return Err(CodecError::Checksum);
        }
        Ok(Self(bytes))
    }
    pub fn slot(self) -> u8 {
        self.0[4]
    }
    pub fn loaded(self) -> u8 {
        self.0[5]
    }
    pub fn voice(self) -> Option<&'a [u8; 128]> {
        if self.0.len() == 7 {
            None
        } else {
            Some(self.0[6..134].try_into().unwrap())
        }
    }
}
#[derive(Clone, Copy)]
pub struct JamRequest<'a>(&'a [u8]);
impl<'a> JamRequest<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 6 || bytes.len() > 4112 || bytes[..3] != [0x7d, 0x50, 0x60] {
            return Err(CodecError::Length);
        }
        if sum7(&bytes[3..bytes.len() - 1]) != bytes[bytes.len() - 1] {
            return Err(CodecError::Checksum);
        }
        if bytes.iter().any(|v| *v > 127) {
            return Err(CodecError::Range);
        }
        Ok(Self(bytes))
    }
    pub fn operation(self) -> u8 {
        self.0[3]
    }
    pub fn token(self) -> u8 {
        self.0[4]
    }
    pub fn payload(self) -> &'a [u8] {
        &self.0[5..self.0.len() - 1]
    }
}
#[derive(Clone, Copy)]
pub struct JamReply<'a>(&'a [u8]);
impl<'a> JamReply<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        frame(bytes, 0x61)?;
        if bytes.len() < 9 || bytes.len() > 145 || !(bytes.len() - 9).is_multiple_of(2) {
            return Err(CodecError::Length);
        }
        if sum7(&bytes[4..bytes.len() - 2]) != bytes[bytes.len() - 2] {
            return Err(CodecError::Checksum);
        }
        if bytes[7..bytes.len() - 2].iter().any(|v| *v > 15) {
            return Err(CodecError::Nibble);
        }
        Ok(Self(bytes))
    }
    pub fn operation(self) -> u8 {
        self.0[4]
    }
    pub fn token(self) -> u8 {
        self.0[5]
    }
    pub fn status(self) -> u8 {
        self.0[6]
    }
    pub fn decode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        decode_nibbles(&self.0[7..self.0.len() - 2], output)
    }
}
/// Opaque raw payload as nibbles; shared Jam reply envelope, not transfer policy.
pub fn jam_reply(
    op: u8,
    token: u8,
    status: u8,
    raw: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if op > 127 || token > 127 || status > 127 || raw.len() > 68 {
        return Err(CodecError::Range);
    }
    let n = 9 + raw.len() * 2;
    space(output, n)?;
    output[..7].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x61, op, token, status]);
    encode_nibbles(raw, &mut output[7..n - 2])?;
    output[n - 2] = sum7(&output[4..n - 2]);
    output[n - 1] = 0xf7;
    Ok(n)
}

pub struct OtaMessage<'a> {
    bytes: &'a [u8],
}
impl<'a> OtaMessage<'a> {
    pub fn parse_decoded(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 7 {
            return Err(CodecError::Length);
        }
        if bytes[..2] != [0, 0x59] {
            return Err(CodecError::Magic);
        }
        let n = bytes[3] as usize | ((bytes[4] as usize) << 8) | ((bytes[5] as usize) << 16);
        if bytes.len() != 7 + n {
            return Err(CodecError::Length);
        }
        if !bytes[6..6 + n].iter().fold(0u8, |a, &b| a.wrapping_add(b)) != bytes[6 + n] {
            return Err(CodecError::Checksum);
        }
        Ok(Self { bytes })
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn command(&self) -> u8 {
        self.bytes[2]
    }
    pub fn body(&self) -> &'a [u8] {
        &self.bytes[6..self.bytes.len() - 1]
    }
}
pub fn decode_ota<'a>(wire: &[u8], scratch: &'a mut [u8]) -> Result<OtaMessage<'a>, CodecError> {
    if wire.len() < 2 || wire[0] != 0xf0 || wire[wire.len() - 1] != 0xf7 {
        return Err(CodecError::Magic);
    }
    if wire[1..wire.len() - 1].iter().any(|v| *v > 127) {
        return Err(CodecError::Range);
    }
    let n = unpack7(&wire[1..wire.len() - 1], scratch)?;
    OtaMessage::parse_decoded(&scratch[..n])
}
pub fn encode_ota(command: u8, body: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    if body.len() > 0xffffff {
        return Err(CodecError::Length);
    }
    let n = 2
        + (7 + body.len())
            .checked_mul(8)
            .and_then(|n| n.checked_add(6))
            .ok_or(CodecError::Length)?
            / 7;
    space(output, n)?;
    let len = body.len() as u32;
    let header = [
        0,
        0x59,
        command,
        len as u8,
        (len >> 8) as u8,
        (len >> 16) as u8,
    ];
    let checksum = [!body.iter().fold(0u8, |a, &b| a.wrapping_add(b))];
    let (mut acc, mut bits, mut at) = (0u32, 0, 1);
    output[0] = 0xf0;
    for &v in header.iter().chain(body).chain(checksum.iter()) {
        acc |= (v as u32) << bits;
        bits += 8;
        while bits >= 7 {
            output[at] = (acc & 127) as u8;
            at += 1;
            acc >>= 7;
            bits -= 7;
        }
    }
    if bits != 0 {
        output[at] = acc as u8;
        at += 1;
    }
    output[at] = 0xf7;
    Ok(n)
}
