// SPDX-License-Identifier: GPL-3.0-only
//! Explicit byte formats. Runtime layout is never a serialization format.
#![no_std]
#![forbid(unsafe_code)]

pub mod containers;
pub mod device_wire;
pub mod jam;
pub mod objects;
pub mod package;
pub mod session;
pub mod storage;
pub mod voice;
pub mod voice_file;
pub mod wire;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecError {
    Length,
    Magic,
    Version,
    Index,
    Range,
    Checksum,
    Nibble,
    Capacity,
    State,
}

pub fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320u32 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

pub fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let a = *bytes.get(offset)?;
    let b = *bytes.get(offset.checked_add(1)?)?;
    Some(u16::from_le_bytes([a, b]))
}
