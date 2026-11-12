// SPDX-License-Identifier: GPL-3.0-only
// Font layout/packing derived from tools/gen_font.py, Leo Kuroshita and contributors.
use crate::{bytes, number, text, Adapter, Error, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
const PAD: usize = 2;
const LIMIT: usize = 1_100_000;
#[derive(Clone, Debug)]
pub struct Glyph {
    pub advance: i64,
    pub width: usize,
    pub pixels: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Font {
    pub height: usize,
    pub glyphs: Vec<Glyph>,
}
pub struct Search<'a> {
    pub fonts: &'a str,
    pub directories: &'a Value,
}
pub struct Spec<'a> {
    pub file: &'a str,
    pub px: i64,
    pub scale: usize,
    pub pixel: bool,
    pub first: u32,
    pub last: u32,
}
fn error(kind: &str, message: impl Into<String>) -> Error {
    Error {
        kind: kind.into(),
        code: String::new(),
        message: message.into(),
    }
}
fn integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .filter(|n| n.unsigned_abs() <= 1 << 40)
        .ok_or_else(|| Error::value("expected font integer"))
}
fn size(v: &Value, max: usize) -> Result<usize> {
    integer(v).and_then(|n| {
        usize::try_from(n)
            .ok()
            .filter(|n| *n <= max)
            .ok_or_else(|| Error::value("font size outside supported bound"))
    })
}
fn action(io: &mut impl Adapter, kind: &str, args: Value) -> Result<Value> {
    io.action(json!({"kind":kind,"args":args}))
}
fn warn(io: &mut impl Adapter, line: String) -> Result<()> {
    action(io, "print", json!([line]))?;
    Ok(())
}
fn ascii_repr(s: &str) -> String {
    crate::installer::repr(s)
        .chars()
        .map(|c| {
            if c < ' ' || c == '\u{7f}' {
                format!("\\x{:02x}", u32::from(c))
            } else {
                c.to_string()
            }
        })
        .collect()
}
fn plane(w: usize, h: usize) -> Result<usize> {
    w.checked_mul(h)
        .filter(|n| *n <= LIMIT)
        .ok_or_else(|| Error::value("font plane exceeds supported bound"))
}
pub fn upscale(pixels: &[u8], w: usize, h: usize, scale: usize) -> Result<Vec<u8>> {
    if scale == 1 {
        return Ok(pixels.to_vec());
    }
    let n = plane(w, h)?;
    if pixels.len() < n {
        return Err(error("IndexError", "list index out of range"));
    }
    let sw = w
        .checked_mul(scale)
        .ok_or_else(|| Error::value("font width overflow"))?;
    let sh = h
        .checked_mul(scale)
        .ok_or_else(|| Error::value("font height overflow"))?;
    plane(sw, sh)?;
    let mut out = Vec::with_capacity(sw * sh);
    for y in 0..h {
        let mut row = Vec::with_capacity(sw);
        for x in 0..w {
            row.extend(std::iter::repeat_n(pixels[y * w + x], scale));
        }
        for _ in 0..scale {
            out.extend(&row);
        }
    }
    Ok(out)
}
fn parsed(line: &str, count: usize) -> Result<Vec<i64>> {
    line.split_whitespace()
        .skip(1)
        .take(count)
        .map(|s| {
            s.parse::<i64>()
                .ok()
                .filter(|n| n.unsigned_abs() <= 1 << 40)
                .ok_or_else(|| {
                    Error::value(format!(
                        "invalid literal for int() with base 10: {}",
                        crate::installer::repr(s)
                    ))
                })
        })
        .collect()
}
fn dim(n: i64) -> Result<usize> {
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= 2048)
        .ok_or_else(|| Error::value("font dimension outside supported bound"))
}
fn row_bits(row: &str, nbits: usize) -> Result<Vec<bool>> {
    let invalid = || {
        Error::value(format!(
            "invalid literal for int() with base 16: {}",
            crate::installer::repr(row)
        ))
    };
    let s = row.trim();
    let (negative, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let s = if let Some(s) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        s.strip_prefix('_').unwrap_or(s)
    } else {
        s
    };
    if s.is_empty() || s.starts_with('_') || s.ends_with('_') || s.contains("__") {
        return Err(invalid());
    }
    let digits: Vec<_> = s
        .bytes()
        .filter(|c| *c != b'_')
        .map(|c| char::from(c).to_digit(16).ok_or_else(invalid))
        .collect::<Result<_>>()?;
    let mut out = Vec::with_capacity(nbits);
    // Python shifts a signed arbitrary-precision integer. For negative rows,
    // form its low nbits in two's complement without narrowing the source.
    let mut carry = negative;
    for bit in 0..nbits {
        let set = digits
            .len()
            .checked_sub(1 + bit / 4)
            .and_then(|i| digits.get(i))
            .is_some_and(|d| d >> (bit % 4) & 1 != 0);
        if negative {
            let inverted = !set;
            out.push(inverted ^ carry);
            carry = inverted && carry;
        } else {
            out.push(set);
        }
    }
    Ok(out)
}
pub fn bdf(
    source: &str,
    path: &str,
    scale: usize,
    first: u32,
    last: u32,
    io: &mut impl Adapter,
) -> Result<Font> {
    if source.len() > 16 * 1024 * 1024 || (last >= first && last - first > 4096) || scale > 16 {
        return Err(Error::value("font input outside supported bounds"));
    }
    let lines: Vec<_> = source.lines().collect();
    let fbb = lines
        .iter()
        .find(|l| l.starts_with("FONTBOUNDINGBOX"))
        .ok_or_else(|| error("StopIteration", ""))?;
    let bb = parsed(fbb, 4)?;
    if bb.len() < 4 {
        return Err(error("IndexError", "list index out of range"));
    }
    let (cw, ch) = (dim(bb[0])?, dim(bb[1])?);
    plane(cw, ch)?;
    let base = ch as i64 + bb[3];
    let mut glyphs: BTreeMap<i64, (i64, Vec<u8>)> = BTreeMap::new();
    let mut stored = 0;
    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with("ENCODING") {
            let code = *parsed(lines[i], 1)?
                .first()
                .ok_or_else(|| error("IndexError", "list index out of range"))?;
            let j = (i..lines.len())
                .find(|j| lines[*j].starts_with("BBX"))
                .ok_or_else(|| error("IndexError", "list index out of range"))?;
            let bb = parsed(lines[j], 4)?;
            if bb.len() < 4 {
                return Err(Error::value(format!(
                    "not enough values to unpack (expected 4, got {})",
                    bb.len()
                )));
            }
            let (bw, bh, bx, by) = (dim(bb[0])?, dim(bb[1])?, bb[2], bb[3]);
            let mut dw = cw as i64;
            let mut k = i;
            while k < lines.len() && !lines[k].starts_with("BITMAP") {
                if lines[k].starts_with("DWIDTH") {
                    dw = *parsed(lines[k], 1)?
                        .first()
                        .ok_or_else(|| error("IndexError", "list index out of range"))?;
                }
                k += 1;
            }
            if k >= lines.len() {
                return Err(error("IndexError", "list index out of range"));
            }
            let nbits = bw.div_ceil(8) * 8;
            let previous = glyphs.get(&code).map_or(0, |g| g.1.len());
            stored = stored - previous + cw * ch;
            if stored > LIMIT || (!glyphs.contains_key(&code) && glyphs.len() >= 4096) {
                return Err(Error::value("BDF glyph storage exceeds supported bound"));
            }
            let mut cell = vec![0; cw * ch];
            let top = base - (by + bh as i64);
            for (r, row) in lines[k + 1..(k + 1 + bh).min(lines.len())]
                .iter()
                .enumerate()
            {
                let bits = row_bits(row, nbits)?;
                for c in 0..bw {
                    if bits[nbits - 1 - c] {
                        let (x, y) = (bx + c as i64, top + r as i64);
                        if x >= 0 && x < cw as i64 && y >= 0 && y < ch as i64 {
                            cell[y as usize * cw + x as usize] = 15;
                        }
                    }
                }
            }
            glyphs.insert(code, (dw, cell));
            i = k + bh;
        }
        i += 1;
    }
    if last >= 0xdc && !glyphs.contains_key(&0xdc) {
        if let Some((dw, original)) = glyphs.get(&85) {
            if stored + original.len() > LIMIT {
                return Err(Error::value("BDF glyph storage exceeds supported bound"));
            }
            let mut cell = original.clone();
            let lit: Vec<_> = (0..ch)
                .flat_map(|y| {
                    (0..cw).filter_map({
                        let cell = &cell;
                        move |x| {
                            if cell[y * cw + x] != 0 {
                                Some((x, y))
                            } else {
                                None
                            }
                        }
                    })
                })
                .collect();
            if !lit.is_empty() {
                let lo = lit.iter().map(|p| p.0).min().unwrap();
                let hi = lit.iter().map(|p| p.0).max().unwrap();
                let top = lit.iter().map(|p| p.1).min().unwrap();
                if top + 1 >= ch {
                    return Err(error("IndexError", "list assignment index out of range"));
                }
                for x in 0..cw {
                    cell[top * cw + x] = if x == lo || x == hi { 15 } else { 0 };
                    cell[(top + 1) * cw + x] = 0;
                }
                glyphs.insert(0xdc, (*dw, cell));
            }
        }
    }
    let missing: String = (first..=last.min(126))
        .filter(|c| !glyphs.contains_key(&i64::from(*c)))
        .filter_map(char::from_u32)
        .collect();
    if !missing.is_empty() {
        let name = std::path::Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        warn(
            io,
            format!(
                "font {name}: no glyph for {} (drawn as '?')",
                ascii_repr(&missing)
            ),
        )?;
    }
    let mut out = Vec::new();
    let mut total = 0;
    for c in first..=last {
        // Python evaluates get(c, glyphs[63]) eagerly, even for a present c.
        let fallback = glyphs.get(&63).ok_or_else(|| error("KeyError", "63"))?;
        let (dw, cell) = glyphs.get(&i64::from(c)).unwrap_or(fallback);
        let bw = cw + 2 * PAD;
        let mut pixels = Vec::with_capacity(bw * ch);
        for y in 0..ch {
            pixels.extend([0; PAD]);
            pixels.extend(&cell[y * cw..(y + 1) * cw]);
            pixels.extend([0; PAD]);
        }
        let advance = dw
            .checked_mul(scale as i64)
            .filter(|n| n.unsigned_abs() <= 32768)
            .ok_or_else(|| Error::value("font advance outside supported bound"))?;
        total += plane(bw * scale, ch * scale)?;
        if total > LIMIT {
            return Err(Error::value("font pixels exceed supported bound"));
        }
        out.push(Glyph {
            advance,
            width: bw * scale,
            pixels: upscale(&pixels, bw, ch, scale)?,
        });
    }
    Ok(Font {
        height: ch * scale,
        glyphs: out,
    })
}
pub fn path(io: &mut impl Adapter, search: &Search<'_>, name: &str) -> Result<String> {
    let dirs = search
        .directories
        .as_array()
        .ok_or_else(|| Error::value("expected font directories"))?;
    for dir in
        std::iter::once(search.fonts).chain(dirs.iter().map(text).collect::<Result<Vec<_>>>()?)
    {
        let path = std::path::Path::new(dir)
            .join(name)
            .to_string_lossy()
            .into_owned();
        if action(io, "exists", json!([path]))? == true {
            return Ok(path);
        }
    }
    Err(error(
        "SystemExit",
        format!("gen_font: {name} not in {} or X0X_FONT_DIRS", search.fonts),
    ))
}
pub fn render(io: &mut impl Adapter, search: &Search<'_>, s: &Spec<'_>) -> Result<Font> {
    if (s.last >= s.first && s.last - s.first > 4096) || s.scale > 16 {
        return Err(Error::value("font character range outside supported bound"));
    }
    if s.file.ends_with(".bdf") {
        let p = path(io, search, s.file)?;
        let source = action(io, "read_bdf", json!([p]))?;
        return bdf(text(&source)?, &p, s.scale, s.first, s.last, io);
    }
    let (name, var) = s.file.split_once('@').unwrap_or((s.file, ""));
    let p = path(io, search, name)?;
    action(io, "open_ttf", json!([p, s.px]))?;
    if !var.is_empty() {
        let mut values = BTreeMap::new();
        for (i, pair) in var.split(',').enumerate() {
            let parts: Vec<_> = pair.split('=').collect();
            if parts.len() != 2 {
                return Err(Error::value(format!(
                    "dictionary update sequence element #{i} has length {}; 2 is required",
                    parts.len()
                )));
            }
            values.insert(parts[0], parts[1]);
        }
        let axes = action(io, "axes", json!([]))?;
        let names: Vec<_> = axes
            .as_array()
            .ok_or_else(|| Error::value("invalid font axes"))?
            .iter()
            .map(|a| text(&a["name"]).map(str::to_owned))
            .collect::<Result<_>>()?;
        let axes = action(io, "axes", json!([]))?;
        let mut selected = Vec::new();
        for (n, axis) in names.iter().zip(
            axes.as_array()
                .ok_or_else(|| Error::value("invalid font axes"))?,
        ) {
            let key: String = n.to_lowercase().chars().take(4).collect();
            let v = values
                .get(key.as_str())
                .or_else(|| values.get(n.as_str()))
                .filter(|s| !s.is_empty());
            selected.push(if let Some(v) = v {
                let value = v.trim().parse::<f64>().map_err(|_| {
                    Error::value(format!(
                        "could not convert string to float: {}",
                        crate::installer::repr(v)
                    ))
                })?;
                if !value.is_finite() {
                    return Err(Error::value("font axis must be finite"));
                }
                value
            } else {
                number(&axis["default"])?
            });
        }
        action(io, "set_axes", json!([selected]))?;
    }
    let features = if s.pixel {
        Value::Null
    } else {
        json!(["tnum"])
    };
    let metrics = action(io, "metrics", json!([features]))?;
    let asc = integer(&metrics["asc"])?;
    let desc = integer(&metrics["desc"])?;
    let top = 0.max(integer(&metrics["bbox_top"])? - 1);
    let h = dim(asc + desc - top)?;
    let full = dim(asc + desc)?;
    let mut glyphs = Vec::new();
    let mut total = 0;
    for code in s.first..=s.last {
        let length = number(&action(io, "length", json!([code, features]))?)?;
        if length.abs() > 2048.0 {
            return Err(Error::value("font advance outside supported bound"));
        }
        let adv = length.round_ties_even() as i64;
        let bw = dim(adv + 2 * PAD as i64)?;
        plane(bw, full)?;
        total += plane(bw * s.scale, h * s.scale)?;
        if total > LIMIT {
            return Err(Error::value("font pixels exceed supported bound"));
        }
        let gray = bytes(&action(
            io,
            "glyph",
            json!([code, bw, full, top, h, features]),
        )?)?;
        if gray.len() != bw * h {
            return Err(Error::value("rasterizer returned wrong glyph size"));
        }
        let pixels: Vec<_> = gray
            .into_iter()
            .map(|v| {
                if s.pixel {
                    if v >= 128 {
                        15
                    } else {
                        0
                    }
                } else {
                    ((u16::from(v) + 8) / 17).min(15) as u8
                }
            })
            .collect();
        glyphs.push(Glyph {
            advance: adv * s.scale as i64,
            width: bw * s.scale,
            pixels: upscale(&pixels, bw, h, s.scale)?,
        });
    }
    Ok(Font {
        height: h * s.scale,
        glyphs,
    })
}
fn encode(font: &Font) -> Result<(Vec<u8>, Vec<usize>)> {
    let mut data = Vec::new();
    let mut offsets = Vec::new();
    for glyph in &font.glyphs {
        if glyph.pixels.len() != glyph.width * font.height {
            return Err(Error::value("glyph plane does not match size"));
        }
        offsets.push(data.len());
        for y in 0..font.height {
            let row = &glyph.pixels[y * glyph.width..(y + 1) * glyph.width];
            for x in (0..glyph.width).step_by(2) {
                data.push((row[x] << 4) | row.get(x + 1).copied().unwrap_or(0));
            }
        }
    }
    if data.len() >= 65536 {
        return Err(super::assertion(""));
    }
    Ok((data, offsets))
}
pub fn generate(
    io: &mut impl Adapter,
    search: &Search<'_>,
    fontset: &str,
    sizes: &Value,
    out: &str,
) -> Result<()> {
    let sizes = sizes
        .as_array()
        .ok_or_else(|| Error::value("expected font sizes"))?;
    if sizes.len() > 64 || fontset.len() > 128 {
        return Err(Error::value("font recipes exceed supported bound"));
    }
    let mut lines = vec![
        format!("/* generated by tools/gen_font.py: font set {fontset} */"),
        "#pragma once".into(),
        "#include <stdint.h>".into(),
        format!("#define FONT_PAD {PAD}  /* x scale for L, see below */"),
        String::new(),
    ];
    for v in sizes {
        let a = super::exact(v, 7)?;
        let name = text(&a[0])?;
        if name.len() > 128 || text(&a[1])?.len() > 4096 {
            return Err(Error::value("font recipe name exceeds supported bound"));
        }
        let spec = Spec {
            file: text(&a[1])?,
            px: integer(&a[2])?,
            scale: size(&a[3], 16)?,
            pixel: a[4] == true,
            first: u32::try_from(integer(&a[5])?)
                .map_err(|_| Error::value("invalid first glyph"))?,
            last: u32::try_from(integer(&a[6])?).map_err(|_| Error::value("invalid last glyph"))?,
        };
        let font = render(io, search, &spec)?;
        let (data, offsets) = encode(&font)?;
        lines.push(format!(
            "static const uint8_t FONT_{name}_DATA[{}] = {{",
            data.len()
        ));
        for row in data.chunks(24) {
            lines.push(format!(
                "    {},",
                row.iter()
                    .map(|b| format!("0x{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        lines.push("};".into());
        let list = |v: Vec<String>| v.join(", ");
        lines.push(format!(
            "static const uint16_t FONT_{name}_OFF[{}] = {{{}}};",
            offsets.len(),
            list(offsets.iter().map(usize::to_string).collect())
        ));
        lines.push(format!(
            "static const uint8_t FONT_{name}_ADV[{}] = {{{}}};",
            font.glyphs.len(),
            list(font.glyphs.iter().map(|g| g.advance.to_string()).collect())
        ));
        lines.push(format!(
            "static const uint8_t FONT_{name}_BW[{}] = {{{}}};",
            font.glyphs.len(),
            list(font.glyphs.iter().map(|g| g.width.to_string()).collect())
        ));
        lines.push(format!("static const felucca_font_t FONT_{name} = {{ {}, {}, {}, {}, FONT_{name}_ADV, FONT_{name}_BW, FONT_{name}_OFF, FONT_{name}_DATA }};",font.height,PAD*spec.scale,spec.first,spec.last));
        lines.push(String::new());
        let index = 48i64 - i64::from(spec.first);
        let index = if index < 0 {
            font.glyphs.len() as i64 + index
        } else {
            index
        };
        let digit = usize::try_from(index)
            .ok()
            .and_then(|i| font.glyphs.get(i))
            .ok_or_else(|| error("IndexError", "list index out of range"))?
            .advance;
        warn(
            io,
            format!(
                "font {name}: {} {}px h {}, digit adv {digit}, {} B",
                spec.file,
                spec.px,
                font.height,
                data.len()
            ),
        )?;
    }
    let approximate = lines
        .iter()
        .filter(|l| l.starts_with("    0x"))
        .map(String::len)
        .sum::<usize>()
        / 6;
    warn(io, format!("font set {fontset}: {approximate} B (approx)"))?;
    action(io, "write_header", json!([out, lines.join("\n")]))?;
    Ok(())
}

fn font_value(font: Font) -> Value {
    json!([
        font.height,
        font.glyphs
            .into_iter()
            .map(|g| json!([g.advance, g.width, g.pixels]))
            .collect::<Vec<_>>()
    ])
}
pub fn request(method: &str, v: &Value, io: &mut impl Adapter) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "upscale" => json!(upscale(
            &bytes(&a[0])?,
            size(&a[1], 2048)?,
            size(&a[2], 2048)?,
            size(&a[3], 16)?
        )?),
        "path" => json!(path(
            io,
            &Search {
                fonts: text(&a[1])?,
                directories: &a[2]
            },
            text(&a[0])?
        )?),
        "bdf" => {
            let p = text(&a[0])?;
            let s = action(io, "read_bdf", json!([p]))?;
            font_value(bdf(
                text(&s)?,
                p,
                size(&a[1], 16)?,
                u32::try_from(integer(&a[2])?).map_err(|_| Error::value("invalid glyph"))?,
                u32::try_from(integer(&a[3])?).map_err(|_| Error::value("invalid glyph"))?,
                io,
            )?)
        }
        "render" => font_value(render(
            io,
            &Search {
                fonts: text(&a[6])?,
                directories: &a[7],
            },
            &Spec {
                file: text(&a[0])?,
                px: integer(&a[1])?,
                scale: size(&a[2], 16)?,
                pixel: a[3] == true,
                first: u32::try_from(integer(&a[4])?).map_err(|_| Error::value("invalid glyph"))?,
                last: u32::try_from(integer(&a[5])?).map_err(|_| Error::value("invalid glyph"))?,
            },
        )?),
        "generate" => {
            generate(
                io,
                &Search {
                    fonts: text(&a[3])?,
                    directories: &a[4],
                },
                text(&a[0])?,
                &a[1],
                text(&a[2])?,
            )?;
            Value::Null
        }
        _ => return Err(Error::value("unknown font method")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Offline;
    impl Adapter for Offline {
        fn action(&mut self, _: Value) -> Result<Value> {
            Err(Error::value("unexpected I/O"))
        }
    }
    #[test]
    fn packed_rows_keep_high_nibble_and_zero_odd_tail() {
        let font = Font {
            height: 2,
            glyphs: vec![
                Glyph {
                    advance: -1,
                    width: 3,
                    pixels: vec![1, 2, 3, 4, 5, 6],
                },
                Glyph {
                    advance: 0,
                    width: 1,
                    pixels: vec![15, 0],
                },
            ],
        };
        assert_eq!(
            encode(&font).unwrap(),
            (vec![0x12, 0x30, 0x45, 0x60, 0xf0, 0], vec![0, 4])
        );
    }
    #[test]
    fn admission_refuses_unbounded_planes_and_ranges_before_io() {
        assert!(plane(usize::MAX, 2).is_err());
        assert!(upscale(&[0], 1, 1, 2048).is_err());
        assert_eq!(upscale(&[], 2, 2, 2).unwrap_err().kind, "IndexError");
        assert!(bdf("", "", 1, 0, 4097, &mut Offline).is_err());
        assert!(bdf("", "", 17, 63, 63, &mut Offline).is_err());
        assert!(integer(&json!(i64::MIN)).is_err());
        assert!(generate(
            &mut Offline,
            &Search {
                fonts: "",
                directories: &json!([])
            },
            "safe",
            &json!(vec![Value::Null; 65]),
            ""
        )
        .is_err());
        let font = Font {
            height: 512,
            glyphs: vec![Glyph {
                advance: 256,
                width: 256,
                pixels: vec![0; 512 * 256],
            }],
        };
        assert_eq!(encode(&font).unwrap_err().kind, "AssertionError");
    }
}
