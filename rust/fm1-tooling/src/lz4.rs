// SPDX-License-Identifier: GPL-3.0-only
// Derived from the loader block codec by Leo Kuroshita (@kurogedelic),
// Copyright (C) 2026 Hügelton Instruments.
//! Byte-compatible greedy block codec used by the existing loader packager.
use crate::{Error, Result};
use std::collections::BTreeMap;
const LIMIT: usize = 1_100_000;
fn index() -> Error {
    Error {
        kind: "IndexError".into(),
        code: String::new(),
        message: "index out of range".into(),
    }
}
fn reserve(n: usize) -> Result<()> {
    if n > LIMIT {
        Err(Error::value("LZ4 output exceeds 1100000 bytes"))
    } else {
        Ok(())
    }
}
pub fn length(mut n: usize) -> Result<Vec<u8>> {
    reserve(n / 255 + 1)?;
    let mut out = Vec::new();
    while n >= 255 {
        out.push(255);
        n -= 255;
    }
    out.push(n as u8);
    Ok(out)
}
pub fn compress(source: &[u8]) -> Result<Vec<u8>> {
    reserve(source.len())?;
    let (mut anchor, mut i) = (0, 0);
    let mut out = Vec::new();
    let mut table = BTreeMap::<[u8; 4], usize>::new();
    while i < source.len().saturating_sub(12) {
        let key = source[i..i + 4].try_into().unwrap();
        let candidate = table.insert(key, i);
        let Some(candidate) = candidate.filter(|&p| i - p <= 65535) else {
            i += 1;
            continue;
        };
        let mut count = 4;
        while i + count < source.len() - 5 && source[candidate + count] == source[i + count] {
            count += 1;
        }
        let literals = &source[anchor..i];
        out.push(((literals.len().min(15) << 4) | (count - 4).min(15)) as u8);
        if literals.len() >= 15 {
            out.extend(length(literals.len() - 15)?);
        }
        out.extend_from_slice(literals);
        out.extend_from_slice(&((i - candidate) as u16).to_le_bytes());
        if count - 4 >= 15 {
            out.extend(length(count - 4 - 15)?);
        }
        i += count;
        anchor = i;
    }
    let literals = &source[anchor..];
    out.push((literals.len().min(15) << 4) as u8);
    if literals.len() >= 15 {
        out.extend(length(literals.len() - 15)?);
    }
    out.extend_from_slice(literals);
    Ok(out)
}
fn extended(block: &[u8], at: &mut usize, mut count: usize) -> Result<usize> {
    if count == 15 {
        loop {
            let b = *block.get(*at).ok_or_else(index)?;
            *at += 1;
            count += b as usize;
            if b != 255 {
                break;
            }
        }
    }
    Ok(count)
}
pub fn decompress(block: &[u8], prefix: &[u8]) -> Result<Vec<u8>> {
    reserve(prefix.len())?;
    let start = prefix.len();
    let mut out = prefix.to_vec();
    let mut at = 0;
    while at < block.len() {
        let token = block[at];
        at += 1;
        let literals = extended(block, &mut at, (token >> 4) as usize)?;
        let take = literals.min(block.len() - at);
        reserve(out.len() + take)?;
        out.extend_from_slice(&block[at..at + take]);
        at += literals;
        // Preserve the old decoder's accepted truncated final literal run.
        if at >= block.len() {
            break;
        }
        let low = *block.get(at).ok_or_else(index)?;
        let high = *block.get(at + 1).ok_or_else(index)?;
        at += 2;
        let offset = u16::from_le_bytes([low, high]) as usize;
        let count = extended(block, &mut at, (token & 15) as usize)? + 4;
        reserve(out.len() + count)?;
        for _ in 0..count {
            // Python out[-0] selects its first byte; retain that legacy case.
            let source = if offset == 0 {
                0
            } else {
                out.len().checked_sub(offset).ok_or_else(index)?
            };
            out.push(*out.get(source).ok_or_else(index)?);
        }
    }
    Ok(out.split_off(start))
}
