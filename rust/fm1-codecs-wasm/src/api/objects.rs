// SPDX-License-Identifier: GPL-3.0-only
use super::{array, copy, i16s, relative, u16s, Request, Writer};
use fm1_formats::{
    containers::PieceParts,
    objects::*,
    storage::{self, Record},
    voice::PackedVoice,
    CodecError,
};
fn optional<const N: usize>(v: &[u8]) -> Result<Option<&[u8; N]>, CodecError> {
    if v.is_empty() {
        Ok(None)
    } else {
        Ok(Some(array(v)?))
    }
}
pub(super) fn run(op: u32, r: &Request<'_>, out: &mut [u8]) -> Result<usize, CodecError> {
    let a = |i| r.arg(i);
    let b = |i| r.data(i);
    match op {
        47 => copy(
            &(storage::restore_valid(
                a(0)?,
                b(0)?,
                storage::RestoreLimits {
                    effects: a(1)? as u16,
                    settings_len: a(2)? as usize,
                },
            ) as u32)
                .to_le_bytes(),
            out,
        ),
        48 => storage::RecordValues {
            parameters: i16s(b(0)?)?,
            pool: a(0)? as u16,
            patterns: u16s(b(1)?)?,
            modifiers: *array(b(2)?)?,
            acid: *array(b(3)?)?,
            pluck: *array(b(4)?)?,
        }
        .encode_legacy(out),
        49 => {
            let d = Record::parse(b(0)?)?.decode_legacy(a(0)? as u16, &i16s(b(1)?)?);
            let mut w = Writer::new(out);
            w.u32(d.valid as u32)?;
            for v in d.values.parameters {
                w.put(&v.to_le_bytes())?;
            }
            w.put(&d.values.pool.to_le_bytes())?;
            for v in d.values.patterns {
                w.put(&v.to_le_bytes())?;
            }
            w.put(&d.values.modifiers)?;
            w.put(&d.values.acid)?;
            w.put(&d.values.pluck)?;
            Ok(w.len())
        }
        50 => GlobalParts {
            values: i16s(b(0)?)?,
            list: optional(b(1)?)?,
            play: a(0)? as u8,
            reserved: [a(1)? as u8, a(2)? as u8, a(3)? as u8],
        }
        .encode_compatible(out),
        51 => SettingsParts {
            effects: b(0)?,
            performance: array(b(1)?)?,
            targets: array(b(2)?)?,
            pattern: a(0)? as u16,
            usb: a(1)? as u16,
        }
        .encode_preserving(if b(3)?.is_empty() { None } else { Some(b(3)?) }, out),
        52 => {
            let bytes = b(0)?;
            if !bytes.len().is_multiple_of(128) || bytes.len() > 2048 {
                return Err(CodecError::Length);
            }
            let empty = PackedVoice::parse(&[0; 128]).unwrap();
            let mut voices = [empty; 16];
            for (i, chunk) in bytes.chunks_exact(128).enumerate() {
                voices[i] = PackedVoice::parse(chunk)?;
            }
            VoicesParts {
                voices: &voices[..bytes.len() / 128],
            }
            .encode_preserving(if b(1)?.is_empty() { None } else { Some(b(1)?) }, out)
        }
        53 => KindsParts { kinds: b(0)? }.encode(out),
        54 => ExtensionsParts {
            kind: match a(0)? {
                0 => ExtensionKind::Extras,
                1 => ExtensionKind::Bass,
                _ => return Err(CodecError::Version),
            },
            genres: a(1)? as u8,
            reserved: [a(2)? as u8, a(3)? as u8],
            records: b(0)?,
        }
        .encode(out),
        55 => {
            let bytes = b(0)?;
            if !bytes.len().is_multiple_of(3) || bytes.len() > 144 {
                return Err(CodecError::Length);
            }
            let empty = MidiEntry::parse([0, 0, 0]).unwrap();
            let mut entries = [empty; 48];
            for (i, v) in bytes.chunks_exact(3).enumerate() {
                entries[i] = MidiEntry::parse(*array(v)?)?;
            }
            MidiMapParts {
                version: if a(0)? == 0 { None } else { Some(a(0)? as u8) },
                entries: &entries[..bytes.len() / 3],
                reserved: [a(1)? as u8, a(2)? as u8, a(3)? as u8],
            }
            .encode_preserving(if b(1)?.is_empty() { None } else { Some(b(1)?) }, out)
        }
        56 => BankParts {
            genre: a(0)? as u8,
            parameters: a(1)? as u16,
            saved: a(2)? as u16,
            answer_encoding: a(3)? as u8,
            voice_numbering: a(4)? as u8,
            progression_flag: a(5)? as u8,
            reserved: a(6)? as u8,
            records: array(b(0)?)?,
            progressions: optional(b(1)?)?,
            trailing: b(2)?,
        }
        .encode(out),
        57 => LegacyKitParts {
            format: a(0)?,
            patterns: &u16s(b(0)?)?,
            lanes: &i16s(b(1)?)?,
            theme: a(1)? as u8,
            reserved: array(b(2)?)?,
        }
        .encode(out),
        58 => {
            let bytes = b(0)?;
            if !bytes.len().is_multiple_of(2) || bytes.len() > 352 {
                return Err(CodecError::Length);
            }
            let mut values = [0i16; 176];
            for (i, v) in values.iter_mut().enumerate().take(bytes.len() / 2) {
                *v = fm1_formats::read_u16(bytes, i * 2).unwrap() as i16;
            }
            LegacyRecordParts {
                parameters: &values[..bytes.len() / 2],
                pool: a(0)? as u16,
                patterns: &u16s(b(1)?)?,
            }
            .encode(out)
        }
        59 => {
            let count = a(0)? as usize;
            if !(1..=176).contains(&count)
                || b(0)?.len() != count * 20
                || b(1)?.len() != 20
                || b(2)?.len() != 720
            {
                return Err(CodecError::Length);
            }
            let mut params = [[0i16; 176]; 10];
            let mut patterns = [[0u16; 36]; 10];
            let mut pools = [0u16; 10];
            for i in 0..10 {
                for (j, value) in params[i].iter_mut().enumerate().take(count) {
                    *value = fm1_formats::read_u16(b(0)?, (i * count + j) * 2).unwrap() as i16;
                }
                pools[i] = fm1_formats::read_u16(b(1)?, i * 2).unwrap();
                patterns[i] = u16s(&b(2)?[i * 72..(i + 1) * 72])?;
            }
            let records = core::array::from_fn(|i| LegacyRecordParts {
                parameters: &params[i][..count],
                pool: pools[i],
                patterns: &patterns[i],
            });
            LegacyBankParts {
                parameters: count as u16,
                saved: a(1)? as u16,
                globals: [a(2)? as i16, a(3)? as i16, a(4)? as i16],
                revision: a(5)? as u16,
                records: &records,
            }
            .encode(out)
        }
        60 => {
            let bank = storage::Bank::parse(
                b(0)?,
                if a(0)? == 255 {
                    *b(0)?.get(5).ok_or(CodecError::Length)?
                } else {
                    a(0)? as u8
                },
            )?;
            let mut w = Writer::new(out);
            for v in [
                bank.modern() as u32,
                bank.genre() as u32,
                bank.parameter_count() as u32,
                bank.saved() as u32,
            ] {
                w.u32(v)?;
            }
            for v in bank.encoding_flags().unwrap_or([0; 4]) {
                w.u32(v as u32)?;
            }
            Ok(w.len())
        }
        61 => {
            let bank = storage::Bank::parse(b(0)?, a(0)? as u8)?;
            let slot = a(1)? as usize;
            if slot >= 10 {
                return Err(CodecError::Index);
            }
            if bank.saved() >> slot & 1 == 0 {
                return Err(CodecError::State);
            }
            let record = bank.record(slot)?;
            PieceParts {
                genre: a(0)? as u8,
                slot: slot as u8,
                parameter_count: bank.parameter_count(),
                flags: 0,
                record: record.bytes(),
                progression: bank.progression(slot)?.unwrap_or(&[0; 6]),
                voice: None,
                metadata: &[],
            }
            .encode_compatible(out)
        }
        62 => {
            if out.len() < 3848 {
                return Err(CodecError::Capacity);
            }
            let result = import_piece(
                b(0)?,
                a(0)? as u8,
                a(1)? as usize,
                a(2)? as u16,
                a(3)? as u16,
                array(b(1)?)?,
                array(b(2)?)?,
                &mut out[12..],
            )?;
            let mut w = Writer::new(out);
            w.u32(result.dropped as u32)?;
            w.u32(result.filled as u32)?;
            w.u32(result.replaced as u32)?;
            Ok(12 + 3836)
        }
        63 => extension_get(b(0)?, a(0)? as usize, a(1)? as usize, a(2)?, out),
        64 => extension_update(
            b(0)?,
            a(0)? as usize,
            a(1)? as usize,
            a(2)? as u8,
            a(3)?,
            optional(b(1)?)?,
            optional(b(2)?)?,
            optional(b(3)?)?,
            out,
        ),
        65 => {
            let map = storage::MidiMap::parse_compatible(b(0)?)?;
            let mut w = Writer::new(out);
            w.u32(map.version() as u32)?;
            w.u32(map.len() as u32)?;
            w.put(&map.reserved())?;
            for i in 0..map.len() {
                w.put(&map.entry(i)?)?;
            }
            Ok(w.len())
        }
        68 => {
            let record = Record::parse(b(0)?)?;
            let index = a(0)? as usize;
            if index >= 176 {
                return Err(CodecError::Index);
            }
            if a(3)? != 0 {
                let mut bytes = *record.bytes();
                bytes[74 + index] = a(4)? as u8;
                copy(&bytes, out)
            } else {
                let value = record.parameter(index, a(1)? as i16, a(2)? as u16)?;
                let mut w = Writer::new(out);
                w.u32(value.is_some() as u32)?;
                w.u32(value.unwrap_or(0) as i32 as u32)?;
                Ok(w.len())
            }
        }
        69 => {
            let bytes = b(0)?;
            match a(0)? {
                0 => {
                    if bytes.len() < 12 || &bytes[..4] != b"PMGL" {
                        return Err(CodecError::Magic);
                    }
                    let mut w = Writer::new(out);
                    for i in 0..4 {
                        w.u32(
                            fm1_formats::read_u16(bytes, 4 + i * 2).unwrap() as i16 as i32 as u32,
                        )?;
                    }
                    if bytes.len() >= 32 {
                        w.put(&bytes[12..32])?;
                    }
                    Ok(w.len())
                }
                1 => {
                    if bytes.len() != 2056 || &bytes[..4] != b"VOX1" {
                        return Err(CodecError::Magic);
                    }
                    let n = fm1_formats::read_u32(bytes, 4).unwrap() as usize;
                    if n > 16 {
                        return Err(CodecError::Range);
                    }
                    copy(&bytes[8..8 + n * 128], out)
                }
                2 => {
                    let v = PackedVoice::parse(bytes)?;
                    copy(&v.bytes()[118..128], out)
                }
                3 => {
                    let settings = storage::Settings::parse(
                        bytes,
                        storage::RestoreLimits {
                            effects: a(1)? as u16,
                            settings_len: bytes.len(),
                        },
                    )?;
                    let mut w = Writer::new(out);
                    w.u32(settings.pattern() as u32)?;
                    w.u32(settings.usb_flags() as u32)?;
                    w.put(settings.effects())?;
                    w.put(settings.performance())?;
                    w.put(settings.targets_and_visualizer())?;
                    Ok(w.len())
                }
                4 => {
                    let bank = storage::Bank::parse(bytes, a(1)? as u8)?;
                    let mut w = Writer::new(out);
                    w.u32(relative(bytes, bank.bytes()) as u32)?;
                    Ok(w.len())
                }
                _ => Err(CodecError::Version),
            }
        }
        _ => Err(CodecError::Version),
    }
}
