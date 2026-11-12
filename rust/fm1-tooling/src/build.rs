// SPDX-License-Identifier: GPL-3.0-only
//! Loader packaging policy; OS and compiler process delivery stay in adapters.
use crate::{bytes, Error, Result};
use fm1_formats::package::{crc16, JlfsParts};
use serde_json::{json, Value};
fn block_bytes(value: &Value) -> Result<Vec<u8>> {
    let array = value
        .as_array()
        .ok_or_else(|| Error::value("expected byte array"))?;
    if array.len() > 1_110_000 {
        return Err(Error::value("LZ4 block exceeds 1110000 bytes"));
    }
    array
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| Error::value("expected byte 0..255"))
        })
        .collect()
}
pub fn loader_wrap(image: &[u8]) -> Result<Vec<u8>> {
    let mut blocks = Vec::new();
    for (i, raw) in image.chunks(4096).enumerate() {
        let compressed = crate::lz4::compress(raw)?;
        if compressed.len() > 4096 || crate::lz4::decompress(&compressed, &[])? != raw {
            return Err(Error {
                kind: "SystemExit".into(),
                code: String::new(),
                message: format!("loader: block {i} does not compress below 4096 B"),
            });
        }
        blocks.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        blocks.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        blocks.extend(compressed);
    }
    let name = *b"usb_hid_ota.bin\0";
    let header = |checksum, offset, length, flags| {
        JlfsParts {
            data_checksum: checksum,
            offset,
            length,
            flags,
            reserved: 0,
            index: 0,
            raw_name: &name,
        }
        .encode()
    };
    let mut body = header(crc16(image, 0), image.len() as u32, 0x01c0a800, 0).to_vec();
    body.extend(blocks);
    let mut out = header(crc16(&body, 0), 0x20, body.len() as u32, 0x41).to_vec();
    out.extend(body);
    Ok(out)
}
pub fn request(method: &str, v: &Value) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "lz4.compress" => json!(crate::lz4::compress(&bytes(&a[0])?)?),
        "lz4.decompress" => json!(crate::lz4::decompress(
            &block_bytes(&a[0])?,
            &bytes(&a[1])?
        )?),
        "lz4.length" => json!(crate::lz4::length(
            a[0].as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| Error::value("expected nonnegative length"))?
        )?),
        "loader-wrap" => json!(loader_wrap(&bytes(&a[0])?)?),
        _ => return Err(Error::value("unknown build method")),
    })
}
