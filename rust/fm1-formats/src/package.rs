// SPDX-License-Identifier: GPL-3.0-only
// Derived from the FM-1 package/OTA primitives by Leo Kuroshita and contributors.
//! Package byte primitives only: no flash access, recovery-stub changes or SDK.
use crate::{crc32, read_u16, read_u32, CodecError};
pub fn crc16(bytes: &[u8], mut crc: u16) -> u16 {
    for &b in bytes {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                crc.wrapping_shl(1) ^ 0x1021
            } else {
                crc.wrapping_shl(1)
            };
        }
    }
    crc
}
pub fn enc(bytes: &mut [u8], mut key: u16) {
    for b in bytes {
        *b ^= key as u8;
        key = if key & 0x8000 != 0 {
            key.wrapping_shl(1) ^ 0x1021
        } else {
            key.wrapping_shl(1)
        };
    }
}
pub fn sfc(bytes: &mut [u8], offset: u32, base: u32, key: u16) -> Result<(), CodecError> {
    let end = offset
        .checked_add(u32::try_from(bytes.len()).map_err(|_| CodecError::Length)?)
        .ok_or(CodecError::Range)?;
    if offset < base || end < base {
        return Err(CodecError::Range);
    }
    for (i, block) in bytes.chunks_mut(32).enumerate() {
        enc(
            block,
            key ^ (((offset - base) / 4).wrapping_add((i * 8) as u32) as u16),
        );
    }
    Ok(())
}
pub fn chip_key(bytes: &[u8]) -> Result<u16, CodecError> {
    if bytes.len() != 32 {
        return Err(CodecError::Length);
    }
    let sum = bytes[..16].iter().fold(0u8, |s, v| s.wrapping_add(*v));
    let threshold = if sum >= 0xe0 {
        0xaa
    } else if sum <= 0x10 {
        0x55
    } else {
        sum
    };
    let mut key = 0;
    for i in 0..16 {
        if bytes[16 + i] ^ bytes[15 - i] < threshold {
            key |= 1 << i;
        }
    }
    Ok(key)
}
pub fn key_blob(key: u16) -> [u8; 34] {
    let mut b = [0x10; 34];
    for i in 0..16 {
        b[16 + i] = b[15 - i] ^ if key >> i & 1 != 0 { 0 } else { 255 };
    }
    let c = crc16(&b[..32], 0);
    b[32..34].copy_from_slice(&c.to_le_bytes());
    b
}
#[derive(Clone, Copy)]
pub struct Fwsc<'a> {
    bytes: &'a [u8],
}
impl<'a> Fwsc<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 960 {
            return Err(CodecError::Length);
        }
        Ok(Self { bytes })
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn logical_len(self) -> usize {
        self.bytes.len() - 20
    }
    pub fn product(self, output: &mut [u8]) -> Result<usize, CodecError> {
        let n = (0usize..20)
            .filter(|i| self.bytes[i * 48 + 47] != 125)
            .count();
        if output.len() < n {
            return Err(CodecError::Capacity);
        }
        let mut at = 0;
        for i in 0..20 {
            let m = self.bytes[i * 48 + 47];
            if m != 125 {
                output[at] = m.wrapping_sub(i as u8 + 1);
                at += 1;
            }
        }
        Ok(n)
    }
    pub fn logical_byte(self, index: usize) -> Option<u8> {
        if index >= self.logical_len() {
            None
        } else {
            self.bytes
                .get(index + if index < 940 { index / 47 } else { 20 })
                .copied()
        }
    }
    pub fn copy_logical(self, offset: usize, output: &mut [u8]) -> Result<(), CodecError> {
        let end = offset.checked_add(output.len()).ok_or(CodecError::Length)?;
        if end > self.logical_len() {
            return Err(CodecError::Length);
        }
        for (i, b) in output.iter_mut().enumerate() {
            *b = self.logical_byte(offset + i).unwrap();
        }
        Ok(())
    }
    pub fn directory(self) -> Result<Directory<'a>, CodecError> {
        let mut header = [0; 64];
        self.copy_logical(0, &mut header)?;
        enc(&mut header, 0xffff);
        if crc16(&header[2..], 0) != read_u16(&header, 0).unwrap() {
            return Err(CodecError::Checksum);
        }
        if &header[16..22] != b"AC791N" {
            return Err(CodecError::Magic);
        }
        let total = read_u32(&header, 4).unwrap() as usize;
        let count = read_u16(&header, 8).unwrap() as usize;
        if total > self.logical_len() || total < 64 + count * 80 {
            return Err(CodecError::Length);
        }
        let mut crc = 0;
        for i in 64..64 + count * 80 {
            crc = crc16(&[self.logical_byte(i).unwrap()], crc);
        }
        if crc != read_u16(&header, 2).unwrap() {
            return Err(CodecError::Checksum);
        }
        let directory = Directory {
            package: self,
            header,
            count,
            total,
        };
        for i in 0..count {
            directory.entry(i)?;
        }
        Ok(directory)
    }
}
pub fn wrap_logical(
    logical: &[u8],
    product: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if logical.len() < 940 || product.len() > 20 {
        return Err(CodecError::Length);
    }
    if product
        .iter()
        .enumerate()
        .any(|(i, v)| v.wrapping_add(i as u8 + 1) == 125)
    {
        return Err(CodecError::Range);
    }
    let n = logical.len().checked_add(20).ok_or(CodecError::Length)?;
    if output.len() < n {
        return Err(CodecError::Capacity);
    }
    for i in 0..20 {
        output[i * 48..i * 48 + 47].copy_from_slice(&logical[i * 47..i * 47 + 47]);
        output[i * 48 + 47] = product.get(i).map_or(125, |v| v.wrapping_add(i as u8 + 1));
    }
    output[960..n].copy_from_slice(&logical[940..]);
    Ok(n)
}
pub struct Directory<'a> {
    package: Fwsc<'a>,
    header: [u8; 64],
    count: usize,
    total: usize,
}
#[derive(Clone, Copy)]
pub struct Entry {
    bytes: [u8; 80],
}
impl Entry {
    pub fn bytes(&self) -> &[u8; 80] {
        &self.bytes
    }
    pub fn kind(&self) -> u16 {
        read_u16(&self.bytes, 0).unwrap()
    }
    pub fn index(&self) -> u16 {
        read_u16(&self.bytes, 2).unwrap()
    }
    pub fn checksum(&self) -> u16 {
        read_u16(&self.bytes, 4).unwrap()
    }
    pub fn offset(&self) -> usize {
        read_u32(&self.bytes, 8).unwrap() as usize
    }
    pub fn len(&self) -> usize {
        read_u32(&self.bytes, 12).unwrap() as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn name(&self) -> &[u8] {
        let b = &self.bytes[64..];
        &b[..b.iter().position(|v| *v == 0).unwrap_or(16)]
    }
}
impl Directory<'_> {
    pub fn header(&self) -> &[u8; 64] {
        &self.header
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn total(&self) -> usize {
        self.total
    }
    pub fn entry(&self, index: usize) -> Result<Entry, CodecError> {
        if index >= self.count {
            return Err(CodecError::Index);
        }
        let mut bytes = [0; 80];
        self.package.copy_logical(64 + index * 80, &mut bytes)?;
        enc(&mut bytes, 0xffff);
        let e = Entry { bytes };
        if e.offset().checked_add(e.len()).ok_or(CodecError::Length)? > self.total {
            return Err(CodecError::Length);
        }
        Ok(e)
    }
    pub fn validate_data(&self, index: usize) -> Result<(), CodecError> {
        let e = self.entry(index)?;
        let mut crc = 0;
        for i in e.offset()..e.offset() + e.len() {
            crc = crc16(&[self.package.logical_byte(i).unwrap()], crc);
        }
        if crc != e.checksum() {
            Err(CodecError::Checksum)
        } else {
            Ok(())
        }
    }
}
/// The unchanged recovery-stub body commit marker. Compensation/reserved bytes
/// remain the package builder's responsibility, never generated by a parser.
pub fn body_marker(body: &[u8]) -> Result<[u8; 20], CodecError> {
    let size = u32::try_from(body.len()).map_err(|_| CodecError::Length)?;
    let mut m = [0; 20];
    m[..4].copy_from_slice(b"CRBM");
    m[4..8].copy_from_slice(&1u32.to_le_bytes());
    m[8..12].copy_from_slice(&size.to_le_bytes());
    m[12..16].copy_from_slice(&crc32(body).to_le_bytes());
    let crc = crc32(&m[..16]);
    m[16..20].copy_from_slice(&crc.to_le_bytes());
    Ok(m)
}
pub fn validate_marker(marker: &[u8], body: &[u8]) -> Result<(), CodecError> {
    if marker.len() < 20 {
        return Err(CodecError::Length);
    }
    if &marker[..4] != b"CRBM" {
        return Err(CodecError::Magic);
    }
    if read_u32(marker, 4) != Some(1) {
        return Err(CodecError::Version);
    }
    if read_u32(marker, 8) != u32::try_from(body.len()).ok() {
        return Err(CodecError::Length);
    }
    if marker[..20] != body_marker(body)? {
        return Err(CodecError::Checksum);
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct JlfsEntry<'a>(&'a [u8; 32]);
impl<'a> JlfsEntry<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        let bytes: &[u8; 32] = bytes.try_into().map_err(|_| CodecError::Length)?;
        if crc16(&bytes[2..], 0) != read_u16(bytes, 0).unwrap() {
            return Err(CodecError::Checksum);
        }
        Ok(Self(bytes))
    }
    pub fn bytes(self) -> &'a [u8; 32] {
        self.0
    }
    pub fn data_checksum(self) -> u16 {
        read_u16(self.0, 2).unwrap()
    }
    pub fn offset(self) -> u32 {
        read_u32(self.0, 4).unwrap()
    }
    pub fn len(self) -> u32 {
        read_u32(self.0, 8).unwrap()
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn flags(self) -> u8 {
        self.0[12]
    }
    pub fn reserved(self) -> u8 {
        self.0[13]
    }
    pub fn index(self) -> u16 {
        read_u16(self.0, 14).unwrap()
    }
    pub fn raw_name(self) -> &'a [u8; 16] {
        self.0[16..].try_into().unwrap()
    }
}
pub struct JlfsParts<'a> {
    pub data_checksum: u16,
    pub offset: u32,
    pub length: u32,
    pub flags: u8,
    pub reserved: u8,
    pub index: u16,
    pub raw_name: &'a [u8; 16],
}
impl JlfsParts<'_> {
    pub fn encode(&self) -> [u8; 32] {
        let mut b = [0; 32];
        b[2..4].copy_from_slice(&self.data_checksum.to_le_bytes());
        b[4..8].copy_from_slice(&self.offset.to_le_bytes());
        b[8..12].copy_from_slice(&self.length.to_le_bytes());
        b[12] = self.flags;
        b[13] = self.reserved;
        b[14..16].copy_from_slice(&self.index.to_le_bytes());
        b[16..32].copy_from_slice(self.raw_name);
        let crc = crc16(&b[2..], 0);
        b[..2].copy_from_slice(&crc.to_le_bytes());
        b
    }
}
/// Pure compensation-byte calculation used by the existing package layout.
/// The four bytes at offset must be zero; caller chooses whether to write them.
pub fn compensate(app: &[u8], offset: usize, want: u16) -> Result<[u8; 4], CodecError> {
    let end = offset.checked_add(4).ok_or(CodecError::Length)?;
    let field = app.get(offset..end).ok_or(CodecError::Length)?;
    if field.iter().any(|v| *v != 0) {
        return Err(CodecError::Range);
    }
    let tail = app.len() - end;
    let mut need = crc16(app, 0) ^ want;
    let mut rows = [(0u16, 0u32); 16];
    let mut count = 0;
    for bit in 0..32 {
        let mut mask = 1u32 << bit;
        let mut value = crc16(&mask.to_le_bytes(), 0);
        for _ in 0..tail {
            value = crc16(&[0], value);
        }
        for &(pivot, pmask) in &rows[..count] {
            if value ^ pivot < value {
                value ^= pivot;
                mask ^= pmask;
            }
        }
        if value != 0 {
            if count == 16 {
                return Err(CodecError::State);
            }
            let mut index = count;
            while index > 0 && rows[index - 1].0 < value {
                rows[index] = rows[index - 1];
                index -= 1;
            }
            rows[index] = (value, mask);
            count += 1;
        }
    }
    let mut output = 0u32;
    for &(pivot, mask) in &rows[..count] {
        if need ^ pivot < need {
            need ^= pivot;
            output ^= mask;
        }
    }
    if need != 0 {
        return Err(CodecError::State);
    }
    Ok(output.to_le_bytes())
}
