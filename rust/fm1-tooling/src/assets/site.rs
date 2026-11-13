// SPDX-License-Identifier: GPL-3.0-only
//! Static-site data transforms. Templates and musician-facing text remain data;
//! filesystem ownership/rollback and delivery are separate transitional seams.
use super::{logo::b64, manual::error};
use crate::{bytes, text, Adapter, Error, Result};
use fancy_regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
pub const LIMIT: usize = 16 * 1024 * 1024;
pub fn bounded(s: &str) -> Result<()> {
    if s.len() > LIMIT {
        Err(Error::value("site text exceeds16MiB bound"))
    } else {
        Ok(())
    }
}
fn replace(s: &str, from: &str, to: &str) -> Result<String> {
    bounded(s)?;
    let count = s.matches(from).count();
    let length = count.checked_mul(to.len()).and_then(|n| {
        s.len()
            .checked_sub(count.checked_mul(from.len())?)
            .and_then(|rest| rest.checked_add(n))
    });
    if length.is_none_or(|n| n > LIMIT) {
        return Err(Error::value("site output exceeds16MiB bound"));
    }
    Ok(s.replace(from, to))
}
pub fn object(rows: &[(&str, Value)]) -> Value {
    let mut m = Map::new();
    for (k, v) in rows {
        m.insert((*k).into(), v.clone());
    }
    Value::Object(m)
}
pub fn sub(p: &str, s: &str, replacement: &str) -> Result<String> {
    bounded(s)?;
    let r = Regex::new(p).map_err(|e| Error::value(e.to_string()))?;
    let mut out = String::new();
    let mut at = 0;
    for m in r.find_iter(s) {
        let m = m.map_err(|e| Error::value(e.to_string()))?;
        out.push_str(&s[at..m.start()]);
        out.push_str(replacement);
        at = m.end();
        if out.len() > LIMIT {
            return Err(Error::value("site output exceeds16MiB bound"));
        }
    }
    out.push_str(&s[at..]);
    bounded(&out)?;
    Ok(out)
}
fn quoted(s: &str, ascii: bool) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c < '\u{20}' || (ascii && c >= '\u{7f}') => {
                let mut buf = [0; 2];
                for n in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{n:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn encode(
    v: &Value,
    ascii: bool,
    indent: Option<usize>,
    compact: bool,
    sort: bool,
    level: usize,
    out: &mut String,
) -> Result<()> {
    if level > 96 {
        return Err(Error::value("site JSON nesting exceeds96"));
    }
    let nl = |out: &mut String, l: usize| {
        if let Some(n) = indent {
            out.push('\n');
            out.push_str(&" ".repeat(l * n));
        }
    };
    match v {
        Value::String(s) => out.push_str(&quoted(s, ascii)),
        Value::Array(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                    if indent.is_none() && !compact {
                        out.push(' ');
                    }
                }
                nl(out, level + 1);
                encode(v, ascii, indent, compact, sort, level + 1, out)?;
            }
            if !a.is_empty() {
                nl(out, level);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            let mut keys: Vec<_> = m.keys().collect();
            if sort {
                keys.sort();
            }
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                    if indent.is_none() && !compact {
                        out.push(' ');
                    }
                }
                nl(out, level + 1);
                out.push_str(&quoted(k, ascii));
                out.push(':');
                if !compact || indent.is_some() {
                    out.push(' ');
                }
                encode(&m[*k], ascii, indent, compact, sort, level + 1, out)?;
            }
            if !m.is_empty() {
                nl(out, level);
            }
            out.push('}');
        }
        Value::Number(n) if n.is_f64() => {
            return Err(Error::value("site metadata JSON requires integer numbers"))
        }
        _ => out.push_str(&v.to_string()),
    }
    if out.len() > LIMIT {
        return Err(Error::value("site JSON exceeds16MiB bound"));
    }
    Ok(())
}
pub fn dumps(
    v: &Value,
    ascii: bool,
    indent: Option<usize>,
    compact: bool,
    sort: bool,
) -> Result<String> {
    let mut out = String::new();
    encode(v, ascii, indent, compact, sort, 0, &mut out)?;
    Ok(out)
}
pub fn strip_module(s: &str) -> Result<String> {
    // Python \s includes the four information separators as well as Unicode
    // White_Space. Import matching deliberately retains the original LF rule.
    sub(r"(?m)^export[\p{White_Space}\x1c-\x1f]+", s, "")
        .and_then(|s| sub(r"(?m)^import .*?;\n", &s, ""))
}
fn decimal(s: &str) -> Result<u32> {
    // Unicode Nd zero codepoints, matching Python int on the release grammar.
    const ZEROS: &[u32] = &[
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0,
        0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0, 0x16a60, 0x16ac0,
        0x16b50, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e950, 0x1fbf0,
    ];
    let mut n = 0;
    for c in s.chars() {
        let c = c as u32;
        let z = ZEROS
            .iter()
            .find(|z| c >= **z && c < **z + 10)
            .ok_or_else(|| {
                Error::value("release decimal digit is outside the Python3.10 Unicode profile")
            })?;
        n = n * 10 + c - z;
    }
    Ok(n)
}
pub fn release_identity(s: &str) -> Result<String> {
    bounded(s)?;
    let r = Regex::new(r"\A(\p{Nd}{1,2})\.(\p{Nd}{1,2})(?:\.(\p{Nd}{1,2}))?(-[A-Za-z0-9]+)?\z")
        .unwrap();
    let c = r
        .captures(s)
        .map_err(|e| Error::value(e.to_string()))?
        .ok_or_else(|| {
            Error::value("release version must be X.Y[.Z][-suffix], up to two digits each")
        })?;
    Ok(format!(
        "FM-1_7{:02}{:02}{:02}",
        decimal(c.get(1).unwrap().as_str())?,
        decimal(c.get(2).unwrap().as_str())?,
        decimal(c.get(3).map_or("0", |m| m.as_str()))?
    ))
}
pub fn redirect(target: &str, label: &str) -> Result<String> {
    Ok(format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>NAIWAN 91 {label}</title><meta http-equiv=\"refresh\" content=\"0; url={target}\"><link rel=\"canonical\" href=\"{target}\"><link rel=\"stylesheet\" href=\"../../site.css\"></head><body><p><a href=\"{target}\">{label}</a></p><script>location.replace({}+location.search+location.hash);</script></body></html>\n",dumps(&json!(target),true,None,false,false)?))
}
pub fn device(data: &str, client: &str, codecs: &str, release: &Value) -> Result<String> {
    let s = format!(
        "{codecs}\n{data}\nconst DEVICE_RELEASE = {};\n{client}",
        dumps(release, true, None, false, false)?.replace('<', "\\u003c")
    );
    bounded(&s)?;
    Ok(s)
}
pub fn runtime(source: &str, wasm: &[u8], manifest: &Value, source_hash: &str) -> Result<String> {
    if manifest["source"] != json!(source_hash)
        || manifest["sha256"] != json!(format!("{:x}", Sha256::digest(wasm)))
    {
        return Err(Error::value(
            "browser codecs are stale; run python3 tools/build_codecs.py",
        ));
    }
    replace(&runtime_body(source)?, "/*CODECS_BASE64*/", &b64(wasm))
}
fn runtime_body(source: &str) -> Result<String> {
    bounded(source)?;
    let start = source
        .find("// NODE_ONLY_BEGIN")
        .ok_or_else(|| Error::value("substring not found"))?;
    let end = source
        .find("// NODE_ONLY_END")
        .ok_or_else(|| Error::value("substring not found"))?
        + 16;
    let s = format!("{}{}", &source[..start], &source[end..]);
    bounded(&s)?;
    Ok(s)
}
pub fn runtime_source(root: &str, io: &mut impl Adapter) -> Result<Value> {
    let mut action = |kind: &str, args: Value| io.action(json!({"kind":kind,"args":args}));
    let manifest = action(
        "read_json",
        json!([format!("{root}/build/codecs/manifest.json")]),
    )?;
    if !manifest.is_object() {
        let kind = match manifest {
            Value::Array(_) => "list",
            Value::Null => "NoneType",
            Value::Bool(_) => "bool",
            Value::String(_) => "str",
            Value::Number(ref n) if n.is_f64() => "float",
            _ => "int",
        };
        return Err(error(
            "AttributeError",
            format!("'{kind}' object has no attribute 'get'"),
        ));
    }
    let source_hash = action("source_hash", json!([root]))?;
    if manifest["source"] != source_hash {
        return Err(Error::value(
            "browser codecs are stale; run python3 tools/build_codecs.py",
        ));
    }
    let data = bytes(&action(
        "read",
        json!([format!("{root}/build/codecs/fm1-codecs.wasm")]),
    )?)?;
    if manifest["sha256"] != json!(format!("{:x}", Sha256::digest(&data))) {
        return Err(Error::value(
            "browser codecs are stale; run python3 tools/build_codecs.py",
        ));
    }
    let source = action(
        "read_text",
        json!([format!("{root}/web/codec-runtime.mjs")]),
    )?;
    let source = runtime_body(text(&source)?)?;
    // Preserve the historical second read after marker parsing, including its
    // side-effect/error order. The file fingerprint is an earlier observation.
    let data = bytes(&action(
        "read",
        json!([format!("{root}/build/codecs/fm1-codecs.wasm")]),
    )?)?;
    Ok(json!(replace(&source, "/*CODECS_BASE64*/", &b64(&data))?))
}
fn copy() -> Value {
    serde_json::from_str(include_str!("../../../../tools/data/site-copy.json"))
        .expect("checked site copy")
}
pub fn manual(source: &str, pdfs: &Value) -> Result<String> {
    let c = copy();
    let mut s = sub(
        r#"<link rel="stylesheet" href="https://fonts.googleapis.com[^>]+>\n"#,
        source,
        "",
    )?;
    s = s.replacen("</head>", text(&c["manual_head"])?, 1).replacen(
        "<main>",
        text(&c["manual_note"])?,
        1,
    );
    for (i, name) in [
        "NAIWAN 91-manual.pdf",
        "NAIWAN 91-quick-manual.pdf",
        "NAIWAN 91-beginners-guide.pdf",
    ]
    .iter()
    .enumerate()
    {
        if !pdfs[i].as_bool().unwrap_or(false) {
            s = sub(
                &format!(
                    r#"(?s)<a(?![\p{{L}}\p{{N}}_])[^>]*href="{}"[^>]*>.*?</a>"#,
                    name.replace('.', r"\.")
                ),
                &s,
                "",
            )?;
        }
    }
    bounded(&s)?;
    Ok(s)
}
pub fn cards(pdfs: &Value) -> Result<String> {
    let c = copy();
    let mut s = String::new();
    for (i, key) in [(2, "beginner_card"), (1, "quick_card"), (0, "full_card")] {
        if pdfs[i].as_bool().unwrap_or(false) {
            s += text(&c[key])?;
        }
    }
    Ok(s)
}
pub fn landing(source: &str, values: &Value) -> Result<String> {
    bounded(source)?;
    let mut out = source.to_owned();
    for (k, v) in values
        .as_object()
        .ok_or_else(|| Error::value("expected landing values"))?
    {
        out = replace(&out, &format!("@@{k}@@"), text(v)?)?;
        bounded(&out)?;
    }
    if Regex::new(r"@@[A-Z_]+@@")
        .unwrap()
        .is_match(&out)
        .map_err(|e| Error::value(e.to_string()))?
    {
        return Err(error("SystemExit", "unresolved landing-page placeholder"));
    }
    Ok(out)
}
pub fn request(method: &str, v: &Value) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "strip-module" => json!(strip_module(text(&a[0])?)?),
        "release" => json!(release_identity(text(&a[0])?)?),
        "redirect" => json!(redirect(text(&a[0])?, text(&a[1])?)?),
        "device" => json!(device(text(&a[0])?, text(&a[1])?, text(&a[2])?, &a[3])?),
        "runtime" => json!(runtime(text(&a[0])?, &bytes(&a[1])?, &a[2], text(&a[3])?)?),
        "manual" => json!(manual(text(&a[0])?, &a[1])?),
        "cards" => json!(cards(&a[0])?),
        "landing" => json!(landing(text(&a[0])?, &a[1])?),
        "sharing" => json!(sub(
            r#"<a (?:class="card" )?href="(?:\.\./)?(?:pieces|library)/(?:[?#][^"]*)?"[^>]*>.*?</a>\n?"#,
            text(&a[0])?,
            ""
        )?),
        "replace" => {
            let mut s = text(&a[0])?.to_owned();
            for pair in a[1]
                .as_array()
                .ok_or_else(|| Error::value("expected replacements"))?
            {
                s = replace(&s, text(&pair[0])?, text(&pair[1])?)?;
                bounded(&s)?;
            }
            json!(s)
        }
        "json" => json!(dumps(
            &a[0],
            a[1].as_bool().unwrap_or(true),
            a[2].as_u64().map(|n| n.min(8) as usize),
            a[3].as_bool().unwrap_or(false),
            a[4].as_bool().unwrap_or(false)
        )?),
        _ => return Err(Error::value("unknown site transform")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn module_and_release_profiles() {
        assert_eq!(
            strip_module("export\u{1c}const a=1;\nimport b;\nx").unwrap(),
            "const a=1;\nx"
        );
        assert_eq!(
            release_identity("\u{ff10}.\u{661}.2-beta").unwrap(),
            "FM-1_7000102"
        );
        assert!(release_identity("1.2.3\n").is_err());
    }
    #[test]
    fn json_order_ascii_and_integer_domain() {
        assert_eq!(
            dumps(
                &object(&[("z", json!("\u{1f600}")), ("a", json!("<\u{7f}"))]),
                true,
                None,
                false,
                false
            )
            .unwrap(),
            r#"{"z": "\ud83d\ude00", "a": "<\u007f"}"#
        );
        assert!(dumps(&json!(0.5), true, None, true, false).is_err());
    }
}
