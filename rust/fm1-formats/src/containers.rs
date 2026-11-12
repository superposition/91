// SPDX-License-Identifier: GPL-3.0-only
//! P1PC, P1ST and P1BK files. JSON is an opaque borrowed byte string: neither
//! unknown keys nor the original text/ordering are lost by a binary round trip.
use crate::{crc32, read_u16, read_u32, CodecError};
pub const RECORD_LEN: usize = 376;
const SET_RECORD: usize = 5 + RECORD_LEN + 6;
fn checked(bytes: &[u8], magic: &[u8; 4], min: usize) -> Result<(), CodecError> {
    if bytes.len() < min {
        return Err(CodecError::Length);
    }
    if &bytes[..4] != magic {
        return Err(CodecError::Magic);
    }
    if bytes[4] != 1 {
        return Err(CodecError::Version);
    }
    if crc32(&bytes[..bytes.len() - 4]) != read_u32(bytes, bytes.len() - 4).unwrap() {
        return Err(CodecError::Checksum);
    }
    Ok(())
}
fn copy(bytes: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    if output.len() < bytes.len() {
        return Err(CodecError::Capacity);
    }
    output[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}
fn finish(output: &mut [u8], n: usize) {
    let crc = crc32(&output[..n - 4]);
    output[n - 4..n].copy_from_slice(&crc.to_le_bytes());
}
fn ids(genre: u8, slot: u8, npar: u16) -> Result<(), CodecError> {
    if genre >= 19 || slot >= 10 {
        return Err(CodecError::Index);
    }
    if npar > 176 {
        return Err(CodecError::Range);
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub struct PieceFile<'a> {
    bytes: &'a [u8],
    voice_len: usize,
    metadata_at: usize,
}
impl<'a> PieceFile<'a> {
    /// Exact browser file-layout compatibility; use `parse` for current indices.
    pub fn parse_compatible(bytes: &'a [u8]) -> Result<Self, CodecError> {
        checked(bytes, b"P1PC", 18)?;
        let voice_len = if bytes[7] & 1 != 0 { 128 } else { 0 };
        let metadata_at = 14 + RECORD_LEN + 6 + voice_len;
        if read_u16(bytes, 10) != Some(RECORD_LEN as u16)
            || metadata_at + read_u16(bytes, 12).unwrap() as usize + 4 != bytes.len()
        {
            return Err(CodecError::Length);
        }
        Ok(Self {
            bytes,
            voice_len,
            metadata_at,
        })
    }
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        let view = Self::parse_compatible(bytes)?;
        ids(view.genre(), view.slot(), view.parameter_count())?;
        Ok(view)
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn genre(self) -> u8 {
        self.bytes[5]
    }
    pub fn slot(self) -> u8 {
        self.bytes[6]
    }
    pub fn flags(self) -> u8 {
        self.bytes[7]
    }
    pub fn parameter_count(self) -> u16 {
        read_u16(self.bytes, 8).unwrap()
    }
    pub fn record(self) -> &'a [u8; RECORD_LEN] {
        self.bytes[14..14 + RECORD_LEN].try_into().unwrap()
    }
    pub fn progression(self) -> &'a [u8; 6] {
        self.bytes[14 + RECORD_LEN..14 + RECORD_LEN + 6]
            .try_into()
            .unwrap()
    }
    pub fn voice(self) -> Option<&'a [u8; 128]> {
        if self.voice_len == 0 {
            None
        } else {
            Some(
                self.bytes[self.metadata_at - 128..self.metadata_at]
                    .try_into()
                    .unwrap(),
            )
        }
    }
    pub fn metadata(self) -> &'a [u8] {
        &self.bytes[self.metadata_at..self.bytes.len() - 4]
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        copy(self.bytes, output)
    }
}
pub struct PieceParts<'a> {
    pub genre: u8,
    pub slot: u8,
    pub flags: u8,
    pub parameter_count: u16,
    pub record: &'a [u8; RECORD_LEN],
    pub progression: &'a [u8; 6],
    pub voice: Option<&'a [u8; 128]>,
    pub metadata: &'a [u8],
}
impl PieceParts<'_> {
    pub fn encode(&self, output: &mut [u8]) -> Result<usize, CodecError> {
        ids(self.genre, self.slot, self.parameter_count)?;
        self.encode_compatible(output)
    }
    /// Layout-only encoding retains unknown index/count fields during a raw
    /// file roundtrip; application compatibility is a separate checked policy.
    pub fn encode_compatible(&self, output: &mut [u8]) -> Result<usize, CodecError> {
        if self.metadata.len() > 65535 {
            return Err(CodecError::Length);
        }
        let n = 14 + RECORD_LEN + 6 + self.voice.map_or(0, |_| 128) + self.metadata.len() + 4;
        if output.len() < n {
            return Err(CodecError::Capacity);
        }
        output[..8].copy_from_slice(&[
            b'P',
            b'1',
            b'P',
            b'C',
            1,
            self.genre,
            self.slot,
            (self.flags & !1) | u8::from(self.voice.is_some()),
        ]);
        output[8..10].copy_from_slice(&self.parameter_count.to_le_bytes());
        output[10..12].copy_from_slice(&(RECORD_LEN as u16).to_le_bytes());
        output[12..14].copy_from_slice(&(self.metadata.len() as u16).to_le_bytes());
        let mut at = 14;
        for data in [
            self.record.as_slice(),
            self.progression.as_slice(),
            self.voice.map_or(&[][..], |v| v.as_slice()),
            self.metadata,
        ] {
            output[at..at + data.len()].copy_from_slice(data);
            at += data.len();
        }
        finish(output, n);
        Ok(n)
    }
}
#[derive(Clone, Copy)]
pub struct SetFile<'a> {
    bytes: &'a [u8],
    pieces: usize,
    voices: usize,
    metadata_at: usize,
}
#[derive(Clone, Copy)]
pub struct SetPiece<'a> {
    pub genre: u8,
    pub slot: u8,
    pub voice_index: u8,
    pub parameter_count: u16,
    pub record: &'a [u8; RECORD_LEN],
    pub progression: &'a [u8; 6],
}
impl<'a> SetFile<'a> {
    pub fn parse_compatible(bytes: &'a [u8]) -> Result<Self, CodecError> {
        checked(bytes, b"P1ST", 30)?;
        let pieces = bytes[23] as usize;
        let voices = bytes[24] as usize;
        let metadata_at = 27 + pieces * SET_RECORD + voices * 128;
        if metadata_at + read_u16(bytes, 25).unwrap() as usize + 4 != bytes.len() {
            return Err(CodecError::Length);
        }
        Ok(Self {
            bytes,
            pieces,
            voices,
            metadata_at,
        })
    }
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        let view = Self::parse_compatible(bytes)?;
        if view.voices > 16 || view.bytes[5..21].iter().any(|v| *v > 190) {
            return Err(CodecError::Index);
        }
        for i in 0..view.pieces {
            let p = view.piece(i).unwrap();
            ids(p.genre, p.slot, p.parameter_count)?;
            if p.voice_index != 255 && p.voice_index as usize >= view.voices {
                return Err(CodecError::Index);
            }
        }
        Ok(view)
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn list(self) -> &'a [u8; 16] {
        self.bytes[5..21].try_into().unwrap()
    }
    pub fn play(self) -> u8 {
        self.bytes[21]
    }
    pub fn mix(self) -> u8 {
        self.bytes[22]
    }
    pub fn piece_count(self) -> usize {
        self.pieces
    }
    pub fn voice_count(self) -> usize {
        self.voices
    }
    pub fn piece(self, index: usize) -> Result<SetPiece<'a>, CodecError> {
        if index >= self.pieces {
            return Err(CodecError::Index);
        }
        let at = 27 + index * SET_RECORD;
        Ok(SetPiece {
            genre: self.bytes[at],
            slot: self.bytes[at + 1],
            voice_index: self.bytes[at + 2],
            parameter_count: read_u16(self.bytes, at + 3).unwrap(),
            record: self.bytes[at + 5..at + 5 + RECORD_LEN].try_into().unwrap(),
            progression: self.bytes[at + 5 + RECORD_LEN..at + SET_RECORD]
                .try_into()
                .unwrap(),
        })
    }
    pub fn voice(self, index: usize) -> Result<&'a [u8; 128], CodecError> {
        if index >= self.voices {
            return Err(CodecError::Index);
        }
        let at = 27 + self.pieces * SET_RECORD + index * 128;
        Ok(self.bytes[at..at + 128].try_into().unwrap())
    }
    pub fn metadata(self) -> &'a [u8] {
        &self.bytes[self.metadata_at..self.bytes.len() - 4]
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        copy(self.bytes, output)
    }
}
pub struct SetParts<'a> {
    pub list: &'a [u8; 16],
    pub play: u8,
    pub mix: u8,
    pub pieces: &'a [SetPiece<'a>],
    pub voices: &'a [[u8; 128]],
    pub metadata: &'a [u8],
}
impl SetParts<'_> {
    pub fn encode(&self, output: &mut [u8]) -> Result<usize, CodecError> {
        if self.pieces.len() > 255 || self.voices.len() > 16 || self.list.iter().any(|v| *v > 190) {
            return Err(CodecError::Index);
        }
        if self.metadata.len() > 65535 {
            return Err(CodecError::Length);
        }
        for p in self.pieces {
            ids(p.genre, p.slot, p.parameter_count)?;
            if p.voice_index != 255 && p.voice_index as usize >= self.voices.len() {
                return Err(CodecError::Index);
            }
        }
        self.encode_compatible(output)
    }
    pub fn encode_compatible(&self, output: &mut [u8]) -> Result<usize, CodecError> {
        if self.pieces.len() > 255 || self.voices.len() > 255 || self.metadata.len() > 65535 {
            return Err(CodecError::Length);
        }
        let n =
            27 + self.pieces.len() * SET_RECORD + self.voices.len() * 128 + self.metadata.len() + 4;
        if output.len() < n {
            return Err(CodecError::Capacity);
        }
        output[..5].copy_from_slice(b"P1ST\x01");
        output[5..21].copy_from_slice(self.list);
        output[21] = self.play;
        output[22] = self.mix;
        output[23] = self.pieces.len() as u8;
        output[24] = self.voices.len() as u8;
        output[25..27].copy_from_slice(&(self.metadata.len() as u16).to_le_bytes());
        let mut at = 27;
        for p in self.pieces {
            output[at..at + 3].copy_from_slice(&[p.genre, p.slot, p.voice_index]);
            output[at + 3..at + 5].copy_from_slice(&p.parameter_count.to_le_bytes());
            output[at + 5..at + 5 + RECORD_LEN].copy_from_slice(p.record);
            output[at + 5 + RECORD_LEN..at + SET_RECORD].copy_from_slice(p.progression);
            at += SET_RECORD;
        }
        for voice in self.voices {
            output[at..at + 128].copy_from_slice(voice);
            at += 128;
        }
        output[at..at + self.metadata.len()].copy_from_slice(self.metadata);
        finish(output, n);
        Ok(n)
    }
}
#[derive(Clone, Copy)]
pub struct BackupObject<'a> {
    pub id: u8,
    pub data: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct BackupFile<'a> {
    bytes: &'a [u8],
    start: usize,
    objects_end: usize,
}
pub struct BackupObjects<'a> {
    bytes: &'a [u8],
    at: usize,
    left: u8,
}
impl<'a> Iterator for BackupObjects<'a> {
    type Item = BackupObject<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        let id = self.bytes[self.at];
        let n = read_u16(self.bytes, self.at + 1).unwrap() as usize;
        self.at += 3;
        let data = &self.bytes[self.at..self.at + n];
        self.at += n;
        self.left -= 1;
        Some(BackupObject { id, data })
    }
}
impl<'a> BackupFile<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        checked(bytes, b"P1BK", 12)?;
        let start = 8 + read_u16(bytes, 6).unwrap() as usize;
        let mut at = start;
        let end = bytes.len() - 4;
        if start > end {
            return Err(CodecError::Length);
        }
        for _ in 0..bytes[5] {
            if at + 3 > end {
                return Err(CodecError::Length);
            }
            let n = read_u16(bytes, at + 1).unwrap() as usize;
            at += 3;
            if n > 3840 || at + n > end {
                return Err(CodecError::Length);
            }
            at += n;
        }
        Ok(Self {
            bytes,
            start,
            objects_end: at,
        })
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn metadata(self) -> &'a [u8] {
        &self.bytes[8..self.start]
    }
    pub fn trailing(self) -> &'a [u8] {
        &self.bytes[self.objects_end..self.bytes.len() - 4]
    }
    pub fn objects(self) -> BackupObjects<'a> {
        BackupObjects {
            bytes: self.bytes,
            at: self.start,
            left: self.bytes[5],
        }
    }
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        copy(self.bytes, output)
    }
}
pub fn backup_encode(
    objects: &[BackupObject<'_>],
    metadata: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if objects.len() > 255 || metadata.len() > 65535 {
        return Err(CodecError::Length);
    }
    let mut n = 12 + metadata.len();
    for object in objects {
        if object.data.len() > 3840 {
            return Err(CodecError::Length);
        }
        n += 3 + object.data.len();
    }
    if output.len() < n {
        return Err(CodecError::Capacity);
    }
    output[..6].copy_from_slice(&[b'P', b'1', b'B', b'K', 1, objects.len() as u8]);
    output[6..8].copy_from_slice(&(metadata.len() as u16).to_le_bytes());
    output[8..8 + metadata.len()].copy_from_slice(metadata);
    let mut at = 8 + metadata.len();
    for object in objects {
        output[at] = object.id;
        output[at + 1..at + 3].copy_from_slice(&(object.data.len() as u16).to_le_bytes());
        at += 3;
        output[at..at + object.data.len()].copy_from_slice(object.data);
        at += object.data.len();
    }
    finish(output, n);
    Ok(n)
}
