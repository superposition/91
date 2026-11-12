// SPDX-License-Identifier: GPL-3.0-only
//! Safe bounded operation dispatcher shared by native tests and the WASM ABI.
#![forbid(unsafe_code)]
use fm1_formats::{package, voice, wire, CodecError};
mod files;
mod objects;
pub const HEADER: usize = 256;
pub fn error_code(error: CodecError) -> i32 {
    -match error {
        CodecError::Length => 1,
        CodecError::Magic => 2,
        CodecError::Version => 3,
        CodecError::Index => 4,
        CodecError::Range => 5,
        CodecError::Checksum => 6,
        CodecError::Nibble => 7,
        CodecError::Capacity => 8,
        CodecError::State => 9,
    }
}
pub(super) struct Request<'a> {
    bytes: &'a [u8],
}
impl<'a> Request<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if bytes.len() < HEADER {
            return Err(CodecError::Length);
        }
        Ok(Self { bytes })
    }
    pub fn arg(&self, index: usize) -> Result<u32, CodecError> {
        if index >= 32 {
            return Err(CodecError::Index);
        }
        fm1_formats::read_u32(self.bytes, index * 4).ok_or(CodecError::Length)
    }
    pub fn data(&self, index: usize) -> Result<&'a [u8], CodecError> {
        if index >= 16 {
            return Err(CodecError::Index);
        }
        let at = (32 + index * 2) * 4;
        let offset = fm1_formats::read_u32(self.bytes, at).unwrap() as usize;
        let length = fm1_formats::read_u32(self.bytes, at + 4).unwrap() as usize;
        if length == 0 {
            return Ok(&self.bytes[..0]);
        }
        if offset < HEADER {
            return Err(CodecError::Length);
        }
        let end = offset.checked_add(length).ok_or(CodecError::Length)?;
        self.bytes.get(offset..end).ok_or(CodecError::Length)
    }
}
pub(super) struct Writer<'a> {
    bytes: &'a mut [u8],
    at: usize,
}
impl<'a> Writer<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn u32(&mut self, value: u32) -> Result<(), CodecError> {
        self.put(&value.to_le_bytes())
    }
    pub fn put(&mut self, value: &[u8]) -> Result<(), CodecError> {
        let end = self
            .at
            .checked_add(value.len())
            .ok_or(CodecError::Capacity)?;
        let out = self
            .bytes
            .get_mut(self.at..end)
            .ok_or(CodecError::Capacity)?;
        out.copy_from_slice(value);
        self.at = end;
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.at
    }
}
pub(super) fn copy(bytes: &[u8], out: &mut [u8]) -> Result<usize, CodecError> {
    if out.len() < bytes.len() {
        return Err(CodecError::Capacity);
    }
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}
pub(super) fn array<const N: usize>(bytes: &[u8]) -> Result<&[u8; N], CodecError> {
    bytes.try_into().map_err(|_| CodecError::Length)
}
pub(super) fn u16s<const N: usize>(bytes: &[u8]) -> Result<[u16; N], CodecError> {
    if bytes.len() != N * 2 {
        return Err(CodecError::Length);
    }
    let mut values = [0; N];
    for (i, v) in values.iter_mut().enumerate() {
        *v = fm1_formats::read_u16(bytes, i * 2).unwrap();
    }
    Ok(values)
}
pub(super) fn i16s<const N: usize>(bytes: &[u8]) -> Result<[i16; N], CodecError> {
    Ok(u16s::<N>(bytes)?.map(|v| v as i16))
}
pub(super) fn relative(root: &[u8], part: &[u8]) -> usize {
    if part.is_empty() {
        0
    } else {
        part.as_ptr() as usize - root.as_ptr() as usize
    }
}
pub fn run(operation: u32, input: &[u8], out: &mut [u8]) -> Result<usize, CodecError> {
    let r = Request::new(input)?;
    if (70..=95).contains(&operation) {
        let mut args = [0; 16];
        for (i, value) in args.iter_mut().enumerate() {
            *value = r.arg(i)?;
        }
        return fm1_formats::device_wire::dispatch(operation, &args, r.data(0)?, out);
    }
    if (41..=46).contains(&operation) {
        return files::run(operation, &r, out);
    }
    if (47..=65).contains(&operation) || operation == 68 || operation == 69 {
        return objects::run(operation, &r, out);
    }
    match operation {
        0 => {
            let mut w = Writer::new(out);
            for v in [
                1, 16, 376, 10, 6, 74, 255, 3776, 3836, 16, 2056, 3, 0x31444e4b, 8, 16, 7,
                0x31584350, 0x31515342, 32, 3840, 256, 64, 48, 152, 176,
            ] {
                w.u32(v)?;
            }
            Ok(w.len())
        }
        1 => copy(&fm1_formats::crc32(r.data(0)?).to_le_bytes(), out),
        2 => copy(
            &(package::crc16(r.data(0)?, r.arg(0)? as u16) as u32).to_le_bytes(),
            out,
        ),
        3 => copy(&(wire::sum7(r.data(0)?) as u32).to_le_bytes(), out),
        4 => wire::pack7(r.data(0)?, out),
        5 => wire::unpack7(r.data(0)?, out),
        6 => wire::encode_nibbles(r.data(0)?, out),
        7 => wire::decode_nibbles(r.data(0)?, out),
        8 => {
            let n = copy(r.data(0)?, out)?;
            package::enc(&mut out[..n], r.arg(0)? as u16);
            Ok(n)
        }
        9 => {
            let data = r.data(0)?;
            let offset = r.arg(0)?;
            let base = r.arg(1)?;
            if offset < base
                || offset
                    .checked_add(u32::try_from(data.len()).map_err(|_| CodecError::Length)?)
                    .is_none()
            {
                return Err(CodecError::Range);
            }
            let n = copy(data, out)?;
            package::sfc(&mut out[..n], offset, base, r.arg(2)? as u16)?;
            Ok(n)
        }
        10 => copy(&(package::chip_key(r.data(0)?)? as u32).to_le_bytes(), out),
        11 => copy(&package::key_blob(r.arg(0)? as u16), out),
        12 => copy(&package::body_marker(r.data(0)?)?, out),
        13 => copy(
            &package::compensate(r.data(0)?, r.arg(0)? as usize, r.arg(1)? as u16)?,
            out,
        ),
        14 => package::Fwsc::parse(r.data(0)?)?.product(out),
        15 => {
            let p = package::Fwsc::parse(r.data(0)?)?;
            let n = p.logical_len();
            if n > out.len() {
                return Err(CodecError::Capacity);
            }
            p.copy_logical(0, &mut out[..n])?;
            Ok(n)
        }
        16 => package::wrap_logical(r.data(0)?, r.data(1)?, out),
        17 => {
            let e = package::JlfsEntry::parse(r.data(0)?)?;
            let mut w = Writer::new(out);
            for v in [
                e.data_checksum() as u32,
                e.offset(),
                e.len(),
                e.flags() as u32,
                e.reserved() as u32,
                e.index() as u32,
            ] {
                w.u32(v)?;
            }
            w.put(e.raw_name())?;
            Ok(w.len())
        }
        18 => copy(
            &package::JlfsParts {
                data_checksum: r.arg(0)? as u16,
                offset: r.arg(1)?,
                length: r.arg(2)?,
                flags: r.arg(3)? as u8,
                reserved: r.arg(4)? as u8,
                index: r.arg(5)? as u16,
                raw_name: array(r.data(0)?)?,
            }
            .encode(),
            out,
        ),
        19 => {
            let p = package::Fwsc::parse(r.data(0)?)?;
            let d = p.directory()?;
            let mut w = Writer::new(out);
            w.u32(d.len() as u32)?;
            w.u32(d.total() as u32)?;
            w.put(d.header())?;
            for i in 0..d.len() {
                w.put(d.entry(i)?.bytes())?;
            }
            Ok(w.len())
        }
        20 => copy(&voice::unpack(array(r.data(0)?)?), out),
        21 => copy(&voice::pack(array(r.data(0)?)?), out),
        22 => {
            let mut v = *array::<155>(r.data(0)?)?;
            voice::sanitize(&mut v);
            copy(&v, out)
        }
        23 => {
            let bytes = r.data(0)?;
            let view = voice::DxMessage::parse(bytes)?;
            let mut w = Writer::new(out);
            w.u32(view.kind() as u32)?;
            w.u32(view.channel().map_or(255, |v| v as u32))?;
            w.u32(view.slot().map_or(255, |v| v as u32))?;
            let payload = match view.kind() {
                voice::IncomingKind::Bulk => &bytes[6..4102],
                voice::IncomingKind::Single => view.single().unwrap().as_slice(),
                voice::IncomingKind::Slot => view.packed(0)?.as_slice(),
                _ => &[],
            };
            w.u32(relative(bytes, payload) as u32)?;
            w.u32(payload.len() as u32)?;
            Ok(w.len())
        }
        24 => match r.arg(0)? {
            1 => voice::dx7_bulk(r.arg(1)? as u8, array(r.data(0)?)?, out),
            2 => voice::dx7_single(r.arg(1)? as u8, array(r.data(0)?)?, out),
            3 => voice::slot_voice(r.arg(1)? as u8, Some(array(r.data(0)?)?), out),
            4 => voice::slot_voice(0, None, out),
            _ => Err(CodecError::Version),
        },
        25 => {
            let bytes = r.data(0)?;
            if !bytes.len().is_multiple_of(4) || bytes.len() > 96 {
                return Err(CodecError::Length);
            }
            let mut vals = [0i32; 24];
            for (i, v) in vals.iter_mut().enumerate().take(bytes.len() / 4) {
                *v = fm1_formats::read_u32(bytes, i * 4).unwrap() as i32;
            }
            wire::editor_dump(r.arg(0)? as u16, &vals[..bytes.len() / 4], out)
        }
        26 => {
            let view = wire::EditorDump::parse(r.data(0)?)?;
            let mut w = Writer::new(out);
            w.u32(view.first() as u32)?;
            w.u32(view.len() as u32)?;
            for i in 0..view.len() {
                w.u32(view.value(i)? as i32 as u32)?;
            }
            Ok(w.len())
        }
        27 => {
            let data = r.data(0)?;
            wire::editor_voice(
                (r.arg(0)? & 127) as u8,
                (r.arg(1)? & 127) as u8,
                if data.is_empty() {
                    None
                } else {
                    Some(array(data)?)
                },
                out,
            )
        }
        28 => {
            let bytes = r.data(0)?;
            let v = wire::VoiceReply::parse(bytes)?;
            let mut w = Writer::new(out);
            w.u32(v.slot() as u32)?;
            w.u32(v.loaded() as u32)?;
            w.put(v.voice().map_or(&[][..], |v| v.as_slice()))?;
            Ok(w.len())
        }
        29 => wire::backup_frame(
            (r.arg(0)? & 127) as u8,
            (r.arg(1)? & 16383) as u16,
            (r.arg(2)? & 16383) as u16,
            r.data(0)?,
            out,
        ),
        30 | 32 => {
            let v = if operation == 30 {
                wire::BackupData::parse_frame(r.data(0)?)?
            } else {
                wire::BackupData::parse_chunk_legacy(r.data(0)?)?
            };
            let mut data = [0; 256];
            let n = v.decode(&mut data)?;
            let mut w = Writer::new(out);
            w.u32(v.object as u32)?;
            w.u32(v.index as u32)?;
            w.u32(v.total as u32)?;
            w.put(&data[..n])?;
            Ok(w.len())
        }
        31 => wire::backup_chunk(
            (r.arg(0)? & 127) as u8,
            (r.arg(1)? & 16383) as u16,
            (r.arg(2)? & 16383) as u16,
            r.data(0)?,
            out,
        ),
        33 => wire::backup_ack(
            (r.arg(0)? & 127) as u8,
            (r.arg(1)? & 127) as u8,
            (r.arg(2)? & 127) as u8,
            out,
        ),
        34 => wire::about(
            (r.arg(0)? & 127) as u8,
            (r.arg(1)? & 16383) as u16,
            (r.arg(2)? & 127) as u8,
            (r.arg(3)? & 127) as u8,
            out,
        ),
        35 => {
            let v = wire::ControlReply::parse(r.data(0)?)?;
            let mut w = Writer::new(out);
            let fields = match v {
                wire::ControlReply::BackupAck {
                    object,
                    index,
                    status,
                } => [0x44, object as u32, index as u32, status as u32, 0],
                wire::ControlReply::About {
                    version,
                    parameters,
                    genres,
                    voices,
                } => [
                    0x45,
                    version as u32,
                    parameters as u32,
                    genres as u32,
                    voices as u32,
                ],
                wire::ControlReply::VoiceAck { kind, count } => {
                    [0x57, kind as u32, count as u32, 0, 0]
                }
            };
            for v in fields {
                w.u32(v)?;
            }
            Ok(w.len())
        }
        36 => wire::jam_reply(
            r.arg(0)? as u8,
            r.arg(1)? as u8,
            r.arg(2)? as u8,
            r.data(0)?,
            out,
        ),
        37 => {
            let v = wire::JamReply::parse(r.data(0)?)?;
            let mut data = [0; 68];
            let n = v.decode(&mut data)?;
            let mut w = Writer::new(out);
            w.u32(v.operation() as u32)?;
            w.u32(v.token() as u32)?;
            w.u32(v.status() as u32)?;
            w.put(&data[..n])?;
            Ok(w.len())
        }
        38 => {
            let v = wire::JamRequest::parse(r.data(0)?)?;
            let mut w = Writer::new(out);
            w.u32(v.operation() as u32)?;
            w.u32(v.token() as u32)?;
            w.put(v.payload())?;
            Ok(w.len())
        }
        39 => wire::encode_ota(r.arg(0)? as u8, r.data(0)?, out),
        40 => {
            let encoded = r.data(0)?;
            let mut scratch = [0; 4112];
            let v = wire::decode_ota(encoded, &mut scratch)?;
            let mut w = Writer::new(out);
            w.u32(v.command() as u32)?;
            w.put(v.body())?;
            Ok(w.len())
        }
        66 => fm1_formats::voice_file::decode_compatible(r.data(0)?, out),
        97 => fm1_formats::jam::JamPack::parse_structural(r.data(0)?)
            .map_err(|_| CodecError::Range)?
            .encode_into(out),
        98 => fm1_formats::session::JamSession::decode(r.data(0)?)
            .map_err(|_| CodecError::Range)?
            .encode_into(out)
            .map_err(|_| CodecError::Capacity),
        96 => {
            let data = r.data(0)?;
            let mut w = Writer::new(out);
            w.u32(fm1_formats::voice_file::FileVoices::new(data).count() as u32)?;
            for v in fm1_formats::voice_file::FileVoices::new(data) {
                for n in [
                    v.packed as u32,
                    relative(data, v.bytes) as u32,
                    v.bytes.len() as u32,
                    relative(data, v.name) as u32,
                ] {
                    w.u32(n)?;
                }
            }
            Ok(w.len())
        }
        67 => {
            if out.len() < 155 {
                return Err(CodecError::Capacity);
            }
            for (i, value) in out[..155].iter_mut().enumerate() {
                *value = voice::maximum(i).unwrap();
            }
            Ok(155)
        }
        _ => Err(CodecError::Version),
    }
}
