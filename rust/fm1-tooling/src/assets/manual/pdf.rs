// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use std::collections::VecDeque;
fn index_error(pop: bool) -> Error {
    error(
        "IndexError",
        if pop {
            "pop from empty list"
        } else {
            "list index out of range"
        },
    )
}
fn pages(io: &mut impl Adapter) -> Result<usize> {
    let n = integer(&call(io, "pdf_count", json!([]))?)?;
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= 4096)
        .ok_or_else(|| Error::value("PDF page count exceeds supported bound"))
}
fn raw_pdf(path: &str, io: &mut impl Adapter) -> Result<Vec<u8>> {
    let v = call(io, "read_pdf", json!([path]))?;
    let s = text(&v)?;
    if s.len() > 24 * 1024 * 1024 || s.len() % 2 != 0 {
        return Err(Error::value("PDF bytes exceed12MiB bound"));
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|c| {
            let a = char::from(c[0]).to_digit(16);
            let b = char::from(c[1]).to_digit(16);
            a.zip(b)
                .map(|(a, b)| (a * 16 + b) as u8)
                .ok_or_else(|| Error::value("invalid PDF byte transport"))
        })
        .collect()
}
pub fn to_pdf(html: &str, pdf: &str, edge: &str, io: &mut impl Adapter) -> Result<()> {
    let uri = call(io, "as_uri", json!([html]))?;
    let command = json!([
        edge,
        "--headless",
        "--disable-gpu",
        format!("--print-to-pdf={pdf}"),
        "--no-pdf-header-footer",
        text(&uri)?
    ]);
    if call(io, "exists", json!([pdf]))? == true {
        call(io, "unlink", json!([pdf]))?;
    }
    call(io, "run", json!([command, false, 120, true]))?;
    let mut size = -1;
    for _ in 0..120 {
        call(io, "sleep", json!([0.5]))?;
        if call(io, "exists", json!([pdf]))? == true {
            let now = integer(&call(io, "size", json!([pdf]))?)?;
            if now != 0 && now == size {
                return Ok(());
            }
            size = now;
        }
    }
    Err(error(
        "SystemExit",
        format!("Edge wrote no PDF (is it at {edge}? pass --edge PATH)"),
    ))
}
pub fn count(data: &[u8], full: bool) -> usize {
    let (mut i, mut count) = (0, 0);
    while i + 5 <= data.len() {
        if &data[i..i + 5] != b"/Type" {
            i += 1;
            continue;
        }
        let mut j = i + 5;
        while j < data.len() && matches!(data[j], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            j += 1;
        }
        if data.get(j..j + 5) == Some(b"/Page") {
            let end = j + 5;
            let next = data.get(end).copied();
            let yes = if full {
                next.is_some_and(|b| b != b's')
            } else {
                next.is_none_or(|b| !(b.is_ascii_alphanumeric() || b == b'_'))
            };
            if yes {
                count += 1;
                i = end + usize::from(full);
                continue;
            }
        }
        i += 1;
    }
    count
}
pub fn report(path: &str, io: &mut impl Adapter) -> Result<(usize, Vec<String>)> {
    if call(io, "pdf_open", json!([path]))? != true {
        let b = raw_pdf(path, io)?;
        return Ok((count(&b, false), Vec::new()));
    }
    let n = pages(io)?;
    let mut firsts = Vec::with_capacity(n);
    for i in 0..n {
        let txt = call(io, "page_text", json!([i]))?;
        let txt = if txt.is_null() { "" } else { text(&txt)? };
        let first = lines(txt)
            .into_iter()
            .map(trim)
            .find(|s| !s.is_empty())
            .map(|s| s.chars().take(70).collect())
            .unwrap_or_else(|| "(no text)".into());
        firsts.push(first);
    }
    Ok((pages(io)?, firsts))
}
fn scalar(v: &Value) -> Result<f64> {
    number(v).and_then(|n| {
        if n.abs() <= 1e12 {
            Ok(n)
        } else {
            Err(Error::value("PDF coordinate exceeds supported bound"))
        }
    })
}
fn component(v: &Value, i: usize) -> Result<f64> {
    scalar(
        v.as_array()
            .and_then(|v| v.get(i))
            .ok_or_else(|| index_error(false))?,
    )
}
pub fn measure(path: &str, io: &mut impl Adapter) -> Result<()> {
    if call(io, "pdf_open", json!([path]))? != true {
        return Err(error(
            "SystemExit",
            "--measure needs pypdf (pip install pypdf)",
        ));
    }
    print(
        io,
        "section heights (mm; a page holds 273, the footer takes the last 6):",
    )?;
    let n = pages(io)?;
    let (mut carried, mut first) = (0.0, String::new());
    for i in 0..n {
        let metrics = call(io, "page_metrics", json!([i]))?;
        let h = scalar(&metrics[0])?;
        let data = metrics[1]
            .as_array()
            .ok_or_else(|| Error::value("expected PDF visitor items"))?;
        if data.len() > 100_000 {
            return Err(Error::value("too many PDF text items"));
        }
        let mut items = Vec::<(f64, String)>::new();
        for item in data {
            let t = trim(text(&item[0])?);
            if !t.is_empty() {
                let cm = &item[1];
                let tm = &item[2];
                let y = component(tm, 4)? * component(cm, 1)?
                    + component(tm, 5)? * component(cm, 3)?
                    + component(cm, 5)?;
                items.push((y, t.into()));
            }
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        if first.is_empty() {
            first = items
                .last()
                .map(|p| p.1.chars().take(50).collect())
                .unwrap_or_default();
        }
        let foot = items
            .iter()
            .find_map(|(y, t)| match matched(r"page \d+ of \d+", t) {
                Ok(true) => Some(Ok(*y)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            })
            .transpose()?;
        let Some(foot) = foot else {
            carried += 273.0;
            continue;
        };
        let bottom = items
            .iter()
            .map(|p| p.0)
            .filter(|y| *y > foot + 6.0)
            .min_by(|a, b| a.partial_cmp(b).unwrap());
        let used = bottom.map_or(0.0, |bottom| {
            (h - bottom - 12.0 * 72.0 / 25.4) / 72.0 * 25.4 + 3.0
        });
        let total = carried + used;
        print(
            io,
            format!(
                "  {total:5.0} mm  {first}{}",
                if total <= 267.0 {
                    String::new()
                } else {
                    format!("   OVER by {:.0}", total - 267.0)
                }
            ),
        )?;
        carried = 0.0;
        first.clear();
    }
    Ok(())
}
fn edge(cfg: &Value) -> Result<&str> {
    text(if cfg["edge_env"].is_null() {
        &cfg["edge"]
    } else {
        &cfg["edge_env"]
    })
}
pub fn quick_main(argv: &Value, cfg: &Value, io: &mut impl Adapter) -> Result<i32> {
    let root = text(&cfg["src"])?;
    let mut md = super::super::dy::join(root, &["docs", "QUICK.md"]);
    let (mut html_only, mut do_measure, mut edge, mut stem, mut font) = (
        false,
        false,
        edge(cfg)?.to_owned(),
        "quick-manual".to_owned(),
        None,
    );
    let mut args: VecDeque<_> = string_list(argv)?.into();
    while let Some(a) = args.pop_front() {
        match a {
            "--html-only" => html_only = true,
            "--measure" => do_measure = true,
            "--edge" => edge = args.pop_front().ok_or_else(|| index_error(true))?.into(),
            "--out" => stem = args.pop_front().ok_or_else(|| index_error(true))?.into(),
            "--font" => {
                font = Some(
                    args.pop_front()
                        .ok_or_else(|| index_error(true))?
                        .to_owned(),
                )
            }
            _ => md = a.into(),
        }
    }
    let dir = super::super::dy::join(root, &["build"]);
    call(io, "mkdir", json!([dir]))?;
    let html = super::super::dy::join(&dir, &[&(stem.clone() + ".html")]);
    let pdf = super::super::dy::join(&dir, &[&(stem.clone() + ".pdf")]);
    let img = call(io, "resolve_parent", json!([md]))?;
    call(io, "set_img_base", json!([img]))?;
    let mut config = cfg.clone();
    config["img_base"] = img;
    let input = call(io, "read_text", json!([md]))?;
    let mut body = quick::convert(text(&input)?, &config, io)?;
    if let Some(font) = font.filter(|f| !f.is_empty()) {
        body = body.replacen(
            "html { font-size: 9pt; }",
            &format!("html {{ font-size: {font}; }}"),
            1,
        );
    }
    call(io, "write_text", json!([html, body, "lf"]))?;
    let len = integer(&call(io, "size", json!([html]))?)?;
    print(io, format!("wrote {html} ({len} bytes)"))?;
    if html_only {
        return Ok(0);
    }
    to_pdf(&html, &pdf, &edge, io)?;
    let (n, firsts) = report(&pdf, io)?;
    let len = integer(&call(io, "size", json!([pdf]))?)?;
    print(io, format!("wrote {pdf} ({len} bytes)"))?;
    for (i, f) in firsts.iter().enumerate() {
        print(io, format!("  page {}: {f}", i + 1))?;
    }
    let want = integer(&cfg["want_pages"])?;
    print(
        io,
        format!(
            "pages: {n} {}",
            if n as i64 == want {
                String::new()
            } else {
                format!("(wanted {want})")
            }
        ),
    )?;
    if do_measure {
        measure(&pdf, io)?;
    }
    Ok(if n as i64 == want || stem != "quick-manual" {
        0
    } else {
        1
    })
}
pub fn full_main(argv: &Value, cfg: &Value, io: &mut impl Adapter) -> Result<i32> {
    let args = string_list(argv)?;
    let mut edge = edge(cfg)?.to_owned();
    if let Some(i) = args.iter().position(|s| *s == "--edge") {
        edge = args
            .get(i + 1)
            .ok_or_else(|| index_error(false))?
            .to_string();
    }
    let script = super::super::dy::join(text(&cfg["here"])?, &["manual_html.py"]);
    call(
        io,
        "run",
        json!([
            [text(&cfg["python"])?, "-I", script],
            true,
            Value::Null,
            false
        ]),
    )?;
    let root = text(&cfg["src"])?;
    let pdf = super::super::dy::join(root, &["build", "manual.pdf"]);
    let html = super::super::dy::join(root, &["docs", "manual.html"]);
    to_pdf(&html, &pdf, &edge, io)?;
    let data = raw_pdf(&pdf, io)?;
    let n = count(&data, true);
    print(
        io,
        format!("wrote {pdf} ({} bytes, about {n} pages)", data.len()),
    )?;
    Ok(if n > 10 { 0 } else { 1 })
}
