// SPDX-License-Identifier: GPL-3.0-only
//! Kick Runner's two-band drawing composition over the caller's existing strip.
use crate::{
    graphics::{rgb, Canvas, Error, Font},
    runner::{State, LANES},
    text::Text,
};
use fm1_engine::effects::math::{abs, floor, max, min, sin_turns, sqrt};

pub fn valid(state: &State) -> bool {
    state.valid()
        && abs(state.view) <= 1_000_000.0
        && (0.0..=1.0).contains(&state.kick)
        && (0.0..=1.0).contains(&state.flash)
        && (0.0..=1.0).contains(&state.crash)
}
fn q16(x: f32) -> i32 {
    if x <= 0.0 {
        0
    } else if x >= 1.0 {
        16
    } else {
        (x * 16.0 + 0.5) as i32
    }
}
fn mix(a: u16, b: u16, t: i32) -> u16 {
    if t <= 0 {
        return a;
    }
    if t >= 16 {
        return b;
    }
    let (a, b, t) = (u32::from(a), u32::from(b), t as u32);
    let r = ((a >> 11) * (16 - t) + (b >> 11) * t) / 16;
    let g = (((a >> 5) & 63) * (16 - t) + ((b >> 5) & 63) * t) / 16;
    let blue = ((a & 31) * (16 - t) + (b & 31) * t) / 16;
    ((r << 11) | (g << 5) | blue) as u16
}
pub fn background(state: &State) -> u16 {
    if state.crash > 0.05 {
        mix(0, rgb(255, 64, 48), q16(state.crash * 0.25))
    } else {
        0
    }
}
fn radius(depth: f32) -> f32 {
    118.0 / (1.0 + 0.62 * depth)
}
fn point(state: &State, depth: f32, lane: f32) -> (f32, f32) {
    let a = state.view + lane / f32::from(LANES);
    let r = radius(depth);
    (120.0 + r * sin_turns(a + 0.25), 108.0 + r * sin_turns(a))
}
fn glow(canvas: &mut Canvas<'_>, cx: f32, cy: f32, r0: f32, r1: f32, color: u16, strength: f32) {
    let (x0, x1) = (floor(cx - r1) as i32, (cx + r1 + 1.0) as i32);
    let (y0, y1) = (floor(cy - r1) as i32, (cy + r1 + 1.0) as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let distance = sqrt(dx * dx + dy * dy);
            if distance >= r1 {
                continue;
            }
            let alpha = if distance <= r0 {
                1.0
            } else {
                1.0 - (distance - r0) / (r1 - r0)
            };
            canvas.blend16(x, y, color, (alpha * alpha * strength * 16.0 + 0.5) as u32);
        }
    }
}
fn segment(
    canvas: &mut Canvas<'_>,
    mut x0: f32,
    mut y0: f32,
    mut x1: f32,
    mut y1: f32,
    color: u16,
    strength: f32,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let ax = if dx < 0.0 { -dx } else { dx };
    let ay = if dy < 0.0 { -dy } else { dy };
    if strength <= 0.02 {
        return;
    }
    if ax >= ay {
        if x1 < x0 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }
        let n = ((x1 - x0) as i32 + 1).clamp(0, 1024);
        let step = if ax > 0.0 { (y1 - y0) / (x1 - x0) } else { 0.0 };
        for k in 0..=n {
            let x = x0 + k as f32;
            let y = y0 + step * k as f32;
            if x > x1 + 0.5 {
                break;
            }
            let iy = floor(y) as i32;
            let fraction = y - iy as f32;
            canvas.blend16(
                x as i32,
                iy,
                color,
                ((1.0 - fraction) * strength * 16.0 + 0.5) as u32,
            );
            canvas.blend16(
                x as i32,
                iy + 1,
                color,
                (fraction * strength * 16.0 + 0.5) as u32,
            );
        }
    } else {
        if y1 < y0 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }
        let n = ((y1 - y0) as i32 + 1).clamp(0, 1024);
        let step = (x1 - x0) / (y1 - y0);
        for k in 0..=n {
            let y = y0 + k as f32;
            let x = x0 + step * k as f32;
            if y > y1 + 0.5 {
                break;
            }
            let ix = floor(x) as i32;
            let fraction = x - ix as f32;
            canvas.blend16(
                ix,
                y as i32,
                color,
                ((1.0 - fraction) * strength * 16.0 + 0.5) as u32,
            );
            canvas.blend16(
                ix + 1,
                y as i32,
                color,
                (fraction * strength * 16.0 + 0.5) as u32,
            );
        }
    }
}
fn centered(
    canvas: &mut Canvas<'_>,
    x: i32,
    y: i32,
    font: &Font<'_>,
    text: &[u8],
    color: u16,
) -> Result<(), Error> {
    let width = font.text_width(text)?;
    canvas.text(x - width / 2, y, font, text, color)?;
    Ok(())
}
fn tag(canvas: &mut Canvas<'_>, x: i32, color: u16) {
    canvas.rectangle(x, 8, 13, 7, color);
    for (j, d) in [2, 1, 0].into_iter().enumerate() {
        canvas.rectangle(x + d, 5 + j as i32, 13 - 2 * d, 1, color);
        canvas.rectangle(x + d, 17 - j as i32, 13 - 2 * d, 1, color);
    }
}
/// Theme: pulse, text, dim, accent. Fonts: XS, S, B. No callback or allocation.
pub fn draw(
    canvas: &mut Canvas<'_>,
    state: &State,
    now: u32,
    playing: bool,
    theme: [u16; 4],
    fonts: [&Font<'_>; 3],
) -> Result<(), Error> {
    if !valid(state) {
        return Err(Error::Canvas);
    }
    let fraction = state.beat - floor(state.beat);
    let (line, text, dim, accent) = (theme[0], theme[1], theme[2], theme[3]);
    let wall = rgb(255, 64, 48);
    let gate = rgb(70, 255, 160);
    for i in 0..LANES {
        let (x0, y0) = point(state, 0.0, f32::from(i));
        let (x1, y1) = point(state, 9.0, f32::from(i));
        segment(canvas, x0, y0, x1, y1, line, 0.35);
    }
    for k in (0..=9).rev() {
        let depth = k as f32 - fraction;
        if depth < 0.0 {
            continue;
        }
        let strength =
            (1.0 - depth / 10.0) * (0.45 + 0.55 * if k == 0 { state.kick } else { 0.0 } + 0.2);
        for i in 0..LANES {
            let (x0, y0) = point(state, depth, f32::from(i));
            let (x1, y1) = point(state, depth, f32::from(i + 1));
            segment(canvas, x0, y0, x1, y1, line, min(strength, 1.0));
        }
    }
    for row in &state.row {
        let depth = row.beat - state.beat;
        if row.wall | row.gate == 0 || !(-0.3..=9.0).contains(&depth) {
            continue;
        }
        for i in 0..LANES {
            let fade = 1.0 - max(depth, 0.0) / 10.0;
            if row.wall >> i & 1 != 0 {
                for t in 0..6 {
                    let z = max(depth + 0.07 * t as f32, 0.0);
                    let (x0, y0) = point(state, z, f32::from(i) + 0.04);
                    let (x1, y1) = point(state, z, f32::from(i) + 0.96);
                    segment(canvas, x0, y0, x1, y1, wall, fade);
                }
                let (x0, y0) = point(state, max(depth, 0.0), f32::from(i) + 0.5);
                let r = radius(max(depth, 0.0));
                let a = state.view + (f32::from(i) + 0.5) / f32::from(LANES);
                let x1 = 120.0 + r * 0.72 * sin_turns(a + 0.25);
                let y1 = 108.0 + r * 0.72 * sin_turns(a);
                segment(canvas, x0, y0, x1, y1, wall, fade * 0.8);
            }
            if row.gate >> i & 1 != 0 && row.taken == 0 {
                let z = max(depth, 0.0);
                let (x0, y0) = point(state, z, f32::from(i) + 0.12);
                let (x1, y1) = point(state, z, f32::from(i) + 0.88);
                segment(canvas, x0, y0, x1, y1, gate, fade);
                let r = radius(z);
                let a = state.view + (f32::from(i) + 0.5) / f32::from(LANES);
                let mx = 120.0 + r * 0.8 * sin_turns(a + 0.25);
                let my = 108.0 + r * 0.8 * sin_turns(a);
                segment(canvas, x0, y0, mx, my, gate, fade);
                segment(canvas, x1, y1, mx, my, gate, fade);
                glow(canvas, mx, my, 1.0, 3.0 + 5.0 * fade, gate, 0.3 * fade);
            }
        }
    }
    if state.over == 0 {
        let a = state.view + (f32::from(state.lane) + 0.5) / f32::from(LANES);
        let r = radius(0.12);
        let c = sin_turns(a + 0.25);
        let s = sin_turns(a);
        let (px, py) = (120.0 + r * c, 108.0 + r * s);
        let (ux, uy, vx, vy) = (-c, -s, -s, c);
        let color = if state.flash > 0.1 {
            mix(text, gate, q16(state.flash))
        } else {
            text
        };
        segment(
            canvas,
            px + vx * 11.0,
            py + vy * 11.0,
            px + ux * 9.0,
            py + uy * 9.0,
            color,
            1.0,
        );
        segment(
            canvas,
            px - vx * 11.0,
            py - vy * 11.0,
            px + ux * 9.0,
            py + uy * 9.0,
            color,
            1.0,
        );
        segment(
            canvas,
            px + vx * 11.0,
            py + vy * 11.0,
            px + vx * 4.0 + ux * 2.0,
            py + vy * 4.0 + uy * 2.0,
            color,
            1.0,
        );
        segment(
            canvas,
            px - vx * 11.0,
            py - vy * 11.0,
            px - vx * 4.0 + ux * 2.0,
            py - vy * 4.0 + uy * 2.0,
            color,
            1.0,
        );
        if state.kick > 0.05 {
            glow(
                canvas,
                px,
                py,
                1.0,
                6.0 + 8.0 * state.kick,
                accent,
                0.35 * state.kick,
            );
        }
    }
    for (k, label) in [b"K", b"H", b"S", b"A", b"D", b"P"].iter().enumerate() {
        let lit = k == 0 || k <= usize::from(state.layer);
        tag(
            canvas,
            6 + k as i32 * 15,
            if lit { accent } else { rgb(30, 30, 34) },
        );
        centered(
            canvas,
            12 + k as i32 * 15,
            5,
            fonts[0],
            *label,
            if lit { 0 } else { dim },
        )?;
    }
    let mut bytes = [0u8; 24];
    let mut number = Text::new(&mut bytes);
    number.unsigned(state.score);
    canvas.text(
        234 - fonts[1].text_width(number.bytes())?,
        3,
        fonts[1],
        number.bytes(),
        text,
    )?;
    for k in 0..state.ships {
        let x = 228.0 - f32::from(k) * 13.0;
        let y = 30.0;
        segment(canvas, x - 5.0, y + 3.0, x, y - 4.0, text, 1.0);
        segment(canvas, x + 5.0, y + 3.0, x, y - 4.0, text, 1.0);
    }
    if !playing && state.over == 0 {
        centered(canvas, 120, 222, fonts[1], b"PRESS PLAY", dim)?;
    } else if now.wrapping_sub(state.start_ms) < 4500 && state.over == 0 {
        centered(
            canvas,
            120,
            222,
            fonts[1],
            b"SELECT STEERS  TAKE THE GATES",
            dim,
        )?;
    }
    if state.over != 0 {
        // The old32-byte local overflows for two full-width u32 scores.
        let mut bytes = [0u8; 40];
        let mut line = Text::new(&mut bytes);
        line.append(b"SCORE ");
        line.unsigned(state.score);
        line.append(b"  BEST ");
        line.unsigned(state.best);
        centered(canvas, 120, 70, fonts[2], b"GAME OVER", wall)?;
        centered(canvas, 120, 104, fonts[1], line.bytes(), text)?;
        centered(
            canvas,
            120,
            128,
            fonts[1],
            b"A KEY: AGAIN   HOME: BACK",
            dim,
        )?;
    } else if now < state.banner_until {
        centered(canvas, 120, 70, fonts[2], &state.banner, accent)?;
    }
    Ok(())
}
