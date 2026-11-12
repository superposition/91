// SPDX-License-Identifier: GPL-3.0-only
//! Device envelopes and typed fields. Transfer ownership and application policy
//! are deliberately separate from these bounded byte operations.
use crate::{wire, CodecError};

/// The original OTA decoder reports the complete decoded size even when only
/// a prefix fits. High input bits are masked, as on the device.
pub fn unpack7_prefix(input: &[u8], output: &mut [u8]) -> Result<usize, CodecError> {
    let total = input.len().checked_mul(7).ok_or(CodecError::Length)? / 8;
    let (mut accumulator, mut bits, mut at) = (0u32, 0u32, 0usize);
    for &byte in input {
        accumulator |= u32::from(byte & 127) << bits;
        bits += 7;
        if bits >= 8 {
            if at < output.len() {
                output[at] = accumulator as u8;
            }
            at += 1;
            accumulator >>= 8;
            bits -= 8;
        }
    }
    debug_assert_eq!(at, total);
    Ok(total)
}

/// Telemetry uses one continuous LSB-first seven-bit stream of little-endian
/// words, rather than independently packing each word.
pub fn word_frame(kind: u8, words: &[u32], output: &mut [u8]) -> Result<usize, CodecError> {
    if kind > 127 {
        return Err(CodecError::Range);
    }
    let payload = words
        .len()
        .checked_mul(32)
        .and_then(|n| n.checked_add(6))
        .ok_or(CodecError::Length)?
        / 7;
    let length = payload.checked_add(5).ok_or(CodecError::Length)?;
    if output.len() < length {
        return Err(CodecError::Capacity);
    }
    output[..4].copy_from_slice(&[0xf0, 0x7d, 0x50, kind]);
    let (mut accumulator, mut bits, mut at) = (0u32, 0u32, 4usize);
    for word in words {
        for byte in word.to_le_bytes() {
            accumulator |= u32::from(byte) << bits;
            bits += 8;
            while bits >= 7 {
                output[at] = (accumulator & 127) as u8;
                at += 1;
                accumulator >>= 7;
                bits -= 7;
            }
        }
    }
    if bits != 0 {
        output[at] = accumulator as u8;
        at += 1;
    }
    output[at] = 0xf7;
    Ok(at + 1)
}

pub fn parse_word_frame(bytes: &[u8], kind: u8, output: &mut [u32]) -> Result<usize, CodecError> {
    if bytes.len() < 5 || bytes[..4] != [0xf0, 0x7d, 0x50, kind] || bytes[bytes.len() - 1] != 0xf7 {
        return Err(CodecError::Magic);
    }
    let payload = &bytes[4..bytes.len() - 1];
    if payload.iter().any(|v| *v > 127) {
        return Err(CodecError::Range);
    }
    let length = payload.len().checked_mul(7).ok_or(CodecError::Length)? / 8;
    if length % 4 != 0 || (length * 8).div_ceil(7) != payload.len() {
        return Err(CodecError::Length);
    }
    if output.len() < length / 4 {
        return Err(CodecError::Capacity);
    }
    let (mut accumulator, mut bits, mut at, mut word) = (0u32, 0u32, 0usize, 0u32);
    for &byte in payload {
        accumulator |= u32::from(byte) << bits;
        bits += 7;
        if bits >= 8 {
            word |= u32::from(accumulator as u8) << ((at % 4) * 8);
            at += 1;
            if at.is_multiple_of(4) {
                output[at / 4 - 1] = word;
                word = 0;
            }
            accumulator >>= 8;
            bits -= 8;
        }
    }
    Ok(length / 4)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JamBegin {
    pub length: u16,
    pub crc: u32,
}
impl JamBegin {
    pub fn parse(bytes: &[u8]) -> Result<Self, CodecError> {
        if bytes.len() != 10 {
            return Err(CodecError::Length);
        }
        let length = value14(&bytes[..2])?;
        let mut crc = 0u32;
        for &nibble in &bytes[2..] {
            if nibble > 15 {
                return Err(CodecError::Nibble);
            }
            crc = (crc << 4) | u32::from(nibble);
        }
        Ok(Self { length, crc })
    }
}

pub fn value14(bytes: &[u8]) -> Result<u16, CodecError> {
    if bytes.len() != 2 {
        return Err(CodecError::Length);
    }
    if bytes.iter().any(|v| *v > 127) {
        return Err(CodecError::Range);
    }
    Ok((u16::from(bytes[0]) << 7) | u16::from(bytes[1]))
}

#[derive(Clone, Copy)]
pub struct JamChunk<'a> {
    pub index: u8,
    pub length: u16,
    nibbles: &'a [u8],
}
impl<'a> JamChunk<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < 3 {
            return Err(CodecError::Length);
        }
        let length = value14(&bytes[1..3])?;
        if bytes[0] > 127 || bytes.len() != 3 + usize::from(length) * 2 {
            return Err(CodecError::Length);
        }
        if bytes[3..].iter().any(|v| *v > 15) {
            return Err(CodecError::Nibble);
        }
        Ok(Self {
            index: bytes[0],
            length,
            nibbles: &bytes[3..],
        })
    }
    pub fn decode(self, output: &mut [u8]) -> Result<usize, CodecError> {
        wire::decode_nibbles(self.nibbles, output)
    }
    pub fn matches(self, bytes: &[u8]) -> bool {
        bytes.len() == usize::from(self.length)
            && bytes
                .iter()
                .zip(self.nibbles.chunks_exact(2))
                .all(|(&byte, pair)| byte == (pair[0] << 4 | pair[1]))
    }
}

