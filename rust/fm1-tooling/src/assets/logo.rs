// SPDX-License-Identifier: GPL-3.0-only
//! Source sprite, RGBA geometry and inline asset policy; PNG is adapter I/O.
use crate::{bytes, text, Adapter, Error, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub fn palette() -> Value {
    serde_json::from_str(include_str!("../../../../tools/data/logo-palette.json"))
        .expect("checked logo palette")
}
fn error(kind: &str, message: &str) -> Error {
    Error {
        kind: kind.into(),
        code: String::new(),
        message: message.into(),
    }
}
pub fn sprite(source: &str) -> Result<Vec<String>> {
    if source.len() > 16 * 1024 * 1024 {
        return Err(Error::value("sprite source exceeds supported bound"));
    }
    let part = source
        .split_once("static const char *const MASCOT[23] = {")
        .and_then(|(_, s)| s.split_once("};"))
        .map(|(s, _)| s)
        .ok_or_else(|| {
            error(
                "AttributeError",
                "'NoneType' object has no attribute 'group'",
            )
        })?;
    let mut rest = part;
    let mut rows = Vec::new();
    while let Some((_, s)) = rest.split_once('"') {
        if let Some((row, s)) = s.split_once('"') {
            rows.push(row.to_owned());
            if rows.len() > 23 {
                return Err(super::assertion("the sprite changed shape"));
            }
            rest = s;
        } else {
            break;
        }
    }
    if rows.len() != 23 || rows.iter().any(|r| r.chars().count() != 23) {
        return Err(super::assertion("the sprite changed shape"));
    }
    Ok(rows)
}
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}
impl Image {
    pub fn value(self) -> Value {
        json!([self.width, self.height, self.rgba])
    }
}
pub fn render(rows: &Value, scale: i64, size: Option<i64>, colors: &Value) -> Result<Image> {
    if scale < 0 || size.is_some_and(|v| v < 0) {
        return Err(Error::value("Width and height must be >= 0"));
    }
    if scale > 16 || size.is_some_and(|v| v > 512) {
        return Err(Error::value("logo dimensions exceed supported bound"));
    }
    let rows = rows
        .as_array()
        .ok_or_else(|| Error::value("expected sprite rows"))?;
    if rows.len() > 64 {
        return Err(Error::value("too many sprite rows"));
    }
    let mut pal = BTreeMap::new();
    let colors = colors
        .as_object()
        .ok_or_else(|| Error::value("expected sprite palette"))?;
    if colors.len() > 256 {
        return Err(Error::value("too many sprite colors"));
    }
    for (k, v) in colors {
        let c = bytes(v)?;
        if c.len() != 3 || k.chars().count() != 1 {
            return Err(Error::value("expected single-character RGB palette"));
        }
        pal.insert(k.chars().next().unwrap(), [c[0], c[1], c[2], 255]);
    }
    let scale = scale as usize;
    let w = 23 * scale;
    let mut rgba = vec![0; w * w * 4];
    for (y, row) in rows.iter().enumerate() {
        let row = text(row)?;
        if row.chars().count() > 64 {
            return Err(Error::value("sprite row exceeds supported bound"));
        }
        for (x, c) in row.chars().enumerate() {
            if let Some(color) = pal.get(&c) {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (px, py) = (x * scale + dx, y * scale + dy);
                        if px >= w || py >= w {
                            return Err(error("IndexError", "image index out of range"));
                        }
                        rgba[(py * w + px) * 4..(py * w + px + 1) * 4].copy_from_slice(color);
                    }
                }
            }
        }
    }
    if let Some(size) = size.filter(|v| *v != 0) {
        let size = size as usize;
        let mut out = vec![0; size * size * 4];
        let origin = (size as i64 - w as i64).div_euclid(2);
        for y in 0..w {
            for x in 0..w {
                let (ox, oy) = (x as i64 + origin, y as i64 + origin);
                if ox >= 0 && oy >= 0 && ox < size as i64 && oy < size as i64 {
                    let at = (oy as usize * size + ox as usize) * 4;
                    out[at..at + 4].copy_from_slice(&rgba[(y * w + x) * 4..(y * w + x + 1) * 4]);
                }
            }
        }
        Ok(Image {
            width: size,
            height: size,
            rgba: out,
        })
    } else {
        Ok(Image {
            width: w,
            height: w,
            rgba,
        })
    }
}
pub(super) fn b64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            out.push(if i > c.len() {
                '='
            } else {
                char::from(TABLE[((n >> (18 - i * 6)) & 63) as usize])
            });
        }
    }
    out
}
fn replace(source: &str, prefix: &str, value: &str) -> Result<String> {
    fn append(out: &mut String, s: &str) -> Result<()> {
        if out.len().saturating_add(s.len()) > 16 * 1024 * 1024 {
            return Err(Error::value("inline asset output exceeds supported bound"));
        }
        out.push_str(s);
        Ok(())
    }
    let mut out = String::new();
    let mut rest = source;
    while let Some((before, after)) = rest.split_once(prefix) {
        append(&mut out, before)?;
        append(&mut out, prefix)?;
        if let Some((_, suffix)) = after.split_once('"') {
            append(&mut out, value)?;
            append(&mut out, "\"")?;
            rest = suffix;
        } else {
            append(&mut out, after)?;
            return Ok(out);
        }
    }
    append(&mut out, rest)?;
    Ok(out)
}
pub fn inline(source: &str, logo: &[u8], fav: &[u8]) -> Result<String> {
    let logo = "data:image/png;base64,".to_owned() + &b64(logo);
    let fav = "data:image/png;base64,".to_owned() + &b64(fav);
    replace(
        &replace(source, "<img class=\"logo\" src=\"", &logo)?,
        "<link rel=\"icon\" type=\"image/png\" href=\"",
        &fav,
    )
}
fn call(io: &mut impl Adapter, kind: &str, args: Value) -> Result<Value> {
    io.action(json!({"kind":kind,"args":args}))
}
fn png(io: &mut impl Adapter, image: Image) -> Result<Vec<u8>> {
    bytes(&call(io, "png", image.value())?)
}
pub fn generate(
    io: &mut impl Adapter,
    argv: &Value,
    here: &str,
    ui: &str,
    colors: &Value,
) -> Result<()> {
    let rows = sprite(text(&call(io, "read_text", json!([ui]))?)?)?;
    let rows = json!(rows);
    let logo = png(io, render(&rows, 4, None, colors)?)?;
    let fav = png(io, render(&rows, 1, Some(32), colors)?)?;
    let image_dir = super::dy::join(here, &["..", "web", "img"]);
    call(io, "mkdir", json!([image_dir]))?;
    call(
        io,
        "write",
        json!([super::dy::join(&image_dir, &["slinky.png"]), logo]),
    )?;
    call(
        io,
        "write",
        json!([super::dy::join(&image_dir, &["favicon.png"]), fav]),
    )?;
    call(
        io,
        "print",
        json!([format!(
            "slinky.png {} B, favicon.png {} B",
            logo.len(),
            fav.len()
        )]),
    )?;
    if argv
        .as_array()
        .ok_or_else(|| Error::value("expected logo arguments"))?
        .iter()
        .any(|v| v == "--inline")
    {
        for name in [
            &["..", "web", "amb_installer.html"][..],
            &["..", "web", "editor", "index.html"][..],
        ] {
            let path = super::dy::join(here, name);
            let source = call(io, "read_text", json!([path]))?;
            let out = inline(text(&source)?, &logo, &fav)?;
            call(io, "write_text", json!([path, out]))?;
            call(io, "print", json!([format!("inlined into {path}")]))?;
        }
    }
    Ok(())
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "palette" => palette(),
        "sprite" => json!(sprite(text(&a[0])?)?),
        "render" => render(
            &a[0],
            a[1].as_i64()
                .ok_or_else(|| Error::value("expected logo scale"))?,
            if a[2].is_null() {
                None
            } else {
                Some(
                    a[2].as_i64()
                        .ok_or_else(|| Error::value("expected logo size"))?,
                )
            },
            &a[3],
        )?
        .value(),
        "inline" => json!(inline(text(&a[0])?, &bytes(&a[1])?, &bytes(&a[2])?)?),
        "main" => {
            generate(io, &a[0], text(&a[1])?, text(&a[2])?, &a[3])?;
            Value::Null
        }
        _ => return Err(Error::value("unknown logo asset method")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_geometry_stops_before_pixel_allocation() {
        assert!(render(&json!([]), 17, None, &palette()).is_err());
        assert!(render(&json!([]), 1, Some(i64::MAX), &palette()).is_err());
        assert!(render(&json!(vec![""; 65]), 1, None, &palette()).is_err());
    }
    #[test]
    fn zero_size_and_floor_centering_keep_transparent_crop() {
        let a = render(&json!(["K"]), 1, Some(0), &palette()).unwrap();
        assert_eq!(a.width, 23);
        let b = render(&json!(["K"]), 1, Some(2), &palette()).unwrap();
        assert_eq!(b.rgba, vec![0; 16]);
        assert_eq!(
            inline("<img class=\"logo\" src=\"", b"", b"").unwrap(),
            "<img class=\"logo\" src=\""
        );
        assert!(replace(&"p\"".repeat(1100), "p", &"x".repeat(16384)).is_err());
    }
}
