// SPDX-License-Identifier: GPL-3.0-only
use super::{array, relative, Request, Writer};
use fm1_formats::{containers::*, CodecError};
pub(super) fn run(operation: u32, r: &Request<'_>, out: &mut [u8]) -> Result<usize, CodecError> {
    match operation {
        41 => {
            let voice = r.data(2)?;
            PieceParts {
                genre: r.arg(0)? as u8,
                slot: r.arg(1)? as u8,
                parameter_count: r.arg(2)? as u16,
                flags: r.arg(3)? as u8,
                record: array(r.data(0)?)?,
                progression: array(r.data(1)?)?,
                voice: if voice.is_empty() {
                    None
                } else {
                    Some(array(voice)?)
                },
                metadata: r.data(3)?,
            }
            .encode_compatible(out)
        }
        42 => {
            let data = r.data(0)?;
            let p = PieceFile::parse_compatible(data)?;
            let mut w = Writer::new(out);
            for v in [
                p.genre() as u32,
                p.slot() as u32,
                p.parameter_count() as u32,
                p.flags() as u32,
            ] {
                w.u32(v)?;
            }
            for part in [
                p.record().as_slice(),
                p.progression().as_slice(),
                p.voice().map_or(&[][..], |v| v.as_slice()),
                p.metadata(),
            ] {
                w.u32(relative(data, part) as u32)?;
                w.u32(part.len() as u32)?;
            }
            Ok(w.len())
        }
        43 => {
            let config = r.data(1)?;
            let records = r.data(2)?;
            let progressions = r.data(3)?;
            let voice_data = r.data(4)?;
            if !config.len().is_multiple_of(16) || !voice_data.len().is_multiple_of(128) {
                return Err(CodecError::Length);
            }
            let count = config.len() / 16;
            let nvoices = voice_data.len() / 128;
            if count > 255
                || nvoices > 255
                || records.len() != count * 376
                || progressions.len() != count * 6
            {
                return Err(CodecError::Length);
            }
            const ZERO_RECORD: [u8; 376] = [0; 376];
            const ZERO_PRG: [u8; 6] = [0; 6];
            const ZERO_VOICE: [u8; 128] = [0; 128];
            let empty = SetPiece {
                genre: 0,
                slot: 0,
                voice_index: 255,
                parameter_count: 0,
                record: &ZERO_RECORD,
                progression: &ZERO_PRG,
            };
            let mut pieces = [empty; 255];
            for (i, piece) in pieces.iter_mut().enumerate().take(count) {
                let get = |field: usize| fm1_formats::read_u32(config, i * 16 + field * 4).unwrap();
                *piece = SetPiece {
                    genre: get(0) as u8,
                    slot: get(1) as u8,
                    voice_index: get(3) as u8,
                    parameter_count: get(2) as u16,
                    record: array(&records[i * 376..(i + 1) * 376])?,
                    progression: array(&progressions[i * 6..(i + 1) * 6])?,
                };
            }
            let mut voices = [ZERO_VOICE; 255];
            for (i, voice) in voices.iter_mut().enumerate().take(nvoices) {
                voice.copy_from_slice(&voice_data[i * 128..(i + 1) * 128]);
            }
            SetParts {
                list: array(r.data(0)?)?,
                play: r.arg(0)? as u8,
                mix: r.arg(1)? as u8,
                pieces: &pieces[..count],
                voices: &voices[..nvoices],
                metadata: r.data(5)?,
            }
            .encode_compatible(out)
        }
        44 => {
            let data = r.data(0)?;
            let p = SetFile::parse_compatible(data)?;
            let mut w = Writer::new(out);
            for v in [
                relative(data, p.list()) as u32,
                p.play() as u32,
                p.mix() as u32,
                p.piece_count() as u32,
                p.voice_count() as u32,
                relative(data, p.metadata()) as u32,
                p.metadata().len() as u32,
            ] {
                w.u32(v)?;
            }
            for i in 0..p.piece_count() {
                let piece = p.piece(i)?;
                for v in [
                    piece.genre as u32,
                    piece.slot as u32,
                    piece.voice_index as u32,
                    piece.parameter_count as u32,
                    relative(data, piece.record) as u32,
                    relative(data, piece.progression) as u32,
                ] {
                    w.u32(v)?;
                }
            }
            for i in 0..p.voice_count() {
                w.u32(relative(data, p.voice(i)?) as u32)?;
            }
            Ok(w.len())
        }
        45 => {
            let config = r.data(1)?;
            let payload = r.data(2)?;
            if !config.len().is_multiple_of(12) || config.len() / 12 > 255 {
                return Err(CodecError::Length);
            }
            let count = config.len() / 12;
            let mut objects = [BackupObject { id: 0, data: &[] }; 255];
            for (i, object) in objects.iter_mut().enumerate().take(count) {
                let id = fm1_formats::read_u32(config, i * 12).unwrap() as u8;
                let at = fm1_formats::read_u32(config, i * 12 + 4).unwrap() as usize;
                let n = fm1_formats::read_u32(config, i * 12 + 8).unwrap() as usize;
                let end = at.checked_add(n).ok_or(CodecError::Length)?;
                *object = BackupObject {
                    id,
                    data: payload.get(at..end).ok_or(CodecError::Length)?,
                };
            }
            backup_encode(&objects[..count], r.data(0)?, out)
        }
        46 => {
            let data = r.data(0)?;
            let p = BackupFile::parse(data)?;
            let mut w = Writer::new(out);
            for v in [
                relative(data, p.metadata()) as u32,
                p.metadata().len() as u32,
                p.objects().count() as u32,
                relative(data, p.trailing()) as u32,
                p.trailing().len() as u32,
            ] {
                w.u32(v)?;
            }
            for object in p.objects() {
                w.u32(object.id as u32)?;
                w.u32(relative(data, object.data) as u32)?;
                w.u32(object.data.len() as u32)?;
            }
            Ok(w.len())
        }
        _ => Err(CodecError::Version),
    }
}
