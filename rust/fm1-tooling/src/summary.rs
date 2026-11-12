// SPDX-License-Identifier: GPL-3.0-only
//! Telemetry aggregation preserves input order, stable peak ties and the legacy
//! report equations. File parsing and printing do not choose report policy.
use crate::{Error, Result};
use serde_json::Value;
use std::collections::BTreeMap;
/// Bounded UTF-8 CSV reader for the native offline command. Quoted newlines,
/// doubled quotes, empty cells and CRLF retain their CSV meanings.
pub fn csv(data: &[u8]) -> Result<Vec<Value>> {
    if data.len() > 32 * 1024 * 1024 {
        return Err(Error::value("telemetry CSV exceeds32 MiB"));
    }
    let source =
        std::str::from_utf8(data).map_err(|_| Error::value("telemetry CSV must be UTF-8"))?;
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted || (c != '\n' && c != '\r') {
            started = true;
        }
        match c {
            '"' if quoted => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"')
                } else {
                    quoted = false
                }
            }
            '"' if cell.is_empty() => quoted = true,
            ',' if !quoted => {
                row.push(std::mem::take(&mut cell));
            }
            '\n' | '\r' if !quoted => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut cell));
                if started || row.len() != 1 || !row[0].is_empty() {
                    records.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
                started = false;
            }
            c => cell.push(c),
        }
        if cell.len() > 1024 * 1024 || row.len() > 256 || records.len() > 100_001 {
            return Err(Error::value("telemetry CSV exceeds bounded rows/cells"));
        }
    }
    if quoted {
        return Err(Error::value("unterminated CSV quote"));
    }
    if started || !row.is_empty() || !cell.is_empty() {
        row.push(cell);
        records.push(row);
    }
    let Some(header) = records.first() else {
        return Ok(Vec::new());
    };
    if header.len() > 256 {
        return Err(Error::value("too many CSV columns"));
    }
    let mut rows = Vec::new();
    for record in records.iter().skip(1) {
        if record.len() != header.len() {
            return Err(Error::value("CSV row does not match header"));
        }
        let mut row = serde_json::Map::new();
        for (k, v) in header.iter().zip(record) {
            row.insert(k.clone(), Value::String(v.clone()));
        }
        rows.push(Value::Object(row));
    }
    Ok(rows)
}
pub const GENRES: [&str; 19] = [
    "HYPNO",
    "LOFI",
    "HOUSE",
    "DUBSTEP",
    "DUB",
    "DNB",
    "BREAKS",
    "GARAGE",
    "ELECTRO",
    "TRAP",
    "DISCO",
    "FUNK",
    "REGGAE",
    "SYNTHWAVE",
    "AMAPIANO",
    "TRIPHOP",
    "INDUS",
    "PSY",
    "FOREST",
];
pub const KEYS: [&str; 9] = [
    "clear", "bassacid", "pluck", "w_pad", "merge", "drums", "echo", "w_rev", "master",
];
const NAMES: [&str; 9] = [
    "clear",
    "bass+acid",
    "pluck+DX+snips",
    "wait pad",
    "merge+sidechain",
    "drums",
    "echoes",
    "wait reverb",
    "master",
];
const PIECES: [&str; 14] = [
    "bass", "acid", "cm", "fm", "dj", "noise", "padmix", "side", "drums", "bassmix", "echoes",
    "fxm", "dyn", "clip",
];
fn n(r: &Value, k: &str) -> i64 {
    r[k].as_i64()
        .or_else(|| r[k].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}
fn f(r: &Value, k: &str) -> f64 {
    r[k].as_f64()
        .or_else(|| r[k].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}
fn genre(g: i64) -> String {
    let i = if g < 0 { GENRES.len() as i64 + g } else { g };
    GENRES
        .get(i as usize)
        .map_or_else(|| g.to_string(), |s| s.to_string())
}
fn sum(rows: &[&Value], key: &str) -> i64 {
    rows.iter().map(|r| n(r, key)).sum()
}
fn groups(rows: &[Value]) -> BTreeMap<(i64, i64), Vec<&Value>> {
    let mut g = BTreeMap::new();
    for r in rows {
        g.entry((n(r, "genre"), n(r, "piece")))
            .or_insert_with(Vec::new)
            .push(r)
    }
    g
}
fn present(rows: &[Value], key: &str) -> bool {
    rows.first().is_some_and(|r| r.get(key).is_some())
}
pub fn report(section: &str, rows: &[Value]) -> Result<String> {
    if rows.len() > 100_000 {
        return Err(Error::value("telemetry report exceeds100000 rows"));
    }
    // Live words are u32. Permit signed accumulated/reset controls, with a
    // wider explicit bound that keeps every100000-row sum/delta inside i64.
    for row in rows {
        for key in crate::telemetry::FIELDS
            .iter()
            .chain(&crate::telemetry::EXTRA)
            .chain(&crate::telemetry::DIAG)
            .copied()
            .chain(["chorus", "bass", "flash", "susp_bad", "boot"])
        {
            if let Some(value) = row.get(key) {
                let value = value
                    .as_i64()
                    .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
                    .ok_or_else(|| Error::value(format!("invalid telemetry integer: {key}")))?;
                if !(-(1i64 << 40)..=(1i64 << 40)).contains(&value) {
                    return Err(Error::value(format!(
                        "telemetry integer outside supported bound: {key}"
                    )));
                }
            }
        }
        if let Some(value) = row.get("t") {
            let value = value
                .as_f64()
                .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
                .ok_or_else(|| Error::value("invalid telemetry time"))?;
            if !value.is_finite() {
                return Err(Error::value("telemetry time must be finite"));
            }
        }
    }
    let mut out = String::new();
    match section {
        "main" => {
            if rows.is_empty() {
                return Err(Error {
                    kind: "SystemExit".into(),
                    code: String::new(),
                    message: "empty".into(),
                });
            }
            out+="xr = halves never rendered (a dropout), late = renders that ran past their half (nothing lost: the next one starts at once)\n";
            let stages = KEYS
                .iter()
                .map(|s| format!("{s:>7}"))
                .collect::<Vec<_>>()
                .join(" ");
            out+=&format!("{:<9} {:>2} {:>5} {:>4} {:>4} {:>3} {:>4} {:>6} {:>6} | us per 5.8 ms chunk: {stages}  isr% chor bass\n","genre","pc","secs","cpu","max","xr","late","peak","gap");
            let mut deltas: BTreeMap<(i64, i64), (i64, i64)> = BTreeMap::new();
            let mut prev = None;
            for r in rows {
                let delta = prev.map_or((0, 0), |p| {
                    (n(r, "xruns") - n(p, "xruns"), n(r, "late") - n(p, "late"))
                });
                let entry = deltas.entry((n(r, "genre"), n(r, "piece"))).or_default();
                entry.0 += delta.0;
                entry.1 += delta.1;
                prev = Some(r);
            }
            for ((ge, pc), s) in groups(rows) {
                let ch = sum(&s, "chunks");
                let ch = if ch == 0 { 1 } else { ch };
                let count = s.len() as f64;
                let max = |key: &str| s.iter().map(|r| n(r, key)).max().unwrap();
                let st = KEYS
                    .iter()
                    .map(|k| {
                        let k = if *k == "w_pad" || *k == "w_rev" {
                            k.to_string()
                        } else {
                            format!("p_{k}")
                        };
                        format!("{:7.0}", sum(&s, &k) as f64 / ch as f64)
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let (dx, dl) = deltas[&(ge, pc)];
                let chorus = s.iter().filter(|r| n(r, "chorus") != 0).count() as f64;
                let bass = s.iter().filter(|r| n(r, "bass") != 0).count() as f64;
                out+=&format!("{:<9} {pc:2} {:5.0} {:3.0}% {:3}% {dx:3} {dl:4} {:5.1}ms {:5.1}ms | {st}  {:4.1}% {:3}% {:3}%\n",genre(ge),count*0.25,sum(&s,"cpu")as f64/count,max("cpu"),max("max_us")as f64/1000.0,max("gap_us")as f64/1000.0,100.0*sum(&s,"isr_in")as f64/(count*250000.0),(100.0*chorus/count)as i64,(100.0*bass/count)as i64);
            }
        }
        "peaks" if present(rows, "pk_total") => {
            out+="\nthe slowest chunks (a chunk is at most 256 frames = 5.8 ms; the two of a render must fit 11.6 ms):\n";
            let mut ranked: Vec<_> = rows.iter().filter(|r| n(r, "pk_total") != 0).collect();
            ranked.sort_by_key(|r| -n(r, "pk_total"));
            for r in ranked.into_iter().take(8) {
                let st: Vec<_> = KEYS.iter().map(|k| n(r, &format!("pk_{k}"))).collect();
                let info = n(r, "pk_info");
                let top = (0..9)
                    .max_by_key(|i| (if *i == 3 || *i == 7 { -1 } else { st[*i] }, -(*i as i64)))
                    .unwrap();
                out+=&format!("  {:5.1} s  {} {:2}  {:5} us ({} frames) step {:2}{}  biggest {} {} us  waits {}+{}  {}{}shed {}\n",f(r,"t"),genre(n(r,"genre")),n(r,"piece"),n(r,"pk_total"),(info>>4)&255,info&15,if info>>12&1!=0{" (just after a step)"}else{""},NAMES[top],st[top],st[3],st[7],if info>>13&1!=0{"drone+acid on core 2 "}else{""},if info>>14&1!=0{"chorus on core 1 "}else{""},info>>15&3);
            }
            let refs: Vec<_> = rows.iter().collect();
            let hist: Vec<_> = ["lt5", "5", "6", "7", "8", "9", "10", "11", "ge12"]
                .iter()
                .map(|k| sum(&refs, &format!("h_{k}")))
                .collect();
            let tot = hist.iter().sum::<i64>();
            let tot = if tot == 0 { 1 } else { tot };
            out += &format!(
                "renders by time (ms): {}\n",
                ["<5", "5", "6", "7", "8", "9", "10", "11", ">=12"]
                    .iter()
                    .zip(hist)
                    .map(|(label, h)| format!("{label} {:.1}%", 100.0 * h as f64 / tot as f64))
                    .collect::<Vec<_>>()
                    .join("  ")
            );
        }
        "halves" if present(rows, "pkr_us") => {
            let mut ranked: Vec<_> = rows.iter().filter(|r| n(r, "pkr_us") != 0).collect();
            ranked.sort_by_key(|r| -n(r, "pkr_us"));
            if ranked.is_empty() {
                return Ok(out);
            }
            out+="\nthe slowest halves (budget 11.6 ms): total = its chunks' stages + the timer interrupt inside it + the rest (drain, control, pulse, swung hits):\n";
            for r in ranked.into_iter().take(12) {
                let (us, st, isr, wt) = (
                    n(r, "pkr_us"),
                    n(r, "pkr_stages"),
                    n(r, "pkr_isr"),
                    n(r, "pkr_waits"),
                );
                let wi = n(r, "pkr_where");
                let place = (wi >> 8) & 15;
                let place = match place {
                    0 => "between turns".into(),
                    1 => "UI frame".into(),
                    2 => "USB services".into(),
                    p => p.to_string(),
                };
                out+=&format!("  {:5.1} s  {} {:2}  {us:5} us = stages {st:5} (waits {wt:4}) + isr {isr:4} ({:2} ticks) + rest {:5}  chunks {}  step {:2}  half {}  main loop: {place}\n",f(r,"t"),genre(n(r,"genre")),n(r,"piece"),n(r,"pkr_nested"),us-st-isr,n(r,"pkr_chunks"),wi&255,wi>>12&1);
            }
        }
        "fixed" | "diag" if present(rows, "sm_n") => {
            if section == "diag" && !present(rows, "d_bass") {
                return Ok(out);
            }
            let mut shown = false;
            for ((ge, pc), s) in groups(rows) {
                let (ns, fs, na) = (sum(&s, "sm_n"), sum(&s, "sm_frames"), sum(&s, "chunks"));
                let nf = na - ns;
                let solve = ns >= 200 && nf >= 200 && fs != 256 * ns;
                if (section == "fixed" && !solve) || (section == "diag" && na < 400) {
                    continue;
                }
                if !shown {
                    out += if section == "fixed" {
                        "\nfixed cost a chunk by stage (us; F + n v solved from the full and the short chunks), and a 256-frame chunk's total:\n"
                    } else {
                        "\nthe pieces inside the stages (us per full chunk; F = the fixed cost a chunk where the log allows it), the chunk sizes, the work between chunks:\n"
                    };
                    shown = true;
                }
                let keys: &[&str] = if section == "fixed" { &KEYS } else { &PIECES };
                let mut columns = Vec::new();
                let (mut total_fixed, mut total_full) = (0.0, 0.0);
                for (i, k) in keys.iter().enumerate() {
                    let all = if section == "diag" {
                        format!("d_{k}")
                    } else if *k == "w_pad" || *k == "w_rev" {
                        k.to_string()
                    } else {
                        format!("p_{k}")
                    };
                    let short = format!("{}_{k}", if section == "diag" { "ds" } else { "sm" });
                    let sa = sum(&s, &all);
                    let ss = sum(&s, &short);
                    let full = (sa - ss) as f64;
                    if solve {
                        let v = (ss as f64 - ns as f64 * full / nf as f64)
                            / (fs as f64 - 256.0 * ns as f64);
                        let fixed = full / nf as f64 - 256.0 * v;
                        total_fixed += fixed;
                        total_full += full / nf as f64;
                        if section == "fixed" {
                            let names = [
                                "clear",
                                "bass+acid",
                                "pluck+DX",
                                "wait pad",
                                "merge",
                                "drums",
                                "echoes",
                                "wait rev",
                                "master",
                            ];
                            columns.push(format!(
                                "{} F {fixed:4.0} (of {:4.0})",
                                names[i],
                                full / nf as f64
                            ));
                        } else {
                            columns.push(format!("{k} {:4.0} F {fixed:4.0}", full / nf as f64));
                        }
                    } else {
                        columns.push(format!("{k} {:4.0}", sa as f64 / na as f64));
                    }
                }
                if section == "fixed" {
                    out+=&format!("  {} {pc:2}  ({ns} short chunks, mean {:.0} frames; {nf} full):  {}  | fixed {total_fixed:.0} of {total_full:.0} us a full chunk\n",genre(ge),fs as f64/ns as f64,columns.join("  "));
                } else {
                    out += &format!(
                        "  {} {pc:2}  ({na} chunks, {ns} short):  {}{}\n",
                        genre(ge),
                        columns.join("  "),
                        if solve {
                            format!("  | fixed in these pieces {total_fixed:.0} us")
                        } else {
                            String::new()
                        }
                    );
                    let hc: Vec<_> = ["lt64", "lt128", "lt256", "256"]
                        .iter()
                        .map(|k| sum(&s, &format!("hc_{k}")))
                        .collect();
                    let r: Vec<_> = ["drain", "control", "pulse", "hits"]
                        .iter()
                        .map(|k| sum(&s, &format!("r_{k}")) as f64 / na as f64)
                        .collect();
                    out+=&format!("           chunks by size: <64 {}  <128 {}  <256 {}  256 {}   | between chunks, us a chunk: drain {:.0} control {:.0} pulse {:.0} hits {:.0}\n",hc[0],hc[1],hc[2],hc[3],r[0],r[1],r[2],r[3]);
                }
            }
        }
        "peaks" | "halves" | "fixed" | "diag" => {}
        _ => return Err(Error::value("unknown telemetry report section")),
    }
    Ok(out)
}
