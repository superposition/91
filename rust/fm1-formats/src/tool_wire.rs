// SPDX-License-Identifier: GPL-3.0-only
//! Retained command-line codec profiles. No USB/MIDI I/O or recovery decisions.
use crate::{device_wire, package, read_u16, read_u32, wire, CodecError};
pub fn telemetry_request(command: u8, out: &mut [u8]) -> Result<usize, CodecError> {
    if command > 2 {
        return Err(CodecError::Range);
    }
    if out.len() < 6 {
        return Err(CodecError::Capacity);
    }
    out[..6].copy_from_slice(&[240, 125, 80, 70, command, 247]);
    Ok(6)
}
pub fn crc_cipher(bytes: &mut [u8]) {
    const MAGIC: [u8; 16] = [
        0xc3, 0xcf, 0xc0, 0xe8, 0xce, 0xd2, 0xb0, 0xae, 0xc4, 0xe3, 0xa3, 0xac, 0xd3, 0xf1, 0xc1,
        0xd6,
    ];
    let mut crc = package::crc16(&[255, 255], 65535);
    for (i, b) in bytes.iter_mut().enumerate() {
        crc = package::crc16(&[MAGIC[i % 16]], crc);
        *b ^= crc as u8;
    }
}
pub fn word35(value: u64) -> [u8; 5] {
    core::array::from_fn(|i| ((value >> (i * 7)) & 127) as u8)
}
pub fn read_word35(bytes: &[u8]) -> Result<u32, CodecError> {
    if bytes.len() != 5 {
        return Err(CodecError::Length);
    }
    if bytes.iter().any(|b| *b > 127) {
        return Err(CodecError::Range);
    }
    Ok(bytes.iter().enumerate().fold(0, |v, (i, b)| {
        v | u32::from(*b).wrapping_shl((i * 7) as u32)
    }))
}
pub fn telemetry_compatible(
    bytes: &[u8],
    kind: u8,
    max_words: usize,
    out: &mut [u8],
) -> Result<usize, CodecError> {
    if bytes.len() < 3 || bytes[..3] != [125, 80, kind] {
        return Err(CodecError::Magic);
    }
    let count = ((bytes.len() - 3) * 7 / 8 / 4).min(max_words);
    let n = count.checked_mul(4).ok_or(CodecError::Length)?;
    if out.len() < n {
        return Err(CodecError::Capacity);
    }
    device_wire::unpack7_prefix(&bytes[3..], &mut out[..n])?;
    Ok(n)
}
pub fn jam_begin(length: u16, crc: u32, out: &mut [u8]) -> Result<usize, CodecError> {
    if length > 16383 {
        return Err(CodecError::Range);
    }
    if out.len() < 10 {
        return Err(CodecError::Capacity);
    }
    out[0] = (length >> 7) as u8;
    out[1] = (length & 127) as u8;
    for i in 0..8 {
        out[i + 2] = ((crc >> (28 - i * 4)) & 15) as u8;
    }
    Ok(10)
}
pub fn jam_chunk(index: u8, bytes: &[u8], out: &mut [u8]) -> Result<usize, CodecError> {
    if index > 127 || bytes.len() > 256 {
        return Err(CodecError::Range);
    }
    let n = 3 + bytes.len() * 2;
    if out.len() < n {
        return Err(CodecError::Capacity);
    }
    out[0] = index;
    out[1] = (bytes.len() >> 7) as u8;
    out[2] = (bytes.len() & 127) as u8;
    wire::encode_nibbles(bytes, &mut out[3..n])?;
    Ok(n)
}
pub fn jam_control(field: u8, value: u16, out: &mut [u8]) -> Result<usize, CodecError> {
    if field > 127 || value > 16383 {
        return Err(CodecError::Range);
    }
    if out.len() < 3 {
        return Err(CodecError::Capacity);
    }
    out[..3].copy_from_slice(&[field, (value >> 7) as u8, (value & 127) as u8]);
    Ok(3)
}
/// The original rescue reader validates its selected flash data checksum, not
/// unrelated package header/directory checksums. Installation policy stays out.
pub fn rescue_flash_compatible(bytes: &[u8], out: &mut [u8]) -> Result<usize, CodecError> {
    let package = package::Fwsc::parse(bytes)?;
    let mut header = [0; 64];
    package.copy_logical(0, &mut header)?;
    package::enc(&mut header, 65535);
    let count = read_u16(&header, 8).unwrap() as usize;
    if header[16..22] != *b"AC791N" || count == 0 || count >= 16 {
        return Err(CodecError::Magic);
    }
    for i in 0..count {
        let mut entry = [0; 80];
        package.copy_logical(64 + i * 80, &mut entry)?;
        package::enc(&mut entry, 65535);
        if read_u16(&entry, 0) != Some(0) {
            continue;
        }
        let offset = read_u32(&entry, 8).unwrap() as usize;
        let n = read_u32(&entry, 12).unwrap() as usize;
        if offset
            .checked_add(n)
            .is_none_or(|end| end > package.logical_len())
        {
            // The original rescue reader reports truncated selected data as a
            // bad flash-entry checksum, before any recovery action is possible.
            return Err(CodecError::Checksum);
        }
        if out.len() < n {
            return Err(CodecError::Capacity);
        }
        package.copy_logical(offset, &mut out[..n])?;
        if package::crc16(&out[..n], 0) != read_u16(&entry, 4).unwrap() {
            return Err(CodecError::Checksum);
        }
        return Ok(n);
    }
    Err(CodecError::Index)
}
