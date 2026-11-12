// SPDX-License-Identifier: GPL-3.0-only
use crate::{bytes, number, text, Error, Result};
use fm1_formats::tool_wire;
use serde_json::{json, Value};
include!("telemetry_fields.rs");
pub fn decode(kind: u8, data: &[u8]) -> Result<Value> {
    let (fields, min, pad) = match kind {
        70 => (&FIELDS[..], 35, true),
        90 => (&EXTRA[..], 20, true),
        91 => (&DIAG[..], 36, false),
        71 => (&[][..], 16, false),
        _ => return Err(Error::value("unknown telemetry frame kind")),
    };
    if data.len() < 3 || data[..3] != [125, 80, kind] {
        return Ok(Value::Null);
    }
    let max = if kind == 71 { 16 } else { fields.len() };
    let mut out = [0u8; 160];
    let n = tool_wire::telemetry_compatible(data, kind, max, &mut out)
        .map_err(|_| Error::value("invalid telemetry frame"))?
        / 4;
    if n < min {
        return Ok(Value::Null);
    }
    let words: Vec<_> = out[..n * 4]
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    if kind == 71 {
        return Ok(json!(words));
    }
    let mut v = json!({});
    for (i, f) in fields
        .iter()
        .enumerate()
        .take(if pad { fields.len() } else { n })
    {
        v[*f] = json!(words.get(i).copied().unwrap_or(0));
    }
    if kind == 70 {
        let shed = words[10];
        for (f, shift, mask) in [
            ("chorus", 8, 1),
            ("bass", 9, 1),
            ("flash", 12, 3),
            ("susp_bad", 14, 1),
            ("boot", 15, 3),
        ] {
            v[f] = json!((shed >> shift) & mask)
        }
        v["shed"] = json!(shed & 255);
    }
    Ok(v)
}
fn integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| Error::value("expected telemetry integer"))
}
fn get(v: &Value, k: &str) -> i64 {
    v[k].as_i64()
        .or_else(|| v[k].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}
pub fn events(d: &Value, prev: &Value) -> Vec<String> {
    let mut e = Vec::new();
    if prev.is_object() {
        for (field, prefix) in [("xruns", "XRUN"), ("late", "LATE")] {
            let delta = get(d, field) - get(prev, field);
            if delta > 0 {
                e.push(format!("{prefix}+{delta}"))
            }
        }
        if get(d, "stores") > get(prev, "stores") {
            e.push(format!("FLASH({}us)", get(d, "store_us")))
        }
        if get(d, "seq") != get(prev, "seq") + 1 {
            e.push(format!("LOST{}", get(d, "seq") - get(prev, "seq") - 1))
        }
        if get(d, "shed") != get(prev, "shed") {
            e.push(format!("SHED{}", get(d, "shed")))
        }
        if get(d, "crashes") != get(prev, "crashes") {
            e.push("CRASH".into())
        }
    }
    if get(d, "max_us") > get(d, "budget_us") {
        e.push("OVER".into())
    }
    if get(d, "gap_us") > 20000 {
        e.push(format!("GAP{}ms", get(d, "gap_us") / 1000))
    }
    e
}
pub fn bench(w: &[Value]) -> Result<Value> {
    if w.len() != 16 {
        return Err(Error::value("benchmark requires16 words"));
    }
    let w = w.iter().map(integer).collect::<Result<Vec<_>>>()?;
    let bn = w[0];
    let tpu = w[1];
    let mut out = format!("bench: {bn} turns each, {tpu} ticks per us\n");
    macro_rules! nonzero {($v:expr,$message:expr)=>{if $v==0.0{return Ok(json!({"text":out,"error":$message}))}};}
    nonzero!(bn as f64, "division by zero");
    let base = w[9] as f64 / bn as f64;
    for (k, name) in BENCH_NAMES.iter().enumerate() {
        let n = if k < 8 { bn } else { (bn / 64) * 256 };
        nonzero!(n as f64, "division by zero");
        let per = w[2 + k] as f64 / n as f64;
        nonzero!(tpu as f64, "float division by zero");
        nonzero!(base, "float division by zero");
        out += &format!(
            "  {name:<34} {per:8.2} ticks/turn = {:7.1} ns  ({:.2}x the int chain)\n",
            per / tpu as f64 * 1000.0,
            per / base
        );
    }
    if w[3] != 0 {
        nonzero!(w[2] as f64, "float division by zero");
        out += &format!(
            "  RAM code runs the float loop in {:.0}% of flash code's time\n",
            100.0 * w[3] as f64 / w[2] as f64
        )
    }
    if w[7] != 0 {
        nonzero!(w[6] as f64, "float division by zero");
        out += &format!(
            "  RAM table is read in {:.0}% of the flash table's time\n",
            100.0 * w[7] as f64 / w[6] as f64
        )
    }
    out+=&format!("  cache 0x1EEE008 {:08X}  0x1EEE00C {:08X}  0x1EEE010 {:08X}   PLL 0x16A00 {:08X}  0x10008 {:08X}\n",w[11],w[12],w[13],w[14],w[15]);
    Ok(json!({"text":out,"error":null}))
}
/// CSV and live display use the same Rust-selected event order and column list.
pub fn row(now: f64, start: f64, d: &Value, prev: &Value, extra: &Value, diag: &Value) -> Value {
    let ev = events(d, prev);
    let mut row = vec![json!(format!("{:.2}", now - start))];
    for k in FIELDS
        .iter()
        .copied()
        .chain(["chorus", "bass", "flash", "susp_bad", "boot"])
    {
        row.push(json!(get(d, k)))
    }
    for k in EXTRA {
        row.push(json!(get(extra, k)))
    }
    for k in DIAG {
        row.push(json!(get(diag, k)))
    }
    row.push(json!(ev.join(" ")));
    let live=format!("\r{:6.1}s cpu {:3}% c1 {:3}% xr {:<4} max {:5.1}ms/{:.1} gap {:5.1}ms ui {:5.1}ms shed {} {}        ",now-start,get(d,"cpu"),get(d,"cpu1"),get(d,"xruns"),get(d,"max_us")as f64/1000.0,get(d,"budget_us")as f64/1000.0,get(d,"gap_us")as f64/1000.0,get(d,"ui_us")as f64/1000.0,get(d,"shed"),ev.join(" "));
    json!({"row":row,"live":live})
}
pub fn request(method: &str, v: &Value) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "telemetry.fields" => {
            json!({"fields":&FIELDS[..],"extra":&EXTRA[..],"diag":&DIAG[..],"bench_names":BENCH_NAMES,"genres":crate::summary::GENRES,"stages":crate::summary::KEYS})
        }
        "telemetry.decode" => decode(
            a[0].as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| Error::value("frame kind must be byte"))?,
            &bytes(&a[1])?,
        )?,
        "telemetry.events" => json!(events(&a[0], &a[1])),
        "telemetry.arm" => json!(number(&a[0])? - number(&a[1])? > 3.0),
        "telemetry.timing" => {
            json!({"bench_seconds":5,"bench_poll_seconds":0.02,"poll_seconds":0.01})
        }
        "telemetry.bench" => json!(bench(
            a[0].as_array()
                .ok_or_else(|| Error::value("expected benchmark words"))?
        )?),
        "telemetry.row" => row(number(&a[0])?, number(&a[1])?, &a[2], &a[3], &a[4], &a[5]),
        "telemetry.summary" => json!(crate::summary::report(
            text(&a[0])?,
            a[1].as_array()
                .ok_or_else(|| Error::value("expected CSV rows"))?
        )?),
        "telemetry.pick" => {
            let key = text(&a[1])?.to_lowercase();
            a[0].as_array()
                .ok_or_else(|| Error::value("expected ports"))?
                .iter()
                .find(|v| {
                    v.as_str().is_some_and(|s| {
                        let s = s.to_lowercase();
                        if key.is_empty() {
                            ["fm-1", "fm1", "perf", "m-vave", "mvave"]
                                .iter()
                                .any(|k| s.contains(k))
                        } else {
                            s.contains(&key)
                        }
                    })
                })
                .cloned()
                .unwrap_or(Value::Null)
        }
        _ => return Err(Error::value("unknown tooling method")),
    })
}
