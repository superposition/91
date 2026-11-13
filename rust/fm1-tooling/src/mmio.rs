// SPDX-License-Identifier: GPL-3.0-only
//! The existing source-level HAL boundary gate, over explicitly supplied files.
use regex::{Captures, Regex};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

static BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)/\*.*?\*/").unwrap());
static COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"//[^\n]*").unwrap());
// Python's Unicode word class excludes combining marks and includes numeric
// categories beyond decimal digits. Its whitespace adds U+001C..U+001F.
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_]$").unwrap());
static TOKENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{N}_]+").unwrap());
static DEFINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^#define[\s\x1c-\x1f]+([\p{L}\p{N}_]+)(?:\([^)]*\))?[\s\x1c-\x1f]+(.*)$")
        .unwrap()
});
static LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)0x0*(1[0-3][0-9a-f]{3}|4[0-3][0-9a-f]{3}|5[01][0-9a-f]{3}|1ee[0-9a-f]{4}|1c7f[0-9a-f]{3}|20[0-9a-f]{5})u?l?").unwrap()
});
static CAST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\([\s\x1c-\x1f]*(?:const[\s\x1c-\x1f]+)?volatile[^()]*\*[\s\x1c-\x1f]*\)").unwrap()
});
static ASM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"__asm__|asm").unwrap());
static BARRIER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^[\s\x1c-\x1f]+volatile[\s\x1c-\x1f]*\([\s\x1c-\x1f]*""[\s\x1c-\x1f]*:::[\s\x1c-\x1f]*"memory"[\s\x1c-\x1f]*\)"#).unwrap()
});
static UPPER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Z_][A-Z0-9_]*").unwrap());

fn word(c: Option<char>) -> bool {
    c.is_some_and(|c| {
        let mut buffer = [0; 4];
        WORD.is_match(c.encode_utf8(&mut buffer))
    })
}
fn bounded(text: &str, start: usize, end: usize) -> bool {
    !word(text[..start].chars().next_back()) && !word(text[end..].chars().next())
}
fn whitespace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}
fn strip(text: &str) -> String {
    let blockless = BLOCK.replace_all(text, |m: &Captures<'_>| {
        "\n".repeat(m[0].bytes().filter(|c| *c == b'\n').count())
    });
    COMMENT.replace_all(&blockless, "").into_owned()
}

pub fn check(headers: &[(String, String)], sources: &[(String, String)]) -> Vec<String> {
    // The filesystem adapter supplies its platform's sorted path order and
    // spelling. Keeping that order preserves duplicate-definition precedence
    // and diagnostics on both Windows and POSIX.
    let mut definitions = BTreeMap::new();
    for (_, text) in headers {
        for capture in DEFINE.captures_iter(&strip(text)) {
            definitions.insert(capture[1].to_owned(), capture[2].to_owned());
        }
    }
    let mut registers = definitions
        .iter()
        .filter(|(_, body)| TOKENS.find_iter(body).any(|m| m.as_str() == "volatile"))
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    loop {
        let more = definitions
            .iter()
            .filter(|(name, body)| {
                !registers.contains(*name)
                    && TOKENS
                        .find_iter(body)
                        .any(|m| registers.contains(m.as_str()))
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        if more.is_empty() {
            break;
        }
        registers.extend(more);
    }
    let mut errors = Vec::new();
    for (name, text) in sources {
        let stripped = strip(text);
        for (index, line) in crate::build_guards::lines(&stripped).enumerate() {
            let location = format!("{name}:{}", index + 1);
            if let Some(m) = LITERAL
                .find_iter(line)
                .find(|m| bounded(line, m.start(), m.end()))
            {
                errors.push(format!(
                    "{location}: register/window address outside hal/ ({}); add a hal/ helper",
                    m.as_str().trim_matches(whitespace)
                ));
            }
            if let Some(m) = CAST.find_iter(line).find(|m| {
                // The word boundary belongs immediately after `volatile`,
                // before the rest of the cast (a greedy match can include it).
                let volatile = m.as_str().find("volatile").unwrap() + m.start();
                !word(line[volatile + 8..].chars().next())
                    && !line[m.end()..]
                        .trim_start_matches(whitespace)
                        .starts_with('&')
            }) {
                errors.push(format!(
                    "{location}: volatile pointer cast outside hal/ ({}); add a hal/ helper",
                    m.as_str().trim_matches(whitespace)
                ));
            }
            if let Some(m) = ASM
                .find_iter(line)
                .find(|m| bounded(line, m.start(), m.end()) && !BARRIER.is_match(&line[m.end()..]))
            {
                errors.push(format!(
                    "{location}: inline asm outside hal/ ({}); add a hal/ helper",
                    m.as_str()
                ));
            }
            if let Some(name) = UPPER
                .find_iter(line)
                .filter(|m| bounded(line, m.start(), m.end()) && registers.contains(m.as_str()))
                .map(|m| m.as_str())
                .min()
            {
                errors.push(format!(
                    "{location}: HAL register macro {name} outside hal/; use a hal/ helper"
                ));
            }
        }
    }
    errors
}
