// SPDX-License-Identifier: GPL-3.0-only
//! Inspect compiler products without executing them or changing the boot stub.
use crate::{
    build_guards::{decimal_word, lines, quoted, rom_addresses, symbol},
    Error, Result,
};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
const STUB_LEN: usize = 0xee0;
static RELEASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A(\d{1,2})\.(\d{1,2})(?:\.(\d{1,2}))?(-[A-Za-z0-9]+)?\z").unwrap()
});
pub fn release(value: Option<&str>, product: &str, version: Option<&str>) -> Result<Value> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(json!({"product":product,"version":version,"name":"naiwan91.fwsc"}));
    };
    let Some(parts) = RELEASE.captures(value) else {
        return Err(exit(format!(
            "--release {value}: use X.Y[.Z][-suffix], up to two digits each"
        )));
    };
    let major = decimal_word(&parts[1])?;
    let minor = decimal_word(&parts[2])?;
    let patch = parts.get(3).map_or(Ok(0), |v| decimal_word(v.as_str()))?;
    Ok(
        json!({"product":format!("FM-1_7{major:02}{minor:02}{patch:02}"),"version":value.to_uppercase(),"name":format!("naiwan91-{value}.fwsc")}),
    )
}
pub fn sdk_check(name: &str, data: &[u8], expected: &str) -> Option<String> {
    (format!("{:x}",Sha256::digest(data))!=expected).then(||format!("warning: SDK {name} differs from AC79NN_SDK_V1.2.1; the package will not match the reference"))
}
fn exit(message: impl Into<String>) -> Error {
    Error {
        kind: "SystemExit".into(),
        code: String::new(),
        message: message.into(),
    }
}
fn whitespace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}
fn fields(line: &str) -> Vec<&str> {
    line.split(whitespace).filter(|v| !v.is_empty()).collect()
}
fn integer(text: &str) -> Result<i128> {
    let original = text;
    let invalid = || {
        Error::value(format!(
            "invalid literal for int() with base 16: {}",
            quoted(original)
        ))
    };
    let (negative, text) = if let Some(v) = text.strip_prefix('-') {
        (true, v)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    let prefix = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"));
    let text = prefix
        .map(|v| v.strip_prefix('_').unwrap_or(v))
        .unwrap_or(text);
    let bytes = text.as_bytes();
    if bytes.is_empty()
        || !bytes.iter().enumerate().all(|(at, b)| {
            b.is_ascii_hexdigit()
                || (*b == b'_'
                    && at > 0
                    && at + 1 < bytes.len()
                    && bytes[at - 1].is_ascii_hexdigit()
                    && bytes[at + 1].is_ascii_hexdigit())
        })
    {
        return Err(invalid());
    }
    let digits = text.replace('_', "");
    let value = i128::from_str_radix(&digits, 16).map_err(|_| invalid())?;
    Ok(if negative { -value } else { value })
}
fn hex(value: i128) -> String {
    if value < 0 {
        format!("-0x{:x}", value.unsigned_abs())
    } else {
        format!("{value:#x}")
    }
}
pub fn loader_sections(headers: &str) -> Result<()> {
    for line in lines(headers) {
        let p = fields(line);
        if p.len() > 4
            && p[1].starts_with('.')
            && integer(p[3])? >= 0x02000000
            && integer(p[2])? != 0
        {
            return Err(exit(format!(
                "loader: section {} at {} is outside RAM",
                p[1], p[3]
            )));
        }
    }
    Ok(())
}
pub fn loader_image(length: u64) -> Result<()> {
    if length == 0 || length > 0x14000 {
        return Err(exit("loader: image empty or too big"));
    }
    Ok(())
}
pub fn loader_summary(image: usize, symbols: &str) -> Result<String> {
    let mut end = None;
    for line in lines(symbols) {
        if line.trim_end_matches(whitespace).ends_with("_bss_end") {
            let values = fields(line);
            let value = integer(values.first().copied().unwrap_or(""))?;
            if end.is_none() {
                end = Some(value);
            }
        }
    }
    Ok(format!(
        "loader: image {image} B at 0x1c0a800{}",
        end.map(|v| format!(", bss end {}", hex(v)))
            .unwrap_or_default()
    ))
}
pub fn stub(image: &[u8], headers: &str, symbols: &str, disassembly: &str) -> Result<Value> {
    for line in lines(headers) {
        let p = fields(line);
        if p.len() > 3
            && p[0].bytes().all(|v| v.is_ascii_digit())
            && p[1].starts_with('.')
            && integer(p[2])? != 0
            && ![".text", ".shstrtab", ".symtab", ".strtab"].contains(&p[1])
        {
            return Err(exit(format!(
                "stub: section {} holds {} B; the stub is code and constants only",
                p[1],
                integer(p[2])?
            )));
        }
    }
    if symbol(symbols, "_start")? != Some(0x02000120) {
        return Err(exit("stub: _start is not at 0x2000120"));
    }
    if image.get(..4) != Some(&[4, 129, 128, 0]) {
        return Err(exit(format!(
            "stub: starts with {}, not the entry the SPL expects (goto +4; rts)",
            image
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )));
    }
    if image.get(8..14) != Some(b"CRSTUB") {
        return Err(exit("stub: no CRSTUB signature at offset 8"));
    }
    if image.is_empty() || image.len() > STUB_LEN - 256 {
        return Err(exit(format!(
            "stub: {} B does not fit {STUB_LEN} B with 256 B to spare",
            image.len()
        )));
    }
    rom_addresses(disassembly, |address| {
        Err(exit(format!(
            "stub: reference to ROM address {address:#010x}"
        )))
    })?;
    let used = image.len();
    let mut padded = image.to_vec();
    padded.resize(STUB_LEN, 255);
    let version = u16::from_le_bytes([padded[14], padded[15]]);
    let sha = format!("{:x}", Sha256::digest(&padded));
    let line = format!(
        "stub: {used} B of {STUB_LEN} ({:.1}%), version {version}, sha256 {}",
        100.0 * used as f64 / STUB_LEN as f64,
        &sha[..16]
    );
    Ok(json!({"image":padded,"line":line,"version":version,"sha":sha}))
}
pub fn stub_lock(
    text: Option<&str>,
    path: &str,
    version: u16,
    sha: &str,
    update: bool,
    release: bool,
) -> Result<Value> {
    let want = text.map(fields).unwrap_or_default();
    let ver = version.to_string();
    if want == [ver.as_str(), sha] {
        return Ok(json!({"kind":"same"}));
    }
    if update {
        return Ok(
            json!({"kind":"update","write":format!("{ver} {sha}\n"),"line":format!("stub: {path} updated")}),
        );
    }
    let prefix = want.join(" ").chars().take(20).collect::<String>();
    let msg=format!("the boot stub differs from {path} ({}..): installing this build rewrites sector 0x4000, the one window an interruption is not rescued in. If the change is meant: bump STUB_VERSION (firmware/stub/stub_core.h), rebuild with FM1_STUB_LOCK=update, and say so in the release notes",if prefix.is_empty(){"none"}else{&prefix});
    if release {
        return Err(exit(format!("build: {msg}")));
    }
    Ok(json!({"kind":"warning","line":format!("WARNING: {msg}")}))
}
