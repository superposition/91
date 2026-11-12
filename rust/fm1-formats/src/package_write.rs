// SPDX-License-Identifier: GPL-3.0-only
//! Explicit UFW/FWSC writers; SDK files and recovery/install policy stay outside.
use crate::{
    package::{crc16, enc},
    CodecError,
};
fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
pub struct HeaderParts<'a> {
    pub total: u32,
    pub entries: u16,
    pub flags: u16,
    pub version: u16,
    pub reserved: u16,
    pub chip: &'a [u8; 6],
    pub directory_checksum: u16,
}
impl HeaderParts<'_> {
    pub fn encode(&self) -> [u8; 64] {
        self.encode_preserving(&[0; 64])
    }
    pub fn encode_preserving(&self, base: &[u8; 64]) -> [u8; 64] {
        let mut b = *base;
        put16(&mut b, 2, self.directory_checksum);
        put32(&mut b, 4, self.total);
        put16(&mut b, 8, self.entries);
        put16(&mut b, 10, self.flags);
        put16(&mut b, 12, self.version);
        put16(&mut b, 14, self.reserved);
        b[16..22].copy_from_slice(self.chip);
        let crc = crc16(&b[2..], 0);
        put16(&mut b, 0, crc);
        b
    }
}
pub struct EntryParts<'a> {
    pub kind: u16,
    pub index: u16,
    pub checksum: u16,
    pub reserved: u16,
    pub offset: u32,
    pub length: u32,
    pub padded_length: u32,
    pub extra: &'a [u8; 44],
    pub name: &'a [u8; 16],
}
impl EntryParts<'_> {
    pub fn encode(&self) -> [u8; 80] {
        let mut b = [0; 80];
        put16(&mut b, 0, self.kind);
        put16(&mut b, 2, self.index);
        put16(&mut b, 4, self.checksum);
        put16(&mut b, 6, self.reserved);
        put32(&mut b, 8, self.offset);
        put32(&mut b, 12, self.length);
        put32(&mut b, 16, self.padded_length);
        b[20..64].copy_from_slice(self.extra);
        b[64..].copy_from_slice(self.name);
        b
    }
}
pub struct UfwParts<'a> {
    pub flash: &'a [u8],
    pub ota: &'a [u8],
    pub product: &'a [u8],
}
impl UfwParts<'_> {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, CodecError> {
        if self.product.len() > 20
            || self
                .product
                .iter()
                .enumerate()
                .any(|(i, b)| b.wrapping_add(i as u8 + 1) == 125)
        {
            return Err(CodecError::Range);
        }
        let offset0 = 1024usize;
        let offset1 = offset0
            .checked_add(self.flash.len())
            .and_then(|n| n.checked_add(255))
            .ok_or(CodecError::Length)?
            & !255;
        let total = offset1
            .checked_add(self.ota.len())
            .ok_or(CodecError::Length)?;
        let n = total.checked_add(20).ok_or(CodecError::Length)?;
        let total32 = u32::try_from(total).map_err(|_| CodecError::Length)?;
        if out.len() < n {
            return Err(CodecError::Capacity);
        }
        let mut directory = [0; 160];
        for (i, (data, offset, name)) in [
            (self.flash, offset0, b"flash.bin\0\0\0\0\0\0\0"),
            (self.ota, offset1, b"ota.bin\0\0\0\0\0\0\0\0\0"),
        ]
        .iter()
        .enumerate()
        {
            let length = u32::try_from(data.len()).map_err(|_| CodecError::Length)?;
            let padded = length.checked_add(31).ok_or(CodecError::Length)? & !31;
            let mut entry = EntryParts {
                kind: if i == 0 { 0 } else { 100 },
                index: i as u16,
                checksum: crc16(data, 0),
                reserved: 0,
                offset: *offset as u32,
                length,
                padded_length: padded,
                extra: &[0; 44],
                name,
            }
            .encode();
            enc(&mut entry, 65535);
            directory[i * 80..(i + 1) * 80].copy_from_slice(&entry);
        }
        let mut header = HeaderParts {
            total: total32,
            entries: 2,
            flags: 4,
            version: 512,
            reserved: 0,
            chip: b"AC791N",
            directory_checksum: crc16(&directory, 0),
        }
        .encode();
        enc(&mut header, 65535);
        // Build the logical image twenty bytes ahead of its final location. Forward
        // movement can only overwrite bytes already read, including inserted markers.
        out[..n].fill(255);
        out[20..84].copy_from_slice(&header);
        out[84..244].copy_from_slice(&directory);
        out[20 + offset0..20 + offset0 + self.flash.len()].copy_from_slice(self.flash);
        out[20 + offset1..n].copy_from_slice(self.ota);
        for i in 0..total {
            let at = i + if i < 940 { i / 47 } else { 20 };
            out[at] = out[20 + i];
            if i < 940 && i % 47 == 46 {
                let index = i / 47;
                out[at + 1] = self
                    .product
                    .get(index)
                    .map_or(125, |v| v.wrapping_add(index as u8 + 1));
            }
        }
        Ok(n)
    }
}
