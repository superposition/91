// SPDX-License-Identifier: GPL-3.0-only
//! DY asset selection, refusal and byte generation. No port/device access.
use crate::{bytes, text, Adapter, Error, Result};
use fm1_formats::voice_file::BulkBanks;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
const MAX_LINES: usize = 4096;
pub fn init() -> [u8; 128] {
    serde_json::from_str::<Vec<u8>>(include_str!("../../../../tools/data/dy-init.json"))
        .expect("checked DY init")
        .try_into()
        .expect("128-byte DY init")
}
pub fn name(record: &[u8]) -> String {
    record
        .get(118..record.len().min(128))
        .unwrap_or_default()
        .iter()
        .map(|b| {
            if (32..127).contains(b) {
                char::from(*b)
            } else {
                '?'
            }
        })
        .collect::<String>()
        .trim_end_matches(' ')
        .to_owned()
}
fn error(kind: &str, message: impl Into<String>) -> Error {
    Error {
        kind: kind.into(),
        code: String::new(),
        message: message.into(),
    }
}
fn index_error() -> Error {
    error("IndexError", "list index out of range")
}
fn integer(s: &str) -> Result<i64> {
    let invalid = || {
        Error::value(format!(
            "invalid literal for int() with base 10: {}",
            crate::installer::repr(s)
        ))
    };
    let t = s.trim();
    let digits = t
        .strip_prefix('+')
        .or_else(|| t.strip_prefix('-'))
        .unwrap_or(t);
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
        || !digits.bytes().all(|b| b.is_ascii_digit() || b == b'_')
    {
        return Err(invalid());
    }
    t.replace('_', "")
        .parse::<i64>()
        .ok()
        .filter(|n| n.unsigned_abs() <= 1 << 40)
        .ok_or_else(invalid)
}
fn select<T>(values: &[T], i: i64) -> Result<&T> {
    let i = if i < 0 { values.len() as i64 + i } else { i };
    usize::try_from(i)
        .ok()
        .and_then(|i| values.get(i))
        .ok_or_else(index_error)
}
pub fn join(base: &str, parts: &[&str]) -> String {
    let mut p = std::path::PathBuf::from(base);
    for part in parts {
        p.push(part);
    }
    p.to_string_lossy().into_owned()
}
fn basename(path: &str) -> &str {
    path.rsplit(|c| c == '/' || (cfg!(windows) && c == '\\'))
        .next()
        .unwrap_or("")
}
fn call(io: &mut impl Adapter, kind: &str, args: Value) -> Result<Value> {
    io.action(json!({"kind":kind,"args":args}))
}
fn print(io: &mut impl Adapter, line: String) -> Result<()> {
    call(io, "print", json!([line]))?;
    Ok(())
}
fn read(io: &mut impl Adapter, path: &str) -> Result<Vec<u8>> {
    bytes(&call(io, "read", json!([path]))?)
}
struct Voice {
    label: String,
    record: [u8; 128],
    original: String,
    bank: String,
    slot: i64,
    source: String,
}
pub fn generate(io: &mut impl Adapter, argv: &Value, root: &str, doc: &str) -> Result<()> {
    let argv = argv
        .as_array()
        .ok_or_else(|| Error::value("expected generator arguments"))?
        .iter()
        .map(text)
        .collect::<Result<Vec<_>>>()?;
    if argv.len() > 128 {
        return Err(Error::value("too many generator arguments"));
    }
    let args: Vec<_> = argv
        .iter()
        .copied()
        .filter(|s| !s.starts_with("--"))
        .collect();
    if args.len() < 2 {
        return Err(error("SystemExit", doc));
    }
    let (manifest, banks) = (args[0], args[1]);
    let rom = argv
        .iter()
        .position(|s| *s == "--rom")
        .map(|i| argv.get(i + 1).copied().ok_or_else(index_error))
        .transpose()?
        .filter(|s| !s.is_empty());
    let mut roms = BTreeSet::<[u8; 118]>::new();
    if let Some(dir) = rom {
        let files = call(io, "listdir", json!([dir]))?;
        let mut files = files
            .as_array()
            .ok_or_else(|| Error::value("invalid ROM listing"))?
            .iter()
            .map(text)
            .collect::<Result<Vec<_>>>()?;
        if files.len() > 4096 {
            return Err(Error::value("too many ROM files"));
        }
        files.sort_unstable();
        for file in files {
            let lower = file.to_lowercase();
            if lower.starts_with("rom") && lower.ends_with(".syx") {
                let data = read(io, &join(dir, &[file]))?;
                for bank in BulkBanks::new(&data) {
                    for record in bank.chunks_exact(128) {
                        roms.insert(record[..118].try_into().unwrap());
                        if roms.len() > 65536 {
                            return Err(Error::value("too many ROM voices"));
                        }
                    }
                }
            }
        }
        print(io, format!("{} ROM voices to steer clear of", roms.len()))?;
    }
    call(io, "open_lines", json!([manifest]))?;
    let (mut voices, mut users) = (BTreeMap::<i64, Voice>::new(), Vec::<[u8; 128]>::new());
    let mut count = 0;
    loop {
        let line = call(io, "line", json!([]))?;
        if line.is_null() {
            break;
        }
        let line = text(&line)?
            .trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c));
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        count += 1;
        if count > MAX_LINES || line.len() > 16384 {
            return Err(Error::value("manifest exceeds supported bound"));
        }
        let fields: Vec<_> = line.splitn(5, '|').map(str::trim).collect();
        if fields.len() != 5 {
            return Err(Error::value(format!(
                "not enough values to unpack (expected 5, got {})",
                fields.len()
            )));
        }
        let (slot, label, bank, vslot, source) =
            (fields[0], fields[1], fields[2], fields[3], fields[4]);
        let (path, which) = bank.split_once('#').unwrap_or((bank, ""));
        let data = read(io, &join(banks, &[path]))?;
        let dumps: Vec<_> = BulkBanks::new(&data).collect();
        if dumps.is_empty() {
            return Err(error(
                "SystemExit",
                format!("{path}: no 32-voice dump in it"),
            ));
        }
        let chosen = select(&dumps, integer(if which.is_empty() { "0" } else { which })?)?;
        let vslot_n = integer(vslot)?;
        let record = select(&chosen.chunks_exact(128).collect::<Vec<_>>(), vslot_n)?.to_vec();
        let original = name(&record);
        let flag = roms.contains(<&[u8; 118]>::try_from(&record[..118]).unwrap());
        print(
            io,
            format!(
                "{slot:>2} {label:<16} <- {original:<10} alg {:>2}  {} {vslot}{}",
                (record[110] & 31) + 1,
                basename(path),
                if flag { "  ROM!" } else { "" }
            ),
        )?;
        if flag {
            return Err(error(
                "SystemExit",
                format!("{label}: the same operators as a Yamaha ROM voice; pick another"),
            ));
        }
        let record: [u8; 128] = record.try_into().unwrap();
        if slot == "U" {
            users.push(record);
        } else {
            voices.insert(
                integer(slot)?,
                Voice {
                    label: label.into(),
                    record,
                    original,
                    bank: basename(path).into(),
                    slot: vslot_n,
                    source: source.into(),
                },
            );
        }
    }
    call(io, "close_lines", json!([]))?;
    let n = voices.len();
    if voices.keys().copied().ne(0..n as i64) {
        return Err(error("SystemExit", "the slots are not 0..n-1"));
    }
    let mut out=vec!["/* SPDX-License-Identifier: GPL-3.0-only */".to_owned(),"/* The DY voices: human-made DX7 voices from public-domain banks (tools/fm6_dy.txt lists each one's bank and source; LICENSING.md the terms),".into()," * one for each of the thirty DX voices, as 128-byte packed records (the DX7's bulk-dump format). Generated by tools/gen_fm6_dy.py: do not edit. */".into(),"#pragma once".into(),"#include <stdint.h>".into(),format!("#define FM6_NDY {n}"),format!("static const char *const FM6_DY_NAME[FM6_NDY] = {{{}}};",voices.values().map(|v|format!("\"{}\"",v.original.replace('\\',"/").replace('"',"'"))).collect::<Vec<_>>().join(", ")),"static const uint8_t FM6_DY[FM6_NDY][128] = {".into()];
    for (i, v) in voices.values().enumerate() {
        out.push(format!(
            "    {{/* {i}: {}: \"{}\", {} voice {}: {} */",
            v.label, v.original, v.bank, v.slot, v.source
        ));
        for (k, row) in v.record.chunks(16).enumerate() {
            out.push(format!(
                "     {}{}",
                row.iter()
                    .map(|b| format!("{b:3}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                if k < 7 { "," } else { "" }
            ));
        }
        out.push(if i < n - 1 { "    }," } else { "    }" }.into());
    }
    out.push("};".into());
    let hp = join(root, &["firmware", "src", "dsp", "fm6_dy.h"]);
    call(io, "write_text", json!([hp, out.join("\n") + "\n"]))?;
    print(io, format!("{hp}: {n} voices"))?;
    call(io, "mkdir", json!([join(root, &["build"])]))?;
    let bp = join(root, &["build", "fm6_dy.bin"]);
    let binary: Vec<u8> = voices.values().flat_map(|v| v.record).collect();
    call(io, "write", json!([bp, binary]))?;
    print(io, format!("{bp}: {n} records"))?;
    if !users.is_empty() {
        let mut body = Vec::with_capacity(4096);
        let initial = init();
        for i in 0..32 {
            body.extend_from_slice(users.get(i).unwrap_or(&initial));
        }
        let mut syx = vec![240, 67, 0, 9, 32, 0];
        syx.extend_from_slice(&body);
        syx.push(fm1_formats::wire::sum7(&body));
        syx.push(247);
        let sp = join(root, &["build", "PERF-1-test-voices.syx"]);
        call(io, "write", json!([sp, syx]))?;
        print(
            io,
            format!("{sp}: {} voices (4104 bytes)", users.len().min(32)),
        )?;
        if users.len() > 32 {
            print(
                io,
                format!(
                    "({} U lines past the 32 a bank holds were left out)",
                    users.len() - 32
                ),
            )?;
        }
    }
    Ok(())
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "dumps" => {
            let b = bytes(&a[0])?;
            json!(BulkBanks::new(&b)
                .map(|b| b.chunks_exact(128).map(|v| v.to_vec()).collect::<Vec<_>>())
                .collect::<Vec<_>>())
        }
        "name" => json!(name(&bytes(&a[0])?)),
        "init" => json!(init().to_vec()),
        "main" => {
            generate(io, &a[0], text(&a[1])?, text(&a[2])?)?;
            Value::Null
        }
        _ => return Err(Error::value("unknown DY asset method")),
    })
}

