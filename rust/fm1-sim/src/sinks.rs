// SPDX-License-Identifier: GPL-3.0-only
use std::{
    fs::File,
    io::{self, BufWriter, Seek, SeekFrom, Write},
    path::Path,
};
fn crc(bytes: &[u8]) -> u32 {
    let mut c = u32::MAX;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb88320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}
fn chunk(out: &mut Vec<u8>, name: &[u8; 4], bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(name);
    out.extend_from_slice(bytes);
    let c = crc(&out[start..]);
    out.extend_from_slice(&c.to_be_bytes());
}
/// Exact original RGB565 expansion and stored-deflate PNG byte layout.
pub fn png(frame: &[u16; 57600]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(173040);
    for row in frame.chunks_exact(240) {
        raw.push(0);
        for p in row {
            let p = p.swap_bytes();
            raw.extend_from_slice(&[
                (((p >> 11) & 31) * 255 / 31) as u8,
                (((p >> 5) & 63) * 255 / 63) as u8,
                ((p & 31) * 255 / 31) as u8,
            ]);
        }
    }
    let mut a = 1u32;
    let mut b = 0u32;
    for v in &raw {
        a = (a + *v as u32) % 65521;
        b = (b + a) % 65521;
    }
    let mut z = vec![0x78, 0x01];
    let count = raw.len().div_ceil(65535);
    for (i, part) in raw.chunks(65535).enumerate() {
        let n = part.len() as u16;
        z.push((i + 1 == count) as u8);
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(part);
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![137, 80, 78, 71, 13, 10, 26, 10];
    chunk(
        &mut out,
        b"IHDR",
        &[0, 0, 0, 240, 0, 0, 0, 240, 8, 2, 0, 0, 0],
    );
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}
pub struct Wav {
    file: BufWriter<File>,
    frames: u32,
}
impl Wav {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = BufWriter::new(File::create(path)?);
        file.write_all(&[0; 44])?;
        Ok(Self { file, frames: 0 })
    }
    pub fn append(&mut self, samples: &[i16]) -> io::Result<()> {
        let mut bytes = [0u8; 1024];
        for part in samples.chunks(512) {
            for (s, b) in part.iter().zip(bytes.chunks_exact_mut(2)) {
                b.copy_from_slice(&s.to_le_bytes());
            }
            self.file.write_all(&bytes[..part.len() * 2])?;
        }
        self.frames = self.frames.wrapping_add((samples.len() / 2) as u32);
        Ok(())
    }
    pub fn finish(mut self) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&wav_header(self.frames))
    }
}
pub fn wav_header(frames: u32) -> [u8; 44] {
    let bytes = frames.wrapping_mul(4);
    let mut h = [0; 44];
    h[..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&bytes.wrapping_add(36).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..24].copy_from_slice(&[1, 0, 2, 0]);
    h[24..28].copy_from_slice(&44100u32.to_le_bytes());
    h[28..32].copy_from_slice(&176400u32.to_le_bytes());
    h[32..36].copy_from_slice(&[4, 0, 16, 0]);
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&bytes.to_le_bytes());
    h
}
pub fn pcm(samples: &[i32; 512], peak: &mut f32) -> [i16; 512] {
    let mut out = [0; 512];
    for (a, b) in samples.iter().zip(out.iter_mut()) {
        let v = *a as f32 / 8388608.0;
        *peak = peak.max(v.abs());
        *b = (v * 32767.0) as i16;
    }
    out
}
#[test]
fn file_layout_pcm_and_png_are_stable() {
    let h = wav_header(256);
    assert_eq!(&h[40..], &1024u32.to_le_bytes());
    assert_eq!(&h[8..16], b"WAVEfmt ");
    let mut p = 0.0;
    let mut input = [0; 512];
    input[0] = 8388607;
    input[1] = -8388608;
    let out = pcm(&input, &mut p);
    assert_eq!((out[0], out[1], p), (32766, -32767, 1.0));
    let frame = Box::new([0; 57600]);
    let a = png(&frame);
    assert_eq!(a.len(), 173118);
    assert_eq!(&a[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert_eq!(&a[a.len() - 4..], &[174, 66, 96, 130]);
}
