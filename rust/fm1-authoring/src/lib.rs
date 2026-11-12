// SPDX-License-Identifier: GPL-3.0-only
//! Host musical documents and imports. Stored data uses the shared explicit
//! pack layout; unknown JSON fields remain authoring metadata, never ABI state.
#![forbid(unsafe_code)]
pub mod acceptance;
pub mod cli;
pub mod device;
mod factory;
mod midi;
mod pack;
pub mod preview;
pub use factory::{factory, programs, song};
pub use midi::{import_midi, inspect_midi, midi_from_log, Selection};
pub use pack::{compile, decode, Document, Limits};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: &'static str,
    pub message: String,
}
impl Error {
    pub fn value(message: impl Into<String>) -> Self {
        Self {
            kind: "ValueError",
            message: message.into(),
        }
    }
    fn missing(name: &str) -> Self {
        Self {
            kind: "KeyError",
            message: name.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.kind == "KeyError" {
            write!(f, "'{}'", self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: "OSError",
            message: e.to_string(),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::value(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn get<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key).ok_or_else(|| Error::missing(key))
}
fn list(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or_else(|| Error::value("expected a list"))
}
fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| Error::value("expected a string"))
}
fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::String(s) => {
            let quote = if s.contains('\'') && !s.contains('"') {
                '"'
            } else {
                '\''
            };
            let mut out = String::from(quote);
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c == quote => {
                        out.push('\\');
                        out.push(c);
                    }
                    c if c.is_control() => {
                        use std::fmt::Write;
                        let n = c as u32;
                        if n <= 255 {
                            write!(out, "\\x{n:02x}").unwrap();
                        } else {
                            write!(out, "\\u{n:04x}").unwrap();
                        }
                    }
                    c => out.push(c),
                }
            }
            out.push(quote);
            out
        }
        Value::Array(a) => format!("[{}]", a.iter().map(py_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}: {}", py_repr(&json!(k)), py_repr(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => v.to_string(),
    }
}
fn integer(v: &Value, lo: i64, hi: i64, name: &str) -> Result<i64> {
    v.as_i64().filter(|n| (lo..=hi).contains(n)).ok_or_else(|| {
        Error::value(format!(
            "{name}: expected integer {lo}..{hi}, got {}",
            py_repr(v)
        ))
    })
}
fn bytes(v: &Value) -> Result<Vec<u8>> {
    list(v)?
        .iter()
        .map(|n| integer(n, 0, 255, "byte").map(|x| x as u8))
        .collect()
}
fn append_source(target: &mut Value, source: Value) -> Result<()> {
    if let Some(array) = target.as_array_mut() {
        array.push(source);
        return Ok(());
    }
    let name = match target {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Object(_) => "dict",
        Value::Array(_) => unreachable!(),
    };
    Err(Error {
        kind: "AttributeError",
        message: format!("'{name}' object has no attribute 'append'"),
    })
}
/// The authoring importer deliberately scans *only* bulk VMEM headers at any
/// byte offset, like the original bank tool. Checksum/trailer validation belongs
/// to strict live MIDI messages; this profile retains file selection behavior.
pub fn import_voice(
    data: &[u8],
    base: &Value,
    index: i64,
    role: &str,
    name: &str,
    limits: Limits<'_>,
) -> Result<Value> {
    let mut voices = Vec::new();
    let mut at = 0;
    while let Some(n) = data
        .get(at..)
        .and_then(|s| s.windows(2).position(|w| w == [240, 67]))
    {
        let start = at + n;
        if start + 4104 > data.len() {
            break;
        }
        if data[start + 3..start + 6] == [9, 32, 0] {
            voices.extend(data[start + 6..start + 4102].chunks_exact(128));
            at = start + 4104;
        } else {
            at = start + 1;
        }
    }
    let voice = usize::try_from(index)
        .ok()
        .and_then(|i| voices.get(i))
        .ok_or_else(|| Error::value("voice index outside bank"))?;
    let mut d = decode(&compile(base, limits)?, limits)?.into_value();
    if d["version"] != 2 {
        return Err(Error::value("voice roles require a Songbook v2 pack"));
    }
    let n = list(get(&d, "voice_programs")?)?.len();
    if n >= 8 {
        return Err(Error::value("all eight program slots are used"));
    }
    if !["keys", "bass"].contains(&role) {
        return Err(Error::value("voice role must be keys or bass"));
    }
    let voice_name: String = voice[118..128]
        .iter()
        .map(|b| {
            if (32..127).contains(b) {
                char::from(*b)
            } else {
                '?'
            }
        })
        .collect();
    d["parts"][role] = json!(n);
    d["voice_programs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":voice_name.trim_end(),"data":voice}));
    append_source(
        &mut d["sources"],
        json!(format!("User-selected DX voice: {name}, voice {index}")),
    )?;
    compile(&d, limits)?;
    Ok(d)
}

