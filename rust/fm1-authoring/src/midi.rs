// SPDX-License-Identifier: GPL-3.0-only
use crate::{compile, decode, pack::Limits, Error, Result};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
const FILE_MAX: usize = 16 * 1024 * 1024;
#[derive(Clone, Debug)]
struct Note {
    tick: u64,
    note: u8,
    velocity: u8,
}
#[derive(Default)]
struct Track {
    name: String,
    named: bool,
    notes: Vec<Note>,
    channels: BTreeSet<u8>,
    non_four: bool,
}
struct Smf {
    kind: u16,
    ticks: i16,
    tracks: Vec<Track>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| Error::value("invalid MIDI length"))?;
        let b = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| Error::value("truncated MIDI file"))?;
        self.at = end;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn long(&mut self) -> Result<usize> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()) as usize)
    }
    fn variable(&mut self) -> Result<u64> {
        let mut n = 0u64;
        for _ in 0..9 {
            let b = self.byte()?;
            n = n
                .checked_mul(128)
                .and_then(|n| n.checked_add((b & 127) as u64))
                .ok_or_else(|| Error::value("MIDI variable integer overflow"))?;
            if b < 128 {
                return Ok(n);
            }
        }
        Err(Error::value("MIDI variable integer overflow"))
    }
}
fn parse(bytes: &[u8]) -> Result<Smf> {
    if bytes.len() > FILE_MAX {
        return Err(Error::value("MIDI file exceeds 16 MiB"));
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(4)? != b"MThd" {
        return Err(Error::value("MThd not found. Probably not a MIDI file"));
    }
    let size = r.long()?;
    if size < 6 {
        return Err(Error::value("invalid MIDI header length"));
    }
    let kind = r.word()?;
    let count = r.word()?;
    let ticks = r.word()? as i16;
    r.take(size - 6)?;
    if kind > 2 {
        return Err(Error::value("unsupported MIDI type or time division"));
    }
    let mut tracks = Vec::new();
    for _ in 0..count {
        if r.take(4)? != b"MTrk" {
            return Err(Error::value("no MTrk header at start of track"));
        }
        let size = r.long()?;
        let mut t = Reader {
            bytes: r.take(size)?,
            at: 0,
        };
        let mut track = Track::default();
        let mut tick = 0u64;
        let mut last = None;
        while t.at < t.bytes.len() {
            tick = tick
                .checked_add(t.variable()?)
                .ok_or_else(|| Error::value("MIDI tick overflow"))?;
            let b = t.byte()?;
            let (status, first) = if b < 128 {
                (
                    last.ok_or_else(|| Error::value("running status without last_status"))?,
                    Some(b),
                )
            } else {
                if b != 255 {
                    last = Some(b);
                }
                (b, None)
            };
            if status == 255 {
                let ty = t.byte()?;
                let n = t.variable()?;
                let data =
                    t.take(usize::try_from(n).map_err(|_| Error::value("invalid MIDI length"))?)?;
                if ty == 3 && !track.named {
                    track.name = data.iter().map(|b| char::from(*b)).collect();
                    track.named = true;
                }
                if ty == 0x58 {
                    if data.len() != 4 {
                        return Err(Error::value("invalid time signature"));
                    }
                    track.non_four |= data[0] != 4 || data[1] != 2;
                }
            } else if [240, 247].contains(&status) {
                let n = t.variable()?;
                t.take(usize::try_from(n).map_err(|_| Error::value("invalid MIDI length"))?)?;
            } else {
                let n = match status {
                    128..=191 | 224..=239 => 2,
                    192..=223 | 241 | 243 => 1,
                    242 => 2,
                    246 | 248 | 250 | 251 | 252 | 254 | 255 => 0,
                    _ => return Err(Error::value("undefined MIDI status")),
                };
                let mut data = Vec::with_capacity(n);
                if let Some(b) = first {
                    data.push(b);
                }
                if data.len() > n {
                    return Err(Error::value("invalid MIDI running status"));
                }
                data.extend(t.take(n - data.len())?);
                if data.iter().any(|b| *b >= 128) {
                    return Err(Error::value("data byte must be in range 0..127"));
                }
                if status < 240 {
                    track.channels.insert(status & 15);
                }
                if status & 0xf0 == 0x90 && data[1] > 0 {
                    track.notes.push(Note {
                        tick,
                        note: data[0],
                        velocity: data[1],
                    });
                    if track.notes.len() > 1_000_000 {
                        return Err(Error::value("too many MIDI note events"));
                    }
                }
            }
        }
        tracks.push(track);
    }
    Ok(Smf {
        kind,
        ticks,
        tracks,
    })
}
pub fn inspect_midi(bytes: &[u8]) -> Result<Value> {
    let mf = parse(bytes)?;
    Ok(
        json!({"ticks_per_beat":mf.ticks,"tracks":mf.tracks.iter().enumerate().map(|(i,t)|json!({"track":i,"name":t.name,"notes":t.notes.len(),"range":if t.notes.is_empty(){vec![]}else{vec![t.notes.iter().map(|n|n.note).min().unwrap(),t.notes.iter().map(|n|n.note).max().unwrap()]},"channels":t.channels})).collect::<Vec<_>>()}),
    )
}
#[derive(Clone, Copy)]
pub struct Selection<'a> {
    pub track: i64,
    pub tonic: u8,
    pub bar: u32,
    pub role: &'a str,
    pub minor: bool,
    pub name: &'a str,
}
pub(crate) fn round_even(n: u64, d: u64) -> u64 {
    let (q, r) = (n / d, n % d);
    q + u64::from(r > d - r || (r == d - r && q & 1 != 0))
}
pub fn import_midi(
    bytes: &[u8],
    base: &Value,
    s: Selection<'_>,
    limits: Limits<'_>,
) -> Result<Value> {
    let mut d = decode(&compile(base, limits)?, limits)?.into_value();
    let mf = parse(bytes)?;
    if mf.kind == 2 {
        return Err(Error::value(
            "type 2 MIDI has independent sequences; export the selected sequence first",
        ));
    }
    let track = usize::try_from(s.track)
        .ok()
        .and_then(|i| mf.tracks.get(i))
        .ok_or_else(|| Error::value("track out of range"))?;
    if mf.tracks.iter().any(|t| t.non_four) {
        return Err(Error::value("import currently requires 4/4 MIDI"));
    }
    if s.bar == 0 {
        return Err(Error::value("bar is one-based"));
    }
    // The old inspector exposes signed SMPTE/zero divisions. Its selected-bar
    // importer finds no events for these divisions rather than treating them as
    // positive PPQ. Preserve both results without unsigned clock conversion.
    if mf.ticks <= 0 {
        return Err(Error::value("no notes in selected track/bar"));
    }
    let start = (s.bar as u64 - 1) * 4 * mf.ticks as u64;
    let end = start + 4 * mf.ticks as u64;
    let events: Vec<_> = track
        .notes
        .iter()
        .filter(|n| n.tick >= start && n.tick < end)
        .collect();
    if events.is_empty() {
        return Err(Error::value("no notes in selected track/bar"));
    }
    if s.role == "bass" {
        let bass = d["bass"].as_array_mut().unwrap();
        if bass.len() >= 8 {
            return Err(Error::value("all eight bass pattern slots are used"));
        }
        let mut notes = [-128; 16];
        let mut velocities = [0; 16];
        for n in &events {
            let step = round_even((n.tick - start) * 4, mf.ticks as u64).min(15) as usize;
            let iv = (n.note as i32 - s.tonic as i32 + 6).rem_euclid(12) - 6;
            if notes[step] != -128 && notes[step] != iv {
                return Err(Error::value(
                    "selected bass track is polyphonic at the sixteenth grid",
                ));
            }
            notes[step] = iv;
            velocities[step] = n.velocity;
        }
        bass.push(json!({"notes":notes,"velocity":velocities,"gate":58}));
    } else if s.role == "harmony" {
        if d["version"] == 2 {
            return Err(Error::value("v2 harmony uses authored charts; inspect the MIDI and edit chart destinations explicitly"));
        }
        // Keep insertion order: selection adds the first new group first.
        let mut groups: Vec<(u64, BTreeSet<u8>)> = Vec::new();
        for n in &events {
            let slot = (n.tick - start) / (mf.ticks as u64 / 32).max(1);
            if let Some((_, pcs)) = groups.iter_mut().find(|(k, _)| *k == slot) {
                pcs.insert(n.note % 12);
            } else {
                groups.push((slot, BTreeSet::from([n.note % 12])));
            }
        }
        let mut added = 0;
        for (_, pcs) in groups {
            if pcs.len() != 4 {
                continue;
            }
            let mut found = None;
            for root in 0..12 {
                for (scale, shape) in [[0, 4, 7, 9], [0, 3, 7, 9]].iter().enumerate() {
                    let relative: BTreeSet<_> = pcs
                        .iter()
                        .map(|n| (*n as i32 - root).rem_euclid(12))
                        .collect();
                    if relative == BTreeSet::from(*shape) {
                        found = Some((root, scale));
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            let Some((root, scale)) = found else {
                continue;
            };
            let rel = (root - s.tonic as i32).rem_euclid(12);
            if d["nodes"].as_array().unwrap().iter().any(|n| {
                n["bass"] == rel
                    && n["sixth"] == rel
                    && n["scale"] == scale
                    && n["mode"] == u8::from(s.minor)
            }) {
                continue;
            }
            if d["nodes"].as_array().unwrap().len() >= 16 {
                break;
            }
            d["nodes"].as_array_mut().unwrap().push(json!({"bass":rel,"sixth":rel,"scale":scale,"label":if scale==1{4}else{0},"mode":u8::from(s.minor)}));
            let size = d["nodes"].as_array().unwrap().len();
            for row in d["transitions"].as_array_mut().unwrap() {
                row.as_array_mut().unwrap().push(json!(0));
            }
            let home = d["cadence"][usize::from(s.minor)][0].as_u64().unwrap() as usize;
            let mut row = vec![0; size];
            row[home] = 8;
            d["transitions"].as_array_mut().unwrap().push(json!(row));
            d["transitions"][home][size - 1] = json!(3);
            added += 1;
        }
        if added == 0 {
            return Err(Error::value(
                "no new major/minor sixth voicings found; inspect the MIDI or choose another bar",
            ));
        }
    } else {
        return Err(Error::value("MIDI role must be bass or harmony"));
    }
    crate::append_source(
        &mut d["sources"],
        json!(format!(
            "User-selected MIDI reference: {}, track {}, bar {}, role {}; relative to tonic {}.",
            s.name, s.track, s.bar, s.role, s.tonic
        )),
    )?;
    Ok(d)
}
fn variable(mut n: u64, out: &mut Vec<u8>) {
    let mut digits = [0u8; 10];
    let mut at = 9;
    digits[at] = (n & 127) as u8;
    while {
        n >>= 7;
        n != 0
    } {
        at -= 1;
        digits[at] = ((n & 127) as u8) | 128;
    }
    out.extend(&digits[at..]);
}
fn chunk(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend(b"MTrk");
    out.extend((bytes.len() as u32).to_be_bytes());
    out.extend(bytes);
}
/// Equivalent SMF1 sink, including channel insertion order and running status.
pub fn midi_from_log(log: &str, bpm: u32) -> Result<Vec<u8>> {
    if bpm == 0 {
        return Err(Error::value("BPM must be positive"));
    }
    let tempo = round_even(60_000_000, bpm as u64);
    if tempo == 0 || tempo > 0xffffff {
        return Err(Error::value("tempo outside MIDI range"));
    }
    let mut tracks: Vec<(u8, u64, u8, Vec<u8>)> = Vec::new();
    let mut index = HashMap::new();
    for line in log.lines() {
        let Some((ms, data)) = line.split_once(" ms: ") else {
            continue;
        };
        if ms.is_empty() || !ms.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let d = data.as_bytes();
        if d.len() < 8
            || d[2] != b' '
            || d[5] != b' '
            || [0, 1, 3, 4, 6, 7]
                .iter()
                .any(|i| !matches!(d[*i],b'0'..=b'9'|b'A'..=b'F'))
        {
            continue;
        }
        let ms = ms
            .parse::<u64>()
            .map_err(|_| Error::value("MIDI log time overflow"))?;
        let st = u8::from_str_radix(&data[..2], 16).unwrap();
        let a = u8::from_str_radix(&data[3..5], 16).unwrap();
        let b = u8::from_str_radix(&data[6..8], 16).unwrap();
        if st >= 240 {
            continue;
        }
        if st < 128 || a >= 128 || (b >= 128 && ![0xc0, 0xd0].contains(&(st & 0xf0))) {
            return Err(Error::value("invalid MIDI log message"));
        }
        let ch = st & 15;
        let i = *index.entry(ch).or_insert_with(|| {
            let name = match ch {
                0 => "Keys".into(),
                2 => "Bass".into(),
                4 => "Swells".into(),
                9 => "Drums".into(),
                _ => format!("Channel {}", ch + 1),
            };
            let mut bytes = vec![0, 255, 3];
            variable(name.len() as u64, &mut bytes);
            bytes.extend(name.as_bytes());
            tracks.push((ch, 0, 0, bytes));
            tracks.len() - 1
        });
        let (_, last, status, bytes) = &mut tracks[i];
        let tick = round_even(
            ms.checked_mul(480000)
                .ok_or_else(|| Error::value("MIDI log time overflow"))?,
            tempo,
        );
        if tick < *last {
            return Err(Error::value(
                "message time must be non-negative in MIDI file",
            ));
        }
        variable(tick - *last, bytes);
        *last = tick;
        if *status != st {
            bytes.push(st);
            *status = st;
        }
        bytes.push(a);
        if ![0xc0, 0xd0].contains(&(st & 0xf0)) {
            bytes.push(b);
        }
    }
    let mut out = b"MThd\0\0\0\x06\0\x01".to_vec();
    out.extend((tracks.len() as u16 + 1).to_be_bytes());
    out.extend(480u16.to_be_bytes());
    let meta = vec![
        0,
        255,
        81,
        3,
        (tempo >> 16) as u8,
        (tempo >> 8) as u8,
        tempo as u8,
        0,
        255,
        47,
        0,
    ];
    chunk(&meta, &mut out);
    for (_, _, _, mut t) in tracks {
        t.extend([0, 255, 47, 0]);
        chunk(&t, &mut out);
    }
    Ok(out)
}
