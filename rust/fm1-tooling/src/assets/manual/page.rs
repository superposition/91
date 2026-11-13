// SPDX-License-Identifier: GPL-3.0-only
use super::*;
pub fn inline(input: &str, colors: &Value) -> Result<String> {
    let mut s = escape(input, false);
    s = sub(r"`([^`]+)`", &s, |c| Ok(format!("<code>{}</code>", &c[1])))?;
    s = sub(r"\*\*([^*]+)\*\*", &s, |c| {
        Ok(format!("<strong>{}</strong>", &c[1]))
    })?;
    s = sub(r"(?<![\w*])\*([^*\s][^*]*)\*(?![\w*])", &s, |c| {
        Ok(format!("<em>{}</em>", &c[1]))
    })?;
    s = sub(r"\[([^\]]+)\]\(([^)\s]+)\)", &s, |c| {
        Ok(format!("<a href=\"{}\">{}</a>", &c[2], &c[1]))
    })?;
    if let Some(colors) = colors.as_object() {
        if colors.len() > 128 {
            return Err(Error::value("too many manual swatches"));
        }
        for (name, col) in colors {
            s = sub(&format!("^<strong>{name}</strong>"), &s, |_| {
                Ok(format!(
                    "<span class=\"swatch\" style=\"background:{}\"></span><strong>{name}</strong>",
                    text(col)?
                ))
            })?;
        }
    }
    Ok(s)
}
pub fn slug(title: &str) -> Result<String> {
    if let Some(c) = groups(r"^(\d+)\.", title)? {
        Ok(format!("s{}", &c[1]))
    } else {
        Ok(
            sub(r"[^a-z0-9]+", &title.to_lowercase(), |_| Ok("-".into()))?
                .trim_matches('-')
                .into(),
        )
    }
}
pub fn figure(caption: &str, src: &str, cfg: &Value, io: &mut impl Adapter) -> Result<String> {
    let url = if cfg["embed"] == true {
        let path = super::super::dy::join(text(&cfg["img_base"])?, &[src]);
        let data = bytes(&call(io, "read", json!([path]))?)?;
        format!(
            "data:{};base64,{}",
            if src.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "image/png"
            },
            b64(&data)
        )
    } else {
        src.into()
    };
    let class = if src.ends_with(".svg") || matched(r"hyp-(device|hero|styles|themes)", src)? {
        "wide"
    } else {
        "shot"
    };
    Ok(format!("<figure class=\"{class}\"><img src=\"{url}\" alt=\"{}\" loading=\"lazy\"><figcaption>{}</figcaption></figure>",escape(caption,true),inline(caption,&cfg["part_colors"])?))
}
pub fn convert(md: &str, cfg: &Value, io: &mut impl Adapter) -> Result<String> {
    checked(md)?;
    let rows = lines(md);
    let (mut out, mut i, mut contents, mut sections) =
        (Output::default(), 0, false, BTreeMap::new());
    for row in &rows {
        if let Some(c) = groups(r"^## (\d+)\. (.+)", row)? {
            sections.insert(trim(&c[2]).to_owned(), format!("s{}", &c[1]));
        }
    }
    let name = text(&cfg["name"])?;
    let cp = &cfg["copy"];
    let colors = &cfg["part_colors"];
    while i < rows.len() {
        let row = rows[i];
        if row.starts_with("```") {
            let mut j = i + 1;
            while j < rows.len() && !rows[j].starts_with("```") {
                j += 1;
            }
            out.push(format!(
                "<pre><code>{}</code></pre>",
                escape(&rows[i + 1..j].join("\n"), false)
            ))?;
            i = j + 1;
            continue;
        }
        if row.starts_with("![") {
            let mut figs = String::new();
            while i < rows.len()
                && (rows[i].starts_with("![")
                    || (trim(rows[i]).is_empty()
                        && i + 1 < rows.len()
                        && rows[i + 1].starts_with("![")))
            {
                if let Some(c) = groups(r"^!\[(.*)\]\((.*)\)", rows[i])? {
                    append(&mut figs, &figure(&c[1], &c[2], cfg, io)?)?;
                }
                i += 1;
            }
            out.push(format!("<div class=\"figs\">{figs}</div>"))?;
            continue;
        }
        if trim(row).starts_with("<details>") {
            let c = groups(r"^\s*<details>\s*<summary>(.*)</summary>\s*$", row)?;
            out.push(format!(
                "<details><summary>{}</summary>",
                if let Some(c) = c {
                    inline(&c[1], colors)?
                } else {
                    String::new()
                }
            ))?;
            i += 1;
            continue;
        }
        if trim(row) == "</details>" {
            out.push("</details>".into())?;
            i += 1;
            continue;
        }
        if row.starts_with("> ") {
            let mut para = Vec::new();
            while i < rows.len() && rows[i].starts_with('>') {
                para.push(trim(&rows[i][1..]));
                i += 1;
            }
            out.push(format!(
                "<div class=\"callout\"><p>{}</p></div>",
                inline(&para.join(" "), colors)?
            ))?;
            continue;
        }
        if let Some(t) = row.strip_prefix("# ") {
            let t = trim(t);
            if t == name {
                out.push(text(&cp["site_header"])?.into())?;
                out.push(text(&cp["title_manual"])?.into())?;
            } else {
                out.push(format!(
                    "<a class=\"home\" href=\"../\">{name}</a>\n<h1>{}</h1>",
                    inline(t, colors)?
                ))?;
            }
            out.push(text(&cp["printable"])?.into())?;
        } else if let Some(t) = row.strip_prefix("## ") {
            let t = trim(t);
            let c = groups(r"^(\d+)\. (.+)", t)?;
            if t == "Contents" {
                contents = true;
                out.push(text(&cp["contents_open"])?.into())?;
            } else if let Some(c) = c {
                out.push(format!(
                    "<h2 id=\"s{}\"><span class=\"num\">{}</span>{}</h2>",
                    &c[1],
                    &c[1],
                    inline(&c[2], colors)?
                ))?;
            } else {
                out.push(format!(
                    "<h2 id=\"{}\">{}</h2>",
                    slug(t)?,
                    inline(t, colors)?
                ))?;
            }
        } else if let Some(t) = row.strip_prefix("### ") {
            out.push(format!(
                "<h3 id=\"{}\">{}</h3>",
                slug(t)?,
                inline(t, colors)?
            ))?;
        } else if trim(row) == "---" {
            if contents {
                out.push("</nav>".into())?;
                contents = false;
            }
            out.push("<hr>".into())?;
        } else if row.starts_with('|') {
            let mut table = Vec::<Vec<&str>>::new();
            while i < rows.len() && rows[i].starts_with('|') {
                table.push(
                    trim(rows[i])
                        .trim_matches('|')
                        .split('|')
                        .map(trim)
                        .collect(),
                );
                i += 1;
            }
            let cells = |row: &[&str], tag: &str| -> Result<String> {
                join_checked(
                    row.iter()
                        .map(|c| Ok(format!("<{tag}>{}</{tag}>", inline(c, colors)?))),
                    "",
                )
            };
            let body = join_checked(
                table[1..]
                    .iter()
                    .filter(|r| !r.join("").chars().all(|c| matches!(c, '-' | ':' | ' ')))
                    .map(|r| Ok(format!("<tr>{}</tr>", cells(r, "td")?))),
                "",
            )?;
            out.push(format!("<div class=\"table\"><table><thead><tr>{}</tr></thead><tbody>{body}</tbody></table></div>",cells(&table[0],"th")?))?;
            continue;
        } else if matched(r"^(\d+\.|-) ", row)? {
            let ordered = !row.starts_with('-');
            let mut items = Vec::<String>::new();
            while i < rows.len()
                && (matched(r"^(\d+\.|-) ", rows[i])?
                    || (rows[i].starts_with("  ") && !items.is_empty()))
            {
                if rows[i].starts_with("  ") {
                    let last = items.last_mut().unwrap();
                    last.push(' ');
                    last.push_str(trim(rows[i]));
                } else {
                    items.push(sub(r"^(\d+\.|-) ", rows[i], |_| Ok(String::new()))?);
                }
                i += 1;
            }
            let items = join_checked(
                items.iter().map(|s| {
                    let body = if contents {
                        format!(
                            "<a href=\"#{}\">{}</a>",
                            sections.get(trim(s)).cloned().unwrap_or(slug(s)?),
                            inline(s, colors)?
                        )
                    } else {
                        inline(s, colors)?
                    };
                    Ok(format!("<li>{body}</li>"))
                }),
                "",
            )?;
            let tag = if ordered { "ol" } else { "ul" };
            out.push(format!("<{tag}>{items}</{tag}>"))?;
            continue;
        } else if !trim(row).is_empty() {
            let mut para = vec![row];
            while i + 1 < rows.len()
                && !trim(rows[i + 1]).is_empty()
                && !matched(
                    r"^(#|\||```|---|\d+\. |- |!\[|> |\s*</?details)",
                    rows[i + 1],
                )?
            {
                i += 1;
                para.push(rows[i]);
            }
            let body = inline(
                &para.iter().map(|s| trim(s)).collect::<Vec<_>>().join(" "),
                colors,
            )?;
            out.push(if body.starts_with("<strong>Status.</strong>") {
                format!("<div class=\"status\"><p>{body}</p></div>")
            } else if out.last().is_some_and(|s| s.starts_with("<h1>")) {
                format!("<p class=\"lede\">{body}</p>")
            } else {
                format!("<p>{body}</p>")
            })?;
        }
        i += 1;
    }
    let result = out.finish();
    checked(&result)?;
    Ok(result)
}
pub fn main(argv: &Value, cfg: &Value, io: &mut impl Adapter) -> Result<()> {
    let argv = string_list(argv)?;
    let args: Vec<_> = argv
        .iter()
        .copied()
        .filter(|s| *s != "--fragment")
        .collect();
    let fragment = argv.contains(&"--fragment");
    call(io, "set_embed", json!([fragment]))?;
    let root = text(&cfg["src"])?;
    let src = args
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| super::super::dy::join(root, &["docs", "MANUAL.md"]));
    let dst = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| super::super::dy::join(root, &["docs", "manual.html"]));
    let md = call(io, "read_text", json!([src]))?;
    let md = text(&md)?;
    let name = text(&cfg["name"])?;
    let h1 = groups(r"(?m)^# (.+)$", md)?;
    let (title, what) = match h1 {
        Some(c) if c[1] != name => (format!("{name}: {}", &c[1]), c[1].clone()),
        _ => (
            format!("{name} {}", text(&cfg["copy"]["manual_title_suffix"])?),
            text(&cfg["copy"]["manual_default_description"])?.into(),
        ),
    };
    let mut config = cfg.clone();
    config["embed"] = json!(fragment);
    let body = convert(md, &config, io)?;
    let mut page=format!("<title>{}</title>\n<meta name=\"description\" content=\"{} for {name}, {}\">\n{}\n\n<style>{}</style>\n<main>\n{body}\n</main>\n",escape(&title,true),escape(&what,true),text(&cfg["copy"]["manual_description_tail"])?,text(&cfg["copy"]["font_link"])?,text(&cfg["css"])?);
    if !fragment {
        let (head, rest) = page.split_once("<main>").unwrap();
        page=format!("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n{head}</head>\n<body>\n<main>{rest}</body>\n</html>\n");
    }
    checked(&page)?;
    call(io, "write_text", json!([dst, page, "native"]))?;
    print(io, format!("{dst}: {} bytes", page.chars().count()))
}
