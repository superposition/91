// SPDX-License-Identifier: GPL-3.0-only
//! Host protocol orchestration as a bounded action machine. MIDI ports, clocks
//! and iterators stay in the I/O adapter; every validation/selection/timeout and
//! transaction decision is made from copied observations here.
use crate::{bytes, get, integer, list, py_repr, string, Error, Result};
use fm1_formats::{crc32, device_wire, tool_wire, wire};
use serde_json::{json, Value};
const ERRORS: [&str; 6] = [
    "malformed request or invalid pack",
    "checksum mismatch",
    "device busy",
    "stop playback before saving",
    "persistent save/readback failed",
    "unsupported pack version",
];
const FIELDS: [&str; 22] = [
    "on",
    "key",
    "bars",
    "form",
    "movement",
    "energy",
    "hold",
    "bass",
    "idea",
    "slot",
    "play",
    "song",
    "swell",
    "variation",
    "keys_program",
    "bass_program",
    "touch",
    "release",
    "bass_gate",
    "swell_voice",
    "bass_attack",
    "brightness",
];
fn runtime(message: impl Into<String>) -> Error {
    Error {
        kind: "RuntimeError",
        message: message.into(),
    }
}
pub fn errors() -> Value {
    let mut v = json!({});
    for (i, e) in ERRORS.iter().enumerate() {
        v[(i + 1).to_string()] = json!(e);
    }
    v
}
pub fn fields() -> Value {
    let mut v = json!({});
    for (i, f) in FIELDS.iter().enumerate() {
        v[*f] = json!(i);
    }
    v
}
pub fn control_check(field: &str, value: &Value) -> Result<Value> {
    let (lo, hi) = match field {
        "on" | "hold" | "play" => (0, 1),
        "key" | "swell_voice" => (0, 23),
        "bars" => (8, 16),
        "form" | "movement" | "variation" => (0, 3),
        "bass" | "keys_program" | "bass_program" => (0, 7),
        "idea" => (1, 1),
        "song" => (0, 6),
        "release" | "bass_gate" => (1, 100),
        "energy" | "swell" | "touch" | "bass_attack" | "brightness" => (0, 100),
        _ => return Err(Error::missing(field)),
    };
    let n = integer(value, lo, hi, field)?;
    if field == "bars" && ![8, 16].contains(&n) {
        return Err(Error::value("bars must be 8 or 16"));
    }
    Ok(json!(n))
}
pub fn pick(names: &Value, port: Option<&Value>) -> Result<Value> {
    let selected: Vec<_> = list(names)?
        .iter()
        .filter_map(|name| name.as_str())
        .filter(|name| {
            let name = name.to_lowercase();
            if let Some(port) = port.and_then(Value::as_str).filter(|s| !s.is_empty()) {
                name.contains(&port.to_lowercase())
            } else {
                name.contains("fm-1") || name.contains("fm1")
            }
        })
        .collect();
    if selected.len() != 1 {
        return Err(runtime(format!(
            "Expected one FM-1 port; use --port. Available: {}",
            py_repr(names)
        )));
    }
    Ok(json!(selected[0]))
}
fn done(v: Value) -> Value {
    json!({"done":true,"value":v})
}
fn next(mut state: Value, phase: u8, action: Value) -> Value {
    state["phase"] = json!(phase);
    json!({"done":false,"state":state,"action":action})
}
fn request(state: Value, phase: u8, op: u8, data: Value, timeout: f64) -> Value {
    next(
        state,
        phase,
        json!({"kind":"request","op":op,"data":data,"timeout":timeout}),
    )
}
fn clock(state: Value, phase: u8) -> Value {
    next(state, phase, json!({"kind":"clock"}))
}
fn sleep(state: Value, phase: u8, time: f64) -> Value {
    next(state, phase, json!({"kind":"sleep","seconds":time}))
}
fn observed_error(v: &Value) -> Result<Error> {
    let kind = match string(get(v, "kind")?)? {
        "ValueError" => "ValueError",
        "KeyError" => "KeyError",
        "TimeoutError" => "TimeoutError",
        "OSError" => "OSError",
        "AssertionError" => "AssertionError",
        _ => "RuntimeError",
    };
    Ok(Error {
        kind,
        message: string(get(v, "message")?)?.into(),
    })
}
fn raw(v: &Value) -> Result<Vec<u8>> {
    bytes(v)
}
pub fn start(operation: &Value, args: &Value) -> Result<Value> {
    let method = string(operation)?;
    let valid = [
        "request",
        "capabilities",
        "features",
        "song_status",
        "status",
        "read",
        "audition",
        "wait_active",
        "save",
        "control",
    ];
    if !valid.contains(&method) {
        return Err(Error::value("unknown device operation"));
    }
    advance(
        json!({"operation":method,"args":args,"phase":0}),
        &Value::Null,
        None,
    )
}
pub fn parse_reply(data: &[u8], op: u8, token: u8) -> Result<Option<Vec<u8>>> {
    if data.len() < 7 || data[..5] != [125, 80, 97, op, token] {
        return Ok(None);
    }
    let mut frame = vec![240];
    frame.extend(data);
    frame.push(247);
    let reply = wire::JamReply::parse(&frame).map_err(|_| Error::value("damaged device reply"))?;
    if reply.status() != 0 {
        return Err(runtime(
            ERRORS.get(reply.status() as usize - 1).map_or_else(
                || format!("device error {}", reply.status()),
                |s| (*s).into(),
            ),
        ));
    }
    let mut out = vec![0; 68];
    let n = reply
        .decode(&mut out)
        .map_err(|_| Error::value("damaged device reply"))?;
    out.truncate(n);
    Ok(Some(out))
}
fn time(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error::value("finite clock observation required"))
}
fn abort(mut s: Value, error: Error) -> Value {
    s["failure"] = json!({"kind":error.kind,"message":error.message});
    request(s, 5, 9, json!([]), 3.0)
}
pub fn advance(mut s: Value, value: &Value, error: Option<&Value>) -> Result<Value> {
    let method = string(get(&s, "operation")?)?.to_owned();
    let phase = integer(get(&s, "phase")?, 0, 10, "device phase")? as u8;
    let a = get(&s, "args")?.clone();
    if let Some(error) = error.filter(|e| !e.is_null()) {
        let e = observed_error(error)?;
        if method == "features" && e.kind == "RuntimeError" && e.message == ERRORS[0] {
            return Err(runtime(
                "device needs the Songbook 0.3.0 engine upgrade for v2 songs and DX voice roles",
            ));
        }
        if method == "audition" && phase >= 2 && phase != 5 {
            return Ok(abort(s, e));
        }
        if method != "audition" || phase != 5 {
            return Err(e);
        }
    }
    Ok(match method.as_str() {
        "request" => {
            let op = integer(get(&a, "op")?, 0, 127, "operation")? as u8;
            let token = integer(get(&a, "token")?, 0, 127, "token")? as u8;
            match phase {
                0 => {
                    let data = raw(get(&a, "data")?)?;
                    let mut frame = vec![0; data.len() + 8];
                    let n =
                        device_wire::dispatch(85, &[op as u32, token as u32], &data, &mut frame)
                            .map_err(|_| Error::value("SysEx payload is not seven-bit clean"))?;
                    next(s, 1, json!({"kind":"send","data":&frame[1..n-1]}))
                }
                1 => clock(s, 2),
                2 => {
                    s["until"] = json!(time(value)? + time(get(&a, "timeout")?)?);
                    clock(s, 3)
                }
                3 => {
                    if time(value)? < time(&s["until"])? {
                        next(s, 4, json!({"kind":"pending-start"}))
                    } else {
                        return Err(Error{kind:"TimeoutError",message:format!("FM-1 did not answer jam operation {op}; it may need the initial Jam firmware upgrade")});
                    }
                }
                4 => next(s, 5, json!({"kind":"pending-next"})),
                5 => {
                    if value.is_null() {
                        sleep(s, 6, 0.005)
                    } else if value["type"] == "sysex" {
                        if let Some(b) = parse_reply(&raw(get(value, "data")?)?, op, token)? {
                            done(json!(b))
                        } else {
                            next(s, 5, json!({"kind":"pending-next"}))
                        }
                    } else {
                        next(s, 5, json!({"kind":"pending-next"}))
                    }
                }
                6 => clock(s, 3),
                _ => return Err(Error::value("invalid request phase")),
            }
        }
        "capabilities" | "features" | "song_status" | "status" => {
            if phase == 0 {
                request(
                    s,
                    1,
                    match method.as_str() {
                        "capabilities" => 1,
                        "features" => 11,
                        "song_status" => 12,
                        _ => 8,
                    },
                    json!([]),
                    3.0,
                )
            } else {
                let b = raw(value)?;
                let v = match method.as_str() {
                    "capabilities" => {
                        if b.len() != 6 || b[0] != 1 {
                            return Err(runtime("unsupported Jam protocol"));
                        }
                        json!({"protocol":b[0],"max_bytes":u16::from_le_bytes([b[1],b[2]]),"slots":b[3],"first_object":b[4],"session_object":b[5]})
                    }
                    "features" => {
                        if b.len() != 4 {
                            return Err(Error::value("bad feature response"));
                        }
                        json!({"pack_versions":(0..2).filter(|i|b[0]>>i&1!=0).map(|i|i+1).collect::<Vec<_>>(),"voice_programs":b[1],"factory_songs":b[2],"fm_voices":b[3]})
                    }
                    "song_status" => {
                        if b.len() != 3 {
                            return Err(Error::value("bad song status"));
                        }
                        json!({"song":b[0],"pack_version":b[1],"loading":b[2]})
                    }
                    _ => {
                        if b.len() != 25 {
                            return Err(Error::value("bad status length"));
                        }
                        let mut v = json!({});
                        for (i, key) in ["on", "bars", "bar", "beat", "hold", "pending", "slot"]
                            .iter()
                            .enumerate()
                        {
                            v[*key] = json!(b[i]);
                        }
                        v["crc32"] = json!(format!(
                            "{:08x}",
                            u32::from_le_bytes(b[7..11].try_into().unwrap())
                        ));
                        v["revision"] = json!(u32::from_le_bytes(b[11..15].try_into().unwrap()));
                        v["xruns"] = json!(u32::from_le_bytes(b[15..19].try_into().unwrap()));
                        for (i, key) in ["key", "minor", "form", "movement", "energy", "playing"]
                            .iter()
                            .enumerate()
                        {
                            v[*key] = json!(b[19 + i]);
                        }
                        v
                    }
                };
                done(v)
            }
        }
        "read" => {
            if phase == 0 {
                s["bytes"] = json!([]);
                s["index"] = json!(0);
                let slot = get(&a, "slot")?.clone();
                request(s, 1, 2, json!([slot, 0]), 3.0)
            } else {
                let r = raw(value)?;
                if r.len() < 4 {
                    return Err(Error::value("truncated readback"));
                }
                let total = u16::from_le_bytes([r[0], r[1]]) as usize;
                let off = u16::from_le_bytes([r[2], r[3]]) as usize;
                let b = s["bytes"]
                    .as_array_mut()
                    .ok_or_else(|| Error::value("invalid readback state"))?;
                if total > 3840
                    || off != b.len()
                    || off > total
                    || r.len() - 4 != 64.min(total - off)
                {
                    return Err(Error::value("inconsistent readback"));
                }
                b.extend(r[4..].iter().map(|n| json!(n)));
                if b.len() == total {
                    done(json!(b))
                } else {
                    let index = integer(&s["index"], 0, 59, "readback index")? + 1;
                    s["index"] = json!(index);
                    request(s, 1, 2, json!([get(&a, "slot")?, index]), 3.0)
                }
            }
        }
        "audition" => {
            let b = raw(get(&a, "bytes")?)?;
            match phase {
                0 => {
                    if b.len() > 16383 {
                        return Err(Error::value("SysEx payload is not seven-bit clean"));
                    }
                    let mut out = [0; 10];
                    tool_wire::jam_begin(b.len() as u16, crc32(&b), &mut out)
                        .map_err(|_| Error::value("invalid audition length"))?;
                    s["index"] = json!(0);
                    request(s, 1, 3, json!(out), 3.0)
                }
                1 | 2 => {
                    let index = integer(&s["index"], 0, 64, "upload chunk")? as usize;
                    if phase == 2
                        && raw(value)?
                            != [index
                                .checked_sub(1)
                                .ok_or_else(|| Error::value("invalid upload chunk state"))?
                                as u8]
                    {
                        return Ok(abort(s, Error::value("wrong chunk acknowledgement")));
                    }
                    if index * 256 >= b.len() {
                        request(s, 3, 5, json!([]), 3.0)
                    } else {
                        let chunk = &b[index * 256..b.len().min((index + 1) * 256)];
                        let mut out = [0; 515];
                        let n = tool_wire::jam_chunk(index as u8, chunk, &mut out)
                            .map_err(|_| Error::value("invalid audition chunk"))?;
                        s["index"] = json!(index + 1);
                        request(s, 2, 4, json!(&out[..n]), 3.0)
                    }
                }
                3 => request(s, 4, 6, json!([]), 3.0),
                4 => done(Value::Null),
                5 => return Err(observed_error(get(&s, "failure")?)?),
                _ => return Err(Error::value("invalid audition phase")),
            }
        }
        "wait_active" => match phase {
            0 => {
                let b = raw(get(&a, "bytes")?)?;
                s["want"] = json!(format!("{:08x}", crc32(&b)));
                clock(s, 1)
            }
            1 => {
                s["until"] = json!(time(value)? + time(get(&a, "timeout")?)?);
                clock(s, 2)
            }
            2 => {
                if time(value)? < time(&s["until"])? {
                    next(s, 3, json!({"kind":"status"}))
                } else {
                    return Err(Error {
                        kind: "TimeoutError",
                        message: "pack is still pending; status can check it without resending"
                            .into(),
                    });
                }
            }
            3 => {
                if value["pending"] == 0 && value["crc32"] == s["want"] {
                    done(value.clone())
                } else {
                    sleep(s, 4, 0.25)
                }
            }
            4 => clock(s, 2),
            _ => return Err(Error::value("invalid activation phase")),
        },
        "save" => match phase {
            0 => next(s, 1, json!({"kind":"status"})),
            1 => {
                if value["playing"].as_i64().unwrap_or(0) != 0 {
                    return Err(runtime(ERRORS[3]));
                }
                s["before"] = value.clone();
                request(s, 2, 7, json!([get(&a, "slot")?]), 15.0)
            }
            2 => {
                s["reply"] = value.clone();
                next(s, 3, json!({"kind":"read","slot":get(&a,"slot")?}))
            }
            3 => {
                let saved = raw(value)?;
                let reply = raw(&s["reply"])?;
                let crc = crc32(&saved);
                if reply.len() != 4
                    || crc != u32::from_le_bytes(reply[..4].try_into().unwrap())
                    || json!(format!("{crc:08x}")) != s["before"]["crc32"]
                {
                    return Err(runtime("saved pack failed readback comparison"));
                }
                done(
                    json!({"slot":get(&a,"slot")?,"bytes":saved.len(),"crc32":s["before"]["crc32"]}),
                )
            }
            _ => return Err(Error::value("invalid save phase")),
        },
        "control" => match phase {
            0 => {
                let field = string(get(&a, "field")?)?;
                let index = FIELDS
                    .iter()
                    .position(|f| *f == field)
                    .ok_or_else(|| Error::missing(field))?;
                let value = integer(get(&a, "value")?, 0, 16383, "control value")?;
                let mut out = [0; 3];
                tool_wire::jam_control(index as u8, value as u16, &mut out)
                    .map_err(|_| Error::value("invalid control"))?;
                request(s, 1, 10, json!(out), 3.0)
            }
            1 => sleep(s, 2, 0.04),
            2 => next(s, 3, json!({"kind":"status"})),
            3 => done(value.clone()),
            _ => return Err(Error::value("invalid control phase")),
        },
        _ => return Err(Error::value("unknown device operation")),
    })
}