/// Preserve the firmware's checksum/header error before the seven-bit error.
/// Return 0 for valid, 1 for a malformed data byte, 2 for envelope/checksum.
pub fn jam_preflight(bytes: &[u8]) -> u32 {
    if bytes.len() < 6
        || bytes[..3] != [0x7d, 0x50, 0x60]
        || wire::sum7(&bytes[3..bytes.len() - 1]) != bytes[bytes.len() - 1]
    {
        2
    } else if bytes.iter().any(|v| *v > 127) {
        1
    } else {
        0
    }
}

pub fn voice_ack(kind: u8, count: u8, output: &mut [u8]) -> Result<usize, CodecError> {
    if output.len() < 7 {
        return Err(CodecError::Capacity);
    }
    output[..7].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x57, kind, count, 0xf7]);
    Ok(7)
}

/// Firmware error replies echo even malformed high-bit op/token bytes. The
/// normal strict Jam encoder remains available in `wire::jam_reply`.
pub fn jam_reply_legacy(
    op: u8,
    token: u8,
    status: u8,
    raw: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    if raw.len() > 68 {
        return Err(CodecError::Range);
    }
    let length = 9 + raw.len() * 2;
    if output.len() < length {
        return Err(CodecError::Capacity);
    }
    output[..7].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x61, op, token, status]);
    wire::encode_nibbles(raw, &mut output[7..length - 2])?;
    output[length - 2] = wire::sum7(&output[4..length - 2]);
    output[length - 1] = 0xf7;
    Ok(length)
}

fn scalar(output: &mut [u8], values: &[u32]) -> Result<usize, CodecError> {
    if output.len() < values.len() * 4 {
        return Err(CodecError::Capacity);
    }
    for (bytes, value) in output.chunks_exact_mut(4).zip(values) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }
    Ok(values.len() * 4)
}

