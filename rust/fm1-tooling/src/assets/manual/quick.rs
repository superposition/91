// SPDX-License-Identifier: GPL-3.0-only
use super::*;
pub fn inline(input: &str) -> Result<String> {
    let mut codes = Vec::new();
    let mut s = sub(r"`([^`]+)`", input, |c| {
        codes.push(format!("<code>{}</code>", escape(&c[1], true)));
        Ok(format!("\u{0}{}\u{0}", codes.len() - 1))
    })?;
    s = escape(&s, false);
    s = sub(r"\*\*(?=\S)(.+?)(?<=\S)\*\*", &s, |c| {
        Ok(format!("<strong>{}</strong>", &c[1]))
    })?;
    s = sub(r"(?<![\w*])\*(?=\S)([^*]+?)(?<=\S)\*(?![\w*])", &s, |c| {
        Ok(format!("<em>{}</em>", &c[1]))
    })?;
    s = sub(r"\[([^\]]+)\]\(([^)\s]+)\)", &s, |c| {
        Ok(format!("<a href=\"{}\">{}</a>", &c[2], &c[1]))
    })?;
    sub(r"\x00(\d+)\x00", &s, |c| {
        c[1].parse::<usize>()
            .ok()
            .and_then(|i| codes.get(i))
            .cloned()
            .ok_or_else(|| error("IndexError", "list index out of range"))
    })
}
pub fn table(input: &[&str]) -> Result<String> {
    let rows: Vec<Vec<&str>> = input
        .iter()
        .map(|s| {
            let s = trim(s);
            let s = s.strip_prefix('|').unwrap_or(s);
            let s = s.strip_suffix('|').unwrap_or(s);
            s.split('|').map(trim).collect()
        })
        .collect();
    let headed = rows.len() >= 2
        && rows[1]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|c| matched(r"^:?-{2,}:?$", c))
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|v| *v);
    let mut out = Output::default();
    out.push("<table>".to_owned())?;
    let cells = |row: &[&str], tag: &str| -> Result<String> {
        join_checked(
            row.iter()
                .map(|c| Ok(format!("<{tag}>{}</{tag}>", inline(c)?))),
            "",
        )
    };
    if headed && !rows[0].is_empty() {
        out.push(format!(
            "<thead><tr>{}</tr></thead>",
            cells(&rows[0], "th")?
        ))?;
    }
    out.push("<tbody>".into())?;
    for row in &rows[if headed { 2 } else { 0 }..] {
        out.push(format!("<tr>{}</tr>", cells(row, "td")?))?;
    }
    out.push("</tbody></table>".into())?;
    let s = out.finish();
    checked(&s)?;
    Ok(s)
}
pub fn blocks(input: &str, cfg: &Value, io: &mut impl Adapter) -> Result<String> {
    checked(input)?;
    let rows: Vec<_> = input.split('\n').collect();
    let (mut out, mut i) = (Output::default(), 0);
    while i < rows.len() {
        let row = rows[i];
        let s = trim(row);
        if s.is_empty() {
            i += 1;
            continue;
        }
        if s.starts_with("<!--") {
            while i < rows.len() && !rows[i].contains("-->") {
                i += 1;
            }
            i += 1;
            continue;
        }
        if s.starts_with('<') {
            out.push(s.into())?;
            i += 1;
            continue;
        }
        if let Some(c) = groups(r"^(#{1,3})\s+(.*)", s)? {
            let n = c[1].len();
            out.push(format!("<h{n}>{}</h{n}>", inline(&c[2])?))?;
            i += 1;
            continue;
        }
        if s.starts_with('|') {
            let mut j = i;
            while j < rows.len() && trim(rows[j]).starts_with('|') {
                j += 1;
            }
            out.push(table(&rows[i..j])?)?;
            i = j;
            continue;
        }
        if matched(r"^[-*]\s+", s)? || matched(r"^\d+\.\s+", s)? {
            let ordered = matched(r"^\d+\.", s)?;
            let mut items = Vec::<String>::new();
            let mut j = i;
            while j < rows.len() {
                let t = trim(rows[j]);
                if matched(if ordered { r"^\d+\.\s+" } else { r"^[-*]\s+" }, t)? {
                    items.push(sub(r"^(\d+\.|[-*])\s+", t, |_| Ok(String::new()))?);
                } else if !t.is_empty() && rows[j].starts_with(' ') && !items.is_empty() {
                    let last = items.last_mut().unwrap();
                    last.push(' ');
                    last.push_str(t);
                } else {
                    break;
                }
                j += 1;
            }
            let tag = if ordered { "ol" } else { "ul" };
            out.push(format!(
                "<{tag}>{}</{tag}>",
                join_checked(
                    items.iter().map(|s| Ok(format!("<li>{}</li>", inline(s)?))),
                    ""
                )?
            ))?;
            i = j;
            continue;
        }
        if let Some(c) = groups(r"^!\[([^\]]*)\]\(([^)]+)\)\s*$", s)? {
            let src = if !cfg["img_base"].is_null() {
                let path = super::super::dy::join(text(&cfg["img_base"])?, &[&c[2]]);
                let uri = call(io, "resolved_uri", json!([path]))?;
                text(&uri)?.to_owned()
            } else {
                c[2].clone()
            };
            out.push(format!(
                "<figure><img src=\"{src}\" alt=\"{}\"><figcaption>{}</figcaption></figure>",
                escape(&c[1], true),
                inline(&c[1])?
            ))?;
            i += 1;
            continue;
        }
        let mut j = i;
        while j < rows.len()
            && !trim(rows[j]).is_empty()
            && !matched(r"^(#{1,3}\s|\||[-*]\s|\d+\.\s|<)", trim(rows[j]))?
        {
            j += 1;
        }
        if j == i {
            return Err(Error::value("unsupported non-progressing Markdown block"));
        }
        out.push(format!(
            "<p>{}</p>",
            inline(
                &rows[i..j]
                    .iter()
                    .map(|s| trim(s))
                    .collect::<Vec<_>>()
                    .join(" ")
            )?
        ))?;
        i = j;
    }
    let s = out.finish();
    checked(&s)?;
    Ok(s)
}
pub fn convert(input: &str, cfg: &Value, io: &mut impl Adapter) -> Result<String> {
    checked(input)?;
    let mut md = input.to_owned();
    let mut foot = String::new();
    if let Some(c) = re(r"(?s)<!--\s*footer:\s*(.*?)\s*-->")?
        .captures(&md)
        .map_err(|e| Error::value(e.to_string()))?
    {
        foot = escape(&c[1], false);
        let m = c.get(0).unwrap();
        md = format!("{}{}", &md[..m.start()], &md[m.end()..]);
    }
    let delimiter = re(r#"(?m)^(?:---+|<div class="page"></div>)\s*$"#)?;
    let mut pages = Vec::new();
    let mut at = 0;
    for m in delimiter.find_iter(&md) {
        let m = m.map_err(|e| Error::value(e.to_string()))?;
        if !trim(&md[at..m.start()]).is_empty() {
            pages.push(&md[at..m.start()]);
        }
        at = m.end();
    }
    if !trim(&md[at..]).is_empty() {
        pages.push(&md[at..]);
    }
    let mut body = format!(
        "<main>\n{}\n</main>",
        join_checked(
            pages.iter().map(|p| Ok(format!(
                "<section class=\"page\">\n{}\n</section>",
                blocks(p, cfg, io)?
            ))),
            "\n"
        )?
    );
    let fcss = foot.replace('\\', "\\\\").replace('"', "\\\"");
    body = text(&cfg["copy"]["footer_style"])?.replacen("%s", &fcss, 1) + &body;
    let title = if let Some(c) = groups(r"(?m)^#\s+(.*)$", &md)? {
        escape(&c[1], true)
    } else {
        text(&cfg["copy"]["quick_fallback_title"])?.into()
    };
    let template = text(&cfg["copy"]["quick_document"])?;
    let mut values = [title, text(&cfg["css"])?.into(), body].into_iter();
    let page = sub("%s", template, |_| {
        values
            .next()
            .ok_or_else(|| Error::value("invalid quick document template"))
    })?;
    checked(&page)?;
    Ok(page)
}
