// SPDX-License-Identifier: GPL-3.0-only
//! Host-side interpretation of the existing vendor image/disassembly gates.
use crate::{Error, Result};
use regex::Regex;
use std::{collections::BTreeSet, sync::LazyLock};
const BODY_MAX: i128 = 0x8c000;
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if matches!(
            c,
            '\n' | '\r'
                | '\u{b}'
                | '\u{c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        ) {
            result.push(&text[start..at]);
            start = at + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|(_, c)| *c == '\n') {
                chars.next();
                start += 1;
            }
        }
    }
    if start < text.len() {
        result.push(&text[start..]);
    }
    result.into_iter()
}
static LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\s\x1c-\x1f]*([0-9a-f]+):[\s\x1c-\x1f]+((?:[0-9a-f]{2} )+)[\s\x1c-\x1f]*\t(.*)$")
        .unwrap()
});
static REFERENCES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"= (-?\d+) <|call -?\d+ <[^:>]*: ([0-9a-f]+) >").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_]$").unwrap());
static PRINTABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{M}\p{N}\p{P}\p{S} ]$").unwrap());
static DECIMAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{Nd}$").unwrap());
fn contains_call(line: &str) -> bool {
    let word = |c: Option<char>| {
        c.is_some_and(|c| {
            let mut buf = [0; 4];
            WORD.is_match(c.encode_utf8(&mut buf))
        })
    };
    line.match_indices("call").any(|(at, _)| {
        !word(line[..at].chars().next_back()) && !word(line[at + 4..].chars().next())
    })
}
static SOFT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)[\s\x1c-\x1f](__(?:add|sub|mul|div|neg|cmp|eq|ne|lt|le|gt|ge|un|extendsf|truncdf|fixdf|fixunsdf|floatsidf|floatunsidf|floatdidf)[\p{L}\p{N}_]*df[\p{L}\p{N}_]*)$").unwrap()
});
pub(crate) fn symbol(text: &str, name: &str) -> Result<Option<i128>> {
    let regex = Regex::new(&format!(
        r"(?m)^([0-9a-f]+) .*[\s\x1c-\x1f]{}$",
        regex::escape(name)
    ))
    .unwrap();
    regex
        .captures(text)
        .map(|m| {
            i128::from_str_radix(&m[1], 16)
                .map_err(|_| Error::value("symbol address exceeds128 bits"))
        })
        .transpose()
}
pub(crate) fn quoted(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if {
                let mut buffer = [0; 4];
                !PRINTABLE.is_match(c.encode_utf8(&mut buffer))
            } =>
            {
                let n = c as u32;
                if n <= 0xff {
                    out.push_str(&format!("\\x{n:02x}"));
                } else if n <= 0xffff {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    out.push_str(&format!("\\U{n:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
fn list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| quoted(v))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
pub(crate) fn decimal_word(text: &str) -> Result<u32> {
    let (negative, digits) = text.strip_prefix('-').map_or((false, text), |v| (true, v));
    let mut value = 0u32;
    for c in digits.chars() {
        let digit = if c.is_ascii_digit() {
            c as u32 - '0' as u32
        } else {
            let decimal = |c: char| {
                let mut buffer = [0; 4];
                DECIMAL.is_match(c.encode_utf8(&mut buffer))
            };
            if !decimal(c) {
                return Err(Error::value("invalid disassembly decimal literal"));
            }
            // Nd ranges consist of ordered 0..9 sets; adjacent sets (such as
            // mathematical styles) restart at zero every ten scalars.
            let mut zero = c as u32;
            while let Some(previous) = zero.checked_sub(1).and_then(char::from_u32) {
                if !decimal(previous) {
                    break;
                }
                zero -= 1;
            }
            (c as u32 - zero) % 10
        };
        value = value.wrapping_mul(10).wrapping_add(digit);
    }
    Ok(if negative {
        value.wrapping_neg()
    } else {
        value
    })
}
pub(crate) fn rom_addresses(
    disassembly: &str,
    mut emit: impl FnMut(u32) -> Result<()>,
) -> Result<()> {
    for line in lines(disassembly) {
        let Some(instruction) = LINE.captures(line) else {
            continue;
        };
        for capture in REFERENCES.captures_iter(&instruction[3]) {
            let value = if let Some(decimal) = capture.get(1) {
                decimal_word(decimal.as_str())?
            } else {
                capture[2].bytes().fold(0u32, |v, c| {
                    v.wrapping_mul(16)
                        .wrapping_add((c as char).to_digit(16).unwrap())
                })
            };
            if (0xffc00000..0xffd00000).contains(&value) {
                emit(value)?;
            }
        }
    }
    Ok(())
}
pub fn check(
    image: &[u8],
    symbols: &str,
    disassembly: &str,
    ram_text: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let (mut errors, mut notes) = (Vec::new(), Vec::new());
    if symbol(symbols, "_start")? != Some(0x02001000) {
        errors.push("_start is not at 0x2001000".into());
    }
    if image.get(..4) != Some(&[4, 129, 128, 0]) {
        let prefix = image
            .iter()
            .take(4)
            .map(|v| format!("{v:02x}"))
            .collect::<String>();
        errors.push(format!("image starts with {prefix}, not the entry stub"));
    }
    let calls = lines(ram_text)
        .filter(|line| contains_call(line))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !calls.is_empty() {
        errors.push(format!(
            ".ram_text contains calls: {}",
            list(&calls[..calls.len().min(3)])
        ));
    } else {
        notes.push(format!(
            ".ram_text: {} insns, no calls",
            lines(ram_text).filter(|line| LINE.is_match(line)).count()
        ));
    }
    rom_addresses(disassembly, |value| {
        errors.push(format!("reference to ROM address {value:#010x}"));
        Ok(())
    })?;
    if image.len() > 0x8dfbc {
        errors.push(format!("image {} B exceeds the app slot", image.len()));
    }
    let body = image.len() as i128 - 0xee0;
    if body > BODY_MAX {
        errors.push(format!(
            "app body {body} B overlaps the commit-marker sector (at most {BODY_MAX} B)"
        ));
    } else {
        notes.push(format!(
            "body {body} B of {BODY_MAX} below the commit marker's sector"
        ));
    }
    let soft = SOFT
        .captures_iter(symbols)
        .map(|m| m[1].to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if !soft.is_empty() {
        errors.push(format!(
            "soft-double routines linked: {} (a double in the code)",
            list(&soft[..soft.len().min(6)])
        ));
    } else {
        notes.push("no soft-double routines (single-precision FPU only)".into());
    }
    let bss = symbol(symbols, "_bss_end")?.unwrap_or(0) - 0x01c08000;
    let pool =
        symbol(symbols, "_pool_end")?.unwrap_or(0) - symbol(symbols, "_pool_start")?.unwrap_or(0);
    notes.push(format!(
        "image {} B; RAM .data+.bss {bss} B of 98304; pool {pool} B of 344064",
        image.len()
    ));
    if bss > 98304 {
        errors.push("RAM region overflow".into());
    }
    if 344064 - pool < 4096 {
        errors.push(format!("pool headroom {} B < 4096 B", 344064 - pool));
    }
    Ok((errors, notes))
}
