// SPDX-License-Identifier: GPL-3.0-only
//! Explicit byte formats. Runtime layout is never a serialization format.
#![no_std]
#![forbid(unsafe_code)]

pub mod jam;
pub mod session;

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
