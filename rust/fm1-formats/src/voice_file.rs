// SPDX-License-Identifier: GPL-3.0-only
//! Legacy editor file import: raw packed banks or concatenated Yamaha messages.
//! The editor historically ignored file checksums; strict MIDI messages use
//! voice::DxMessage instead. This explicit profile retains old import behavior.
use crate::{voice, CodecError};
#[derive(Clone, Copy)]
pub struct FileVoice<'a> {
    pub packed: bool,
    pub bytes: &'a [u8],
    pub name: &'a [u8],
}
pub struct FileVoices<'a> {
    bytes: &'a [u8],
    at: usize,
    bank: usize,
    remaining: usize,
}
impl<'a> FileVoices<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: if bytes.len() == 4096 { 4096 } else { 0 },
            bank: 0,
            remaining: if bytes.len() == 4096 { 32 } else { 0 },
        }
    }
}
impl<'a> Iterator for FileVoices<'a> {
    type Item = FileVoice<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining > 0 {
                let at = self.bank;
                self.bank += 128;
                self.remaining -= 1;
                return Some(FileVoice {
                    packed: true,
                    bytes: &self.bytes[at..at + 128],
                    name: &self.bytes[at + 118..at + 128],
                });
            }
            if self.at >= self.bytes.len() {
                return None;
            }
            if self.bytes[self.at] != 240 {
                self.at += 1;
                continue;
            }
            let start = self.at;
            let end = self.bytes[start + 1..]
                .iter()
                .position(|v| *v == 247)
                .map_or(self.bytes.len(), |i| start + 1 + i);
            self.at = end + 1;
            let m = &self.bytes[start + 1..end];
            if m.len() == 4102 && m[0] == 0x43 && m[2..5] == [9, 32, 0] {
                self.bank = start + 6;
                self.remaining = 32;
            } else if m.len() == 161 && m[0] == 0x43 && m[2..5] == [0, 1, 27] {
                return Some(FileVoice {
                    packed: false,
                    bytes: &self.bytes[start..self.at.min(self.bytes.len())],
                    name: &m[150..160],
                });
            }
        }
    }
}
pub fn decode_compatible(bytes: &[u8], out: &mut [u8]) -> Result<usize, CodecError> {
    let mut written = 0;
    let mut add = |packed: &[u8]| -> Result<(), CodecError> {
        let end = written + 155;
        let target = out.get_mut(written..end).ok_or(CodecError::Capacity)?;
        let mut value = voice::unpack(packed.try_into().map_err(|_| CodecError::Length)?);
        voice::sanitize(&mut value);
        target.copy_from_slice(&value);
        written = end;
        Ok(())
    };
    if bytes.len() == 4096 {
        for packed in bytes.chunks_exact(128) {
            add(packed)?;
        }
        return Ok(written);
    }
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != 240 {
            at += 1;
            continue;
        }
        let start = at + 1;
        let end = bytes[start..]
            .iter()
            .position(|v| *v == 247)
            .map_or(bytes.len(), |i| start + i);
        let m = &bytes[start..end];
        at = end + 1;
        if m.len() == 4102 && m[0] == 0x43 && m[2..5] == [9, 32, 0] {
            for packed in m[5..4101].chunks_exact(128) {
                add(packed)?;
            }
        } else if m.len() == 161 && m[0] == 0x43 && m[2..5] == [0, 1, 27] {
            let mut value: [u8; 155] = m[5..160].try_into().unwrap();
            voice::sanitize(&mut value);
            add(&voice::pack(&value))?;
        }
    }
    Ok(written)
}
