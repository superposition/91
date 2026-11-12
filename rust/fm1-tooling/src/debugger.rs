// SPDX-License-Identifier: GPL-3.0-only
//! Development MIDI request/poll policy, without a MIDI handle.
use crate::{bytes, number, text, Adapter, Error, Result};
use fm1_formats::tool_wire;
use serde_json::{json, Value};
const HDR: [u8; 3] = [125, 70, 76];
fn exit(message: impl Into<String>) -> Error {
    Error {
        kind: "SystemExit".into(),
        code: String::new(),
        message: message.into(),
    }
}
fn action(io: &mut impl Adapter, kind: &str, args: Value) -> Result<Value> {
    io.action(json!({"kind":kind,"args":args}))
}
pub fn ask(io: &mut impl Adapter, cmd: u8, args: &[u8], timeout: f64) -> Result<Vec<u8>> {
    let names = action(io, "debug_outputs", json!([]))?;
    let name = names
        .as_array()
        .ok_or_else(|| Error::value("expected output names"))?
        .iter()
        .find(|n| {
            n.as_str()
                .is_some_and(|n| n.contains("FM-1") || n.contains("X0X"))
        })
        .cloned()
        .ok_or_else(|| exit("fm1_debug: no FM-1 MIDI port"))?;
    action(io, "debug_open", json!([name]))?;
    let result = (|| {
        while !action(io, "debug_poll", json!([]))?.is_null() {}
        let mut data = HDR.to_vec();
        data.push(cmd);
        data.extend(args);
        action(io, "debug_send", json!([data]))?;
        let end = number(&action(io, "debug_clock", json!([]))?)? + timeout;
        while number(&action(io, "debug_clock", json!([]))?)? < end {
            let message = action(io, "debug_poll", json!([]))?;
            if message.is_null() {
                action(io, "debug_sleep", json!([0.002]))?;
                continue;
            }
            if message["type"] == "sysex" {
                let data = bytes(&message["data"])?;
                if data.len() >= 4 && data[..3] == HDR && data[3] == cmd {
                    return Ok(data[4..].to_vec());
                }
            }
        }
        Err(exit(
            "fm1_debug: no reply (not a debug build, or an address outside the allowed ranges)",
        ))
    })();
    let close = action(io, "debug_close", json!([]));
    close?;
    result
}
fn integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .ok_or_else(|| Error::value("expected bounded integer"))
}
fn w35(n: i64) -> [u8; 5] {
    tool_wire::word35(n as u64)
}
fn r35(b: &[u8], i: usize) -> Result<u32> {
    let error = |code: &str| Error {
        kind: "CodecError".into(),
        code: code.into(),
        message: String::new(),
    };
    tool_wire::read_word35(b.get(i..i.saturating_add(5)).ok_or_else(|| error("-1"))?).map_err(|e| {
        error(if e == fm1_formats::CodecError::Range {
            "-5"
        } else {
            "-1"
        })
    })
}

fn hex(n: i64) -> String {
    if n < 0 {
        format!("-{:07X}", n.unsigned_abs())
    } else {
        format!("{n:08X}")
    }
}
pub fn plan(command: &str, a: i64, b: i64) -> Result<Value> {
    let (cmd, args, timeout) = match command {
        "peek" => {
            let mut data = w35(a).to_vec();
            data.extend(w35(b));
            (40, data, 3)
        }
        "poke" => {
            let mut data = w35(a).to_vec();
            data.extend(w35(b));
            (41, data, 3)
        }
        "clock" => (42, w35(a).to_vec(), 10),
        _ => return Err(Error::value("unknown debugger command")),
    };
    Ok(json!({"cmd":cmd,"args":args,"timeout":timeout,"command":command,"a":a,"b":b}))
}
pub fn report(plan: &Value, reply: &[u8]) -> Result<Value> {
    let a = integer(&plan["a"])?;
    let b = integer(&plan["b"])?;
    let mut out = String::new();
    let result = (|| -> Result<()> {
        match text(&plan["command"])? {
            "peek" => {
                for k in 0..b.max(0) {
                    let addr = a
                        .checked_add(4 * k)
                        .ok_or_else(|| Error::value("address outside supported bound"))?;
                    out += &format!("{}: {:08X}\n", hex(addr), r35(reply, 5 + 5 * k as usize)?);
                }
            }
            "poke" => {
                out = format!(
                    "{}: {:08X} -> {:08X}\n",
                    hex(a),
                    r35(reply, 5)?,
                    r35(reply, 10)?
                )
            }
            "clock" => {
                let (ticks, c0, c1) = (r35(reply, 0)?, r35(reply, 5)?, r35(reply, 10)?);
                let us = f64::from(ticks) / 24.0;
                if us == 0.0 {
                    return Err(Error {
                        kind: "ZeroDivisionError".into(),
                        code: String::new(),
                        message: "float division by zero".into(),
                    });
                }
                let count = c1.wrapping_sub(c0);
                out=format!("{a} iterations: {ticks} ticks = {us:.1} us = {:.2} iterations per us; C0_TL_CKCNT {c0:08X} -> {c1:08X} ({count} counted{}\n",a as f64/us,if c1!=c0{format!(", {:.1} MHz)",f64::from(count)/us)}else{", not counting)".into()});
            }
            _ => return Err(Error::value("unknown debugger report")),
        }
        Ok(())
    })();
    Ok(match result {
        Ok(()) => json!({"text":out,"error":null}),
        Err(e) => json!({"text":out,"error":{"kind":e.kind,"code":e.code,"message":e.message}}),
    })
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "ask" => json!(ask(
            io,
            u8::try_from(integer(&a[0])?).map_err(|_| Error::value("expected command byte"))?,
            &bytes(&a[1])?,
            number(&a[2])?
        )?),
        "plan" => plan(text(&a[0])?, integer(&a[1])?, integer(&a[2])?)?,
        "report" => json!(report(&a[0], &bytes(&a[1])?)?),
        _ => return Err(Error::value("unknown debugger method")),
    })
}