#[derive(Default)]
pub struct Native {
    lines: Option<std::io::BufReader<std::fs::File>>,
}
impl Adapter for Native {
    fn action(&mut self, v: Value) -> Result<Value> {
        let a = &v["args"];
        Ok(match text(&v["kind"])? {
            "listdir" => json!(std::fs::read_dir(text(&a[0])?)?
                .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
                .collect::<std::io::Result<Vec<_>>>()?),
            "read" => json!(crate::read_file(text(&a[0])?, 1_100_000)?),
            "open_lines" => {
                self.lines = Some(std::io::BufReader::new(std::fs::File::open(text(&a[0])?)?));
                Value::Null
            }
            "line" => {
                let mut s = String::new();
                if self
                    .lines
                    .as_mut()
                    .ok_or_else(|| Error::value("no manifest"))?
                    .read_line(&mut s)?
                    == 0
                {
                    Value::Null
                } else {
                    json!(s)
                }
            }
            "close_lines" => {
                self.lines = None;
                Value::Null
            }
            "print" => {
                let line = text(&a[0])?.to_owned() + if cfg!(windows) { "\r\n" } else { "\n" };
                std::io::stdout().write_all(line.as_bytes())?;
                Value::Null
            }
            "mkdir" => {
                std::fs::create_dir_all(text(&a[0])?)?;
                Value::Null
            }
            "write_text" => {
                std::fs::write(text(&a[0])?, text(&a[1])?)?;
                Value::Null
            }
            "write" => {
                std::fs::write(text(&a[0])?, bytes(&a[1])?)?;
                Value::Null
            }
            _ => return Err(Error::value("unknown native DY action")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audition_init_retains_its_distinct_operator_six_level() {
        assert_eq!(init()[99], 0);
        assert_eq!(fm1_engine::fm6::tables::FM6_INIT[99], 99);
        assert_ne!(init(), fm1_engine::fm6::tables::FM6_INIT);
        assert!(integer("9223372036854775808").is_err());
        assert_eq!(integer(" -1_024 ").unwrap(), -1024);
    }
}