/// A small JSON transport for the retained Python entry points. All musical
/// work happens here; Python is responsible only for file/device adapters.
pub fn request(request: &Value) -> Result<Value> {
    let method = string(get(request, "method")?)?;
    let limits = request.get("limits").filter(|v| !v.is_null());
    match method {
        "integer" => Ok(json!(integer(
            get(request, "value")?,
            integer(get(request, "low")?, i64::MIN, i64::MAX, "minimum")?,
            integer(get(request, "high")?, i64::MIN, i64::MAX, "maximum")?,
            string(get(request, "name")?)?
        )?)),
        "c-header" => Ok(json!(cli::c_header(
            &bytes(get(request, "bytes")?)?,
            string(get(request, "name")?)?,
            string(get(request, "prefix")?)?
        ))),
        "device-control-check" => {
            device::control_check(string(get(request, "field")?)?, get(request, "value")?)
        }
        "device-needs-features" => Ok(json!(bytes(get(request, "bytes")?)?.get(4) == Some(&2))),
        "device-compatible" => {
            if !list(get(get(request, "features")?, "pack_versions")?)?.contains(&json!(2)) {
                return Err(Error::value("device needs Songbook engine upgrade"));
            }
            Ok(Value::Null)
        }
        "device-pull-check" => {
            if bytes(get(request, "bytes")?)?.is_empty() {
                return Err(Error::value("slot is empty"));
            }
            Ok(Value::Null)
        }
        "device-errors" => Ok(device::errors()),
        "device-fields" => Ok(device::fields()),
        "device-pick" => device::pick(get(request, "names")?, request.get("port")),
        "device-start" => device::start(get(request, "operation")?, get(request, "args")?),
        "device-next" => device::advance(
            get(request, "state")?.clone(),
            request.get("value").unwrap_or(&Value::Null),
            request.get("error"),
        ),
        "device-reply" => Ok(json!(device::parse_reply(
            &bytes(get(request, "bytes")?)?,
            integer(get(request, "op")?, 0, 127, "operation")? as u8,
            integer(get(request, "token")?, 0, 127, "token")? as u8
        )?)),
        "device-frame" => {
            let args = [
                integer(get(request, "op")?, 0, 127, "operation")
                    .map_err(|_| Error::value("SysEx payload is not seven-bit clean"))?
                    as u32,
                integer(get(request, "token")?, 0, 127, "token")
                    .map_err(|_| Error::value("SysEx payload is not seven-bit clean"))?
                    as u32,
            ];
            let data = bytes(get(request, "bytes")?)
                .map_err(|_| Error::value("SysEx payload is not seven-bit clean"))?;
            let mut out = vec![0; data.len() + 8];
            let n = fm1_formats::device_wire::dispatch(85, &args, &data, &mut out)
                .map_err(|_| Error::value("SysEx payload is not seven-bit clean"))?;
            Ok(json!(&out[1..n - 1]))
        }
        "accept-init" => acceptance::initial(get(request, "features")?),
        "accept-ready" => acceptance::ready(request),
        "accept-step" => acceptance::step(request),
        "accept-cleanup" => acceptance::cleanup(get(request, "before")?),
        "accept-validate" => {
            acceptance::validate(request)?;
            Ok(Value::Null)
        }
        "accept-finalize" => acceptance::finalize(request),
        "accept-start" => Ok(
            json!([{"kind":"request","command":13,"data":[1]},{"kind":"control","field":"play","value":1}]),
        ),
        "accept-window" => Ok(json!(
            request["elapsed"]
                .as_f64()
                .ok_or_else(|| Error::value("elapsed time required"))?
                < request["seconds"]
                    .as_f64()
                    .ok_or_else(|| Error::value("duration required"))?
        )),
        "accept-exit" => Ok(json!(u8::from(!list(get(request, "errors")?)?.is_empty()))),
        "factory" => Ok(factory()),
        "song" => song(integer(get(request, "song")?, 0, 2, "song")? as usize),
        "programs" => Ok(programs()),
        "compile" => Ok(json!(compile(get(request, "document")?, limits)?)),
        "decode" => Ok(decode(&bytes(get(request, "bytes")?)?, limits)?.into_value()),
        "inspect-midi" => inspect_midi(&bytes(get(request, "bytes")?)?),
        "import-midi" => import_midi(
            &bytes(get(request, "bytes")?)?,
            get(request, "document")?,
            midi::Selection {
                track: integer(get(request, "track")?, i64::MIN, i64::MAX, "track")?,
                tonic: integer(get(request, "tonic")?, 0, 11, "tonic")? as u8,
                bar: integer(get(request, "bar")?, 1, i32::MAX as i64, "bar")? as u32,
                role: string(get(request, "role")?)?,
                minor: request["minor"].as_bool().unwrap_or(false),
                name: string(get(request, "name")?)?,
            },
            limits,
        ),
        "import-voice" => import_voice(
            &bytes(get(request, "bytes")?)?,
            get(request, "document")?,
            integer(get(request, "voice")?, i64::MIN, i64::MAX, "voice")?,
            string(get(request, "role")?)?,
            string(get(request, "name")?)?,
            limits,
        ),
        "midi-log" => Ok(json!(midi_from_log(
            string(get(request, "log")?)?,
            integer(get(request, "bpm")?, 1, 60000000, "BPM")? as u32
        )?)),
        "preview-plan" => Ok(json!(preview::plan(preview::Options::from_json(request)?)?)),
        "preview" => preview::run(
            string(get(request, "root")?)?,
            string(get(request, "pack")?)?,
            string(get(request, "output")?)?,
            preview::Options::from_json(request)?,
            limits,
        ),
        _ => Err(Error::value("unknown authoring operation")),
    }
}
