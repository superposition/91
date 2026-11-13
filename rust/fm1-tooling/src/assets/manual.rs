// SPDX-License-Identifier: GPL-3.0-only
//! Exact manual-tool grammar and workflow. File/PDF/browser effects are adapters.
use super::logo::b64;
use crate::{bytes, number, text, Adapter, Error, Result};
use fancy_regex::{Captures, Regex, RegexBuilder};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
pub mod page;
pub mod pdf;
pub mod quick;
const LIMIT: usize = 16 * 1024 * 1024;
pub fn copy() -> Value {
    serde_json::from_str(include_str!("../../../../tools/data/manual-copy.json"))
        .expect("checked manual copy")
}
pub fn css(quick: bool) -> &'static str {
    if quick {
        include_str!("../../../../tools/data/quick-manual.css")
    } else {
        include_str!("../../../../tools/data/manual.css")
    }
}
pub fn error(kind: &str, message: impl Into<String>) -> Error {
    Error {
        kind: kind.into(),
        code: String::new(),
        message: message.into(),
    }
}
pub fn space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}
pub fn trim(s: &str) -> &str {
    s.trim_matches(space)
}
pub fn escape(s: &str, quote: bool) -> String {
    let mut o = String::new();
    for c in s.chars() {
        o.push_str(match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' if quote => "&quot;",
            '\'' if quote => "&#x27;",
            _ => {
                o.push(c);
                continue;
            }
        });
    }
    o
}
fn pattern(p: &str) -> String {
    let mut out = String::new();
    let mut class = 0;
    let mut chars = p.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(c) = chars.next() {
                let value = match c {
                    'w' => Some(r"\p{L}\p{N}_"),
                    's' => Some(r"\p{White_Space}\x1c-\x1f"),
                    'd' => Some(r"\p{Nd}"),
                    _ => None,
                };
                if let Some(value) = value {
                    if class == 0 && c != 'd' {
                        out.push('[');
                        out += value;
                        out.push(']');
                    } else {
                        out += value;
                    }
                } else if c == 'S' {
                    out += r"[^\p{White_Space}\x1c-\x1f]";
                } else {
                    out.push('\\');
                    out.push(c);
                }
            } else {
                out.push('\\');
            }
        } else {
            if c == '[' {
                class += 1;
            }
            if c == ']' {
                class -= 1;
            }
            out.push(c);
        }
    }
    out
}
fn re(p: &str) -> Result<Arc<Regex>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Arc<Regex>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| Error::value("manual regex cache poisoned"))?;
    if let Some(r) = cache.get(p) {
        return Ok(Arc::clone(r));
    }
    if p.len() > 4096 {
        return Err(Error::value("manual pattern exceeds bound"));
    }
    let r = Arc::new(
        RegexBuilder::new(&pattern(p))
            .backtrack_limit(1_000_000)
            .build()
            .map_err(|e| error("error", e.to_string()))?,
    );
    if cache.len() >= 256 {
        cache.clear();
    }
    cache.insert(p.into(), Arc::clone(&r));
    Ok(r)
}
fn checked(s: &str) -> Result<()> {
    if s.len() > LIMIT {
        Err(Error::value("manual text exceeds16MiB bound"))
    } else {
        Ok(())
    }
}
fn append(out: &mut String, s: &str) -> Result<()> {
    if out.len().saturating_add(s.len()) > LIMIT {
        return Err(Error::value("manual output exceeds16MiB bound"));
    }
    out.push_str(s);
    Ok(())
}
#[derive(Default)]
struct Output {
    rows: Vec<String>,
    len: usize,
}
impl Output {
    fn push(&mut self, s: String) -> Result<()> {
        let n = self
            .len
            .saturating_add(s.len())
            .saturating_add(usize::from(!self.rows.is_empty()));
        if n > LIMIT {
            return Err(Error::value("manual output exceeds16MiB bound"));
        }
        self.len = n;
        self.rows.push(s);
        Ok(())
    }
    fn last(&self) -> Option<&String> {
        self.rows.last()
    }
    fn finish(self) -> String {
        self.rows.join("\n")
    }
}
fn join_checked(values: impl IntoIterator<Item = Result<String>>, sep: &str) -> Result<String> {
    let mut out = String::new();
    let mut first = true;
    for value in values {
        if !first {
            append(&mut out, sep)?;
        }
        first = false;
        append(&mut out, &value?)?;
    }
    Ok(out)
}
fn sub(p: &str, s: &str, mut f: impl FnMut(&Captures<'_>) -> Result<String>) -> Result<String> {
    checked(s)?;
    let regex = re(p)?;
    let (mut out, mut at) = (String::new(), 0);
    for c in regex.captures_iter(s) {
        let c = c.map_err(|e| Error::value(e.to_string()))?;
        let m = c.get(0).unwrap();
        append(&mut out, &s[at..m.start()])?;
        append(&mut out, &f(&c)?)?;
        at = m.end();
    }
    append(&mut out, &s[at..])?;
    Ok(out)
}
fn matched(p: &str, s: &str) -> Result<bool> {
    re(p)?.is_match(s).map_err(|e| Error::value(e.to_string()))
}
fn groups(p: &str, s: &str) -> Result<Option<Vec<String>>> {
    Ok(re(p)?
        .captures(s)
        .map_err(|e| Error::value(e.to_string()))?
        .map(|c| {
            (0..c.len())
                .map(|i| c.get(i).map_or("", |m| m.as_str()).to_owned())
                .collect()
        }))
}
fn lines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
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
            out.push(&s[start..i]);
            start = i + c.len_utf8();
            if c == '\r' && it.peek().is_some_and(|(_, c)| *c == '\n') {
                start = it.next().unwrap().0 + 1;
            }
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}
fn call(io: &mut impl Adapter, kind: &str, args: Value) -> Result<Value> {
    io.action(json!({"kind":kind,"args":args}))
}
fn print(io: &mut impl Adapter, line: impl Into<String>) -> Result<()> {
    call(io, "print", json!([line.into()]))?;
    Ok(())
}
fn string_list(v: &Value) -> Result<Vec<&str>> {
    v.as_array()
        .ok_or_else(|| Error::value("expected manual arguments"))?
        .iter()
        .map(text)
        .collect()
}
fn integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .filter(|n| n.unsigned_abs() <= 1 << 40)
        .ok_or_else(|| Error::value("expected bounded manual integer"))
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "config" => json!({"copy":copy(),"manual_css":css(false),"quick_css":css(true)}),
        "inline" => json!(page::inline(text(&a[0])?, &a[1])?),
        "slug" => json!(page::slug(text(&a[0])?)?),
        "figure" => json!(page::figure(text(&a[0])?, text(&a[1])?, &a[2], io)?),
        "uri" => json!(
            "data:image/png;base64,".to_owned()
                + &b64(&bytes(&call(io, "read", json!([text(&a[0])?]))?)?)
        ),
        "convert" => json!(page::convert(text(&a[0])?, &a[1], io)?),
        "main" => {
            page::main(&a[0], &a[1], io)?;
            Value::Null
        }
        "quick-inline" => json!(quick::inline(text(&a[0])?)?),
        "table" => json!(quick::table(&string_list(&a[0])?)?),
        "blocks" => json!(quick::blocks(text(&a[0])?, &a[1], io)?),
        "quick-convert" => json!(quick::convert(text(&a[0])?, &a[1], io)?),
        "to-pdf" => {
            pdf::to_pdf(text(&a[0])?, text(&a[1])?, text(&a[2])?, io)?;
            Value::Null
        }
        "measure" => {
            pdf::measure(text(&a[0])?, io)?;
            Value::Null
        }
        "page-report" => {
            let (n, firsts) = pdf::report(text(&a[0])?, io)?;
            json!([n, firsts])
        }
        "quick-main" => json!(pdf::quick_main(&a[0], &a[1], io)?),
        "full-main" => json!(pdf::full_main(&a[0], &a[1], io)?),
        "count" => json!(pdf::count(&bytes(&a[0])?, a[1] == true)),
        _ => return Err(Error::value("unknown manual method")),
    })
}
pub fn native(input: &str, quick_html: bool) -> Result<String> {
    struct Offline;
    impl Adapter for Offline {
        fn action(&mut self, _: Value) -> Result<Value> {
            Err(Error::value("unexpected native document I/O"))
        }
    }
    let copy = copy();
    let cfg = json!({"copy":copy,"name":copy["name"],"css":css(quick_html),"img_base":Value::Null,"embed":false,"part_colors":{}});
    if quick_html {
        quick::convert(input, &cfg, &mut Offline)
    } else {
        page::convert(input, &cfg, &mut Offline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_grammars_keep_python_words_spaces_and_distinct_code_spans() {
        assert_eq!(
            page::inline("a\u{301}*x*", &json!({})).unwrap(),
            "a\u{301}<em>x</em>"
        );
        assert_eq!(quick::inline("a\u{301}*x*").unwrap(), "a\u{301}<em>x</em>");
        assert_eq!(page::inline("*\u{1f}x*", &json!({})).unwrap(), "*\u{1f}x*");
        assert_eq!(
            page::inline("`**x**`", &json!({})).unwrap(),
            "<code><strong>x</strong></code>"
        );
        assert_eq!(quick::inline("`**x**`").unwrap(), "<code>**x**</code>");
    }
    #[test]
    fn repeated_embedding_refuses_growth_before_reading_every_image() {
        struct Images(usize);
        impl Adapter for Images {
            fn action(&mut self, v: Value) -> Result<Value> {
                assert_eq!(v["kind"], "read");
                self.0 += 1;
                Ok(json!(vec![0u8; 128 * 1024]))
            }
        }
        let cfg = json!({"name":"NAIWAN 91","copy":copy(),"img_base":"virtual","embed":true,"part_colors":{}});
        let mut io = Images(0);
        let result = page::convert(&"![x](a.png)\n".repeat(200), &cfg, &mut io);
        assert_eq!(result.unwrap_err().kind, "ValueError");
        assert!(io.0 > 1 && io.0 < 200);
    }
    #[test]
    fn binary_pdf_counters_retain_eof_boundary_and_consumption_profiles() {
        assert_eq!(pdf::count(b"/Type/Page", false), 1);
        assert_eq!(pdf::count(b"/Type/Page", true), 0);
        assert_eq!(pdf::count(b"/Type/Page/Type/Page!", false), 2);
        assert_eq!(pdf::count(b"/Type/Page/Type/Page!", true), 1);
        assert_eq!(pdf::count(b"/Type\x1f/Page!", false), 0);
    }
}