/// Shared synchronous codec dispatcher for the browser sidecar and native ABI.
/// Scalars use little-endian u32. Encoded packets retain their exact wire bytes.
pub fn dispatch(
    operation: u32,
    args: &[u32],
    data: &[u8],
    output: &mut [u8],
) -> Result<usize, CodecError> {
    let arg = |i: usize| args.get(i).copied().ok_or(CodecError::Length);
    let byte = |i: usize| u8::try_from(arg(i)?).map_err(|_| CodecError::Range);
    match operation {
        70 => scalar(
            output,
            &[u32::from(crate::package::crc16(data, arg(0)? as u16))],
        ),
        71 => {
            if output.len() < data.len() {
                return Err(CodecError::Capacity);
            }
            output[..data.len()].copy_from_slice(data);
            crate::package::enc(&mut output[..data.len()], arg(0)? as u16);
            Ok(data.len())
        }
        72 => wire::pack7(data, output),
        73 => {
            let total = data.len().checked_mul(7).ok_or(CodecError::Length)? / 8;
            let prefix = (arg(0)? as usize).min(total);
            if output.len() < 4 + prefix {
                return Err(CodecError::Capacity);
            }
            unpack7_prefix(data, &mut output[4..4 + prefix])?;
            scalar(output, &[total as u32])?;
            Ok(4 + prefix)
        }
        74 => wire::encode_ota(byte(0)?, data, output),
        75 => {
            let message = wire::OtaMessage::parse_decoded(data)?;
            let body = message.body();
            if output.len() < 8 + body.len() {
                return Err(CodecError::Capacity);
            }
            scalar(output, &[u32::from(message.command()), body.len() as u32])?;
            output[8..8 + body.len()].copy_from_slice(body);
            Ok(8 + body.len())
        }
        76 => {
            if !data.len().is_multiple_of(4) || data.len() > 160 {
                return Err(CodecError::Length);
            }
            let mut words = [0u32; 40];
            for (word, bytes) in words.iter_mut().zip(data.chunks_exact(4)) {
                *word = u32::from_le_bytes(bytes.try_into().unwrap());
            }
            word_frame(byte(0)?, &words[..data.len() / 4], output)
        }
        77 => {
            let mut words = [0u32; 40];
            let count = parse_word_frame(data, byte(0)?, &mut words)?;
            scalar(output, &words[..count])
        }
        78 => scalar(output, &[jam_preflight(data)]),
        79 => jam_reply_legacy(byte(0)?, byte(1)?, byte(2)?, data, output),
        80 => {
            let begin = JamBegin::parse(data)?;
            scalar(output, &[u32::from(begin.length), begin.crc])
        }
        81 => {
            let chunk = JamChunk::parse(data)?;
            if output.len() < 8 + usize::from(chunk.length) {
                return Err(CodecError::Capacity);
            }
            scalar(output, &[u32::from(chunk.index), u32::from(chunk.length)])?;
            Ok(8 + chunk.decode(&mut output[8..])?)
        }
        82 => {
            if data.len() != 3 {
                return Err(CodecError::Length);
            }
            if data[0] > 127 {
                return Err(CodecError::Range);
            }
            scalar(
                output,
                &[u32::from(data[0]), u32::from(value14(&data[1..])?)],
            )
        }
        83 => voice_ack(byte(0)?, byte(1)?, output),
        84 => scalar(output, &[u32::from(wire::sum7(data))]),
        85 => {
            let op = byte(0)?;
            let token = byte(1)?;
            if op > 127 || token > 127 || data.iter().any(|b| *b > 127) {
                return Err(CodecError::Range);
            }
            let length = data.len().checked_add(8).ok_or(CodecError::Length)?;
            if output.len() < length {
                return Err(CodecError::Capacity);
            }
            output[..6].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x60, op, token]);
            output[6..length - 2].copy_from_slice(data);
            output[length - 2] = wire::sum7(&output[4..length - 2]);
            output[length - 1] = 0xf7;
            Ok(length)
        }
        86 => {
            let reply = wire::JamReply::parse(data)?;
            if output.len() < 12 {
                return Err(CodecError::Capacity);
            }
            scalar(
                output,
                &[
                    u32::from(reply.operation()),
                    u32::from(reply.token()),
                    u32::from(reply.status()),
                ],
            )?;
            Ok(12 + reply.decode(&mut output[12..])?)
        }
        87 => {
            let mut scratch = [0u8; 640];
            let message = wire::decode_ota(data, &mut scratch)?;
            let body = message.body();
            if output.len() < 8 + body.len() {
                return Err(CodecError::Capacity);
            }
            scalar(output, &[u32::from(message.command()), body.len() as u32])?;
            output[8..8 + body.len()].copy_from_slice(body);
            Ok(8 + body.len())
        }
        88 => {
            let request = wire::JamRequest::parse(data)?;
            let payload = request.payload();
            if output.len() < 8 + payload.len() {
                return Err(CodecError::Capacity);
            }
            scalar(
                output,
                &[u32::from(request.operation()), u32::from(request.token())],
            )?;
            output[8..8 + payload.len()].copy_from_slice(payload);
            Ok(8 + payload.len())
        }
        89 => wire::decode_nibbles(data, output),
        90 => wire::encode_nibbles(data, output),
        91 => scalar(output, &[u32::from(value14(data)?)]),
        92 => {
            let mut packet = [0u8; 10];
            packet[..3].copy_from_slice(&[0xf0, 0x7d, 0x50]);
            let length = match arg(0)? {
                0 => {
                    packet[3..6].copy_from_slice(&[0x42, 1, (arg(1)? & 127) as u8]);
                    7
                }
                1 => {
                    let length = arg(2)? & 16383;
                    packet[3..8].copy_from_slice(&[
                        0x42,
                        4,
                        (arg(1)? & 127) as u8,
                        (length >> 7) as u8,
                        (length & 127) as u8,
                    ]);
                    9
                }
                2 => {
                    packet[3..5].copy_from_slice(&[0x42, 5]);
                    6
                }
                3 => {
                    packet[3..5].copy_from_slice(&[0x42, 6]);
                    6
                }
                4 => {
                    packet[3..5].copy_from_slice(&[0x50, 2]);
                    6
                }
                5 => {
                    let parameter = arg(1)?;
                    if parameter > 16383 {
                        return Err(CodecError::Range);
                    }
                    let value = arg(2)?.wrapping_add(8192) & 16383;
                    packet[3..9].copy_from_slice(&[
                        0x50,
                        1,
                        (parameter >> 7) as u8,
                        (parameter & 127) as u8,
                        (value >> 7) as u8,
                        (value & 127) as u8,
                    ]);
                    10
                }
                6 => {
                    packet[3..6].copy_from_slice(&[0x56, 0x7e, byte(1)?]);
                    7
                }
                7 => {
                    let enabled = byte(1)?;
                    if enabled > 1 {
                        return Err(CodecError::Range);
                    }
                    packet[3..5].copy_from_slice(&[0x46, enabled]);
                    6
                }
                _ => return Err(CodecError::Index),
            };
            if packet[1..length - 1].iter().any(|b| *b > 127) {
                return Err(CodecError::Range);
            }
            if output.len() < length {
                return Err(CodecError::Capacity);
            }
            packet[length - 1] = 0xf7;
            output[..length].copy_from_slice(&packet[..length]);
            Ok(length)
        }
        93 => {
            if data.len() > 512 {
                return Err(CodecError::Length);
            }
            let mut body = [0u8; 520];
            body[0] = byte(1)?;
            body[1..5].copy_from_slice(&arg(0)?.to_le_bytes());
            body[5..8].copy_from_slice(&[data.len() as u8, (data.len() >> 8) as u8, 0]);
            body[8..8 + data.len()].copy_from_slice(data);
            wire::encode_ota(0x30, &body[..8 + data.len()], output)
        }
        94 => {
            let mut scratch = [0u8; 640];
            if data.len() < 2 {
                return Err(CodecError::Length);
            }
            let length = unpack7_prefix(&data[1..data.len() - 1], &mut scratch)?;
            match arg(0)? {
                0 => {
                    if data[0] != 0xf0 || data[data.len() - 1] != 0xf7 {
                        return Err(CodecError::Magic);
                    }
                    if length != 15 || scratch[..3] != [0, 0x59, 0x30] {
                        return Err(CodecError::Length);
                    }
                    // Original updater validates payload checksum, but does
                    // not consume the decoded header's length field.
                    if !scratch[6..14]
                        .iter()
                        .fold(0u8, |sum, b| sum.wrapping_add(*b))
                        != scratch[14]
                    {
                        return Err(CodecError::Checksum);
                    }
                    let body = &scratch[6..14];
                    scalar(
                        output,
                        &[
                            u32::from(body[0]),
                            crate::read_u32(body, 1).unwrap(),
                            u32::from(body[5])
                                | (u32::from(body[6]) << 8)
                                | (u32::from(body[7]) << 16),
                        ],
                    )
                }
                1 => {
                    // Explicit legacy discovery profile: historical browser
                    // identities ignore outer framing, header length and CRC.
                    // Strict device messages still use wire::decode_ota.
                    if length != 34 || scratch[..3] != [0, 0x59, 0x11] {
                        return Err(CodecError::Length);
                    }
                    if output.len() < 27 {
                        return Err(CodecError::Capacity);
                    }
                    output[..27].copy_from_slice(&scratch[6..33]);
                    Ok(27)
                }
                _ => Err(CodecError::Index),
            }
        }
        95 => match arg(0)? {
            0 => wire::encode_ota(0x11, &[], output),
            1 => wire::encode_ota(0x11, &[1], output),
            2 => {
                if output.len() < 6 {
                    return Err(CodecError::Capacity);
                }
                output[..6].copy_from_slice(&[0xf0, 0x22, 0x24, 0x35, 0x7f, 0xf7]);
                Ok(6)
            }
            _ => Err(CodecError::Index),
        },
        _ => Err(CodecError::Index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_browser_discovery_and_masked_short_requests() {
        let mut raw = [0u8; 34];
        raw[..3].copy_from_slice(&[0, 0x59, 0x11]);
        raw[3] = 99; // Historical identity decoder ignores this length.
        raw[6..20].copy_from_slice(b"FM-1_7000302\0\0");
        raw[33] = 42; // Historical identity decoder also ignores checksum.
        let mut packet = [0u8; 64];
        let n = wire::pack7(&raw, &mut packet[1..]).unwrap() + 2;
        packet[0] = 1;
        packet[n - 1] = 2; // Outer bytes are ignored only by identity discovery.
        let mut out = [0u8; 64];
        assert_eq!(dispatch(94, &[1], &packet[..n], &mut out), Ok(27));
        assert_eq!(&out[..27], &raw[6..33]);
        assert_eq!(
            dispatch(94, &[0], &packet[..n], &mut out),
            Err(CodecError::Magic)
        );
        assert_eq!(dispatch(92, &[0, 255], &[], &mut out), Ok(7));
        assert_eq!(&out[..7], &[0xf0, 0x7d, 0x50, 0x42, 1, 127, 0xf7]);
        assert_eq!(dispatch(92, &[1, 255, 0x8123], &[], &mut out), Ok(9));
        assert_eq!(&out[..9], &[0xf0, 0x7d, 0x50, 0x42, 4, 127, 2, 35, 0xf7]);
    }
    #[test]
    fn telemetry_continuous_word_stream() {
        let words = [0x12345678, u32::MAX, 0, 0x80000001];
        let raw: [u8; 16] = core::array::from_fn(|i| words[i / 4].to_le_bytes()[i % 4]);
        let mut expected = [0u8; 32];
        let n = wire::pack7(&raw, &mut expected[4..]).unwrap();
        expected[..4].copy_from_slice(&[0xf0, 0x7d, 0x50, 0x46]);
        expected[4 + n] = 0xf7;
        let mut actual = [0u8; 32];
        assert_eq!(word_frame(0x46, &words, &mut actual).unwrap(), n + 5);
        assert_eq!(actual, expected);
        let mut recovered = [0u32; 4];
        assert_eq!(
            parse_word_frame(&actual[..n + 5], 0x46, &mut recovered),
            Ok(4)
        );
        assert_eq!(recovered, words);
    }
    #[test]
    fn truncated_ota_writes_only_the_prefix() {
        let mut packed = [0u8; 16];
        let n = wire::pack7(&[255, 128, 7, 42, 19], &mut packed).unwrap();
        for byte in &mut packed[..n] {
            *byte |= 128;
        }
        let mut out = [0xabu8; 4];
        assert_eq!(unpack7_prefix(&packed[..n], &mut out[..2]), Ok(5));
        assert_eq!(out, [255, 128, 0xab, 0xab]);
        assert_eq!(unpack7_prefix(&packed[..n], &mut []), Ok(5));
    }
    #[test]
    fn jam_error_precedence_and_fields() {
        let mut bytes = [0x7d, 0x50, 0x60, 1, 0, 0];
        bytes[5] = wire::sum7(&bytes[3..5]);
        assert_eq!(jam_preflight(&bytes), 0);
        bytes[3] |= 128;
        assert_eq!(jam_preflight(&bytes), 1);
        bytes[5] ^= 1;
        assert_eq!(jam_preflight(&bytes), 2);
        assert_eq!(
            JamBegin::parse(&[1, 2, 1, 2, 3, 4, 5, 6, 7, 8]),
            Ok(JamBegin {
                length: 130,
                crc: 0x12345678
            })
        );
        let c = JamChunk::parse(&[3, 0, 2, 15, 15, 8, 0]).unwrap();
        assert!(c.matches(&[255, 128]));
        assert!(!c.matches(&[255]));
    }
}
