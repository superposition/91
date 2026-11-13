// SPDX-License-Identifier: GPL-3.0-only
//! Connected acceptance policy over copied facts. This module never opens a
//! MIDI/audio device or observes wall time: adapters supply those observations.
use crate::{get, integer, list, string, Error, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};
fn assertion(message: &str) -> Error {
    Error {
        kind: "AssertionError",
        message: message.into(),
    }
}
fn number(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error::value("expected finite numeric observation"))
}
fn control(field: &str, value: Value) -> Value {
    json!({"kind":"control","field":field,"value":value})
}
pub fn initial(features: &Value) -> Result<Value> {
    if features["factory_songs"] != 3 {
        return Err(assertion(""));
    }
    Ok(json!([
        control("play", json!(0)),
        control("hold", json!(0)),
        control("bars", json!(16)),
        control("form", json!(0)),
        control("song", json!(0))
    ]))
}
/// Polling phases preserve the original short-circuit request order: version,
/// then a fresh loading read, then pending. The adapter performs only the
/// requested read/sleep and passes the result back without retaining a borrow.
pub fn ready(v: &Value) -> Result<Value> {
    let phase = integer(get(v, "phase")?, 0, 2, "readiness phase")?;
    let reply = get(v, "reply")?;
    let pending = match phase {
        0 => reply["pack_version"] != 2 && reply["pack_version"] != 3,
        1 => reply["loading"]
            .as_bool()
            .unwrap_or_else(|| reply["loading"].as_i64().unwrap_or(0) != 0),
        _ => reply["pending"]
            .as_bool()
            .unwrap_or_else(|| reply["pending"].as_i64().unwrap_or(0) != 0),
    };
    if pending {
        if number(get(v, "now")?)? > number(get(v, "deadline")?)? {
            return Err(Error {
                kind: "TimeoutError",
                message: "factory song activation".into(),
            });
        }
        Ok(json!({"kind":"sleep","phase":0}))
    } else if phase < 2 {
        Ok(json!({"kind":"next","phase":phase+1}))
    } else {
        Ok(json!({"kind":"ready","phase":0}))
    }
}
const EVENTS: [(u32, &str, i32); 10] = [
    (180, "song", 1),
    (360, "song", 2),
    (540, "touch", 60),
    (542, "brightness", 45),
    (544, "keys_program", 0),
    (546, "bass_program", 3),
    (548, "swell", 40),
    (550, "hold", 1),
    (560, "hold", 0),
    (562, "idea", 1),
];
pub fn step(v: &Value) -> Result<Value> {
    if v["stop"].as_bool() == Some(true) {
        return Err(Error {
            kind: "InterruptedError",
            message: "acceptance stopped by stop file".into(),
        });
    }
    let elapsed = number(get(v, "elapsed")?)?;
    let now = number(get(v, "now")?)?;
    let state = get(v, "state")?;
    let mut armed = state["armed"].as_f64().unwrap_or(0.0);
    let mut done = state["done"].as_u64().unwrap_or(0) as u16;
    let mut uploaded = state["uploaded"].as_bool().unwrap_or(false);
    let block = (elapsed / 30.0).floor() as i64;
    let printed = state["printed"].as_i64().unwrap_or(-1);
    let mut actions = Vec::new();
    if now - armed > 3.0 {
        actions.push(json!({"kind":"arm"}));
        armed = now;
    }
    for (index, (at, field, value)) in EVENTS.iter().enumerate() {
        if elapsed >= *at as f64 && done & (1 << index) == 0 {
            let mut a = control(field, json!(value));
            a["seconds"] = json!(elapsed);
            actions.push(a);
            done |= 1 << index;
        }
    }
    if elapsed >= 570.0 && !uploaded {
        actions.push(json!({"kind":"upload","seconds":elapsed}));
        uploaded = true;
    }
    Ok(
        json!({"state":{"armed":armed,"done":done,"uploaded":uploaded,"printed":block},"actions":actions,"print":block!=printed,"printed_seconds":elapsed.round_ties_even()as i64}),
    )
}
pub fn cleanup(before: &Value) -> Result<Value> {
    Ok(json!([
        json!({"kind":"request","command":13,"data":[0]}),
        control("play", json!(0)),
        control("bars", get(before, "bars")?.clone()),
        control(
            "key",
            json!(
                integer(get(before, "key")?, 0, 11, "key")?
                    + 12 * integer(get(before, "minor")?, 0, 1, "minor")?
            )
        ),
        control("hold", get(before, "hold")?.clone()),
        control("energy", get(before, "energy")?.clone()),
        control("song", json!(0))
    ]))
}
pub fn validate(v: &Value) -> Result<()> {
    let before = get(v, "before")?;
    let after = get(v, "after")?;
    let telemetry = list(get(v, "telemetry")?)?;
    let trace = list(get(v, "trace")?)?;
    if after["xruns"] != before["xruns"] {
        return Err(assertion("audio xruns increased"));
    }
    if telemetry.is_empty() {
        return Err(assertion("no hardware telemetry"));
    }
    let first = &telemetry[0];
    let last = telemetry.last().unwrap();
    for field in ["xruns", "late", "crashes"] {
        if last[field] != first[field] {
            return Err(assertion(&format!("{field} increased")));
        }
    }
    if telemetry
        .iter()
        .filter_map(|t| t["shed"].as_i64())
        .max()
        .unwrap_or(0)
        != 0
    {
        return Err(assertion("load shedding"));
    }
    if !list(get(v, "flags")?)?.is_empty() {
        return Err(assertion("USB recording callback flags"));
    }
    let crcs: BTreeSet<_> = trace.iter().map(|s| s["crc32"].to_string()).collect();
    if crcs.len() < 3 {
        return Err(assertion("not all songs activated"));
    }
    if trace
        .iter()
        .any(|s| s["key"] != before["key"] || s["minor"] != before["minor"])
    {
        return Err(assertion("home key changed"));
    }
    Ok(())
}
// NumPy's float64 reduction tree: eight independent accumulators within a
// 128-value block and aligned pairwise recursion outside it. Squared PCM16
// values are individually exact; retaining this tree preserves legacy reports.
fn squares(bytes: &[u8]) -> f64 {
    let n = bytes.len() / 2;
    let sample = |i: usize| {
        let v = i16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]) as f64;
        v * v
    };
    if n < 8 {
        return (0..n).fold(-0.0, |sum, i| sum + sample(i));
    }
    if n <= 128 {
        let mut sums = [0.0; 8];
        for (i, s) in sums.iter_mut().enumerate() {
            *s = sample(i);
        }
        let end = n - n % 8;
        let mut i = 8;
        while i < end {
            for (j, s) in sums.iter_mut().enumerate() {
                *s += sample(i + j);
            }
            i += 8;
        }
        let mut sum = ((sums[0] + sums[1]) + (sums[2] + sums[3]))
            + ((sums[4] + sums[5]) + (sums[6] + sums[7]));
        for i in end..n {
            sum += sample(i);
        }
        sum
    } else {
        let middle = (n / 2) - (n / 2) % 8;
        squares(&bytes[..middle * 2]) + squares(&bytes[middle * 2..])
    }
}
pub fn finalize(v: &Value) -> Result<Value> {
    let folder = Path::new(string(get(v, "folder")?)?);
    let path = string(get(v, "pcm")?)?;
    if fs::metadata(path)?.len() > 1024 * 1024 * 1024 {
        return Err(Error::value(
            "acceptance recording must be bounded stereo PCM16",
        ));
    }
    let bytes = fs::read(path)?;
    if bytes.len() % 4 != 0 {
        return Err(Error::value(
            "acceptance recording must be bounded stereo PCM16",
        ));
    }
    fs::create_dir_all(folder)?;
    let mut wav = fm1_sim::sinks::Wav::open(&folder.join("device-songbook.wav"))?;
    for part in bytes.chunks(65536) {
        let samples: Vec<_> = part
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        wav.append(&samples)?;
    }
    wav.finish()?;
    let peak = bytes
        .chunks_exact(2)
        .map(|b| (i16::from_le_bytes([b[0], b[1]]) as i32).abs())
        .max()
        .unwrap_or(0);
    let samples = bytes.len() / 2;
    let rms = if samples == 0 {
        0.0
    } else {
        (squares(&bytes) / samples as f64).sqrt()
    };
    let telemetry = list(get(v, "telemetry")?)?;
    let mut report = json!({"before":get(v,"before")?,"after":get(v,"after")?,"seconds":bytes.len()as f64/4.0/44100.0,"errors":get(v,"errors")?,"capture_flags":get(v,"flags")?,"peak_pcm16":peak,"rms_pcm16":rms,"telemetry_frames":telemetry.len(),"events":get(v,"events")?});
    if !telemetry.is_empty() {
        for (dest, source) in [
            ("cpu_max", "cpu"),
            ("cpu1_max", "cpu1"),
            ("render_max_us", "max_us"),
            ("shed", "shed"),
        ] {
            let max = telemetry
                .iter()
                .map(|t| integer(&t[source], i64::MIN, i64::MAX, source))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .max()
                .unwrap();
            report[dest] = json!(max);
        }
        let mut deltas = json!({});
        for key in ["xruns", "late", "crashes"] {
            deltas[key] = json!(
                integer(&telemetry.last().unwrap()[key], i64::MIN, i64::MAX, key)?
                    - integer(&telemetry[0][key], i64::MIN, i64::MAX, key)?
            );
        }
        report["deltas"] = deltas;
    }
    for (name, value) in [
        ("report", &report),
        ("telemetry", get(v, "telemetry")?),
        ("status", get(v, "trace")?),
        ("extra", get(v, "extra")?),
        ("diag", get(v, "diag")?),
    ] {
        fs::write(
            folder.join(format!("{name}.json")),
            serde_json::to_vec_pretty(value)?,
        )?;
    }
    Ok(report)
}
