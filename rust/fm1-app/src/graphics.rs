// SPDX-License-Identifier: GPL-3.0-only
//! RGB565 strip primitives. No LCD, callback, heap or retained pixel borrow.
pub const MAX_PIXELS: usize = 240 * 144;
pub const TEXT_MAX: usize = 256;
use core::mem::MaybeUninit;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Canvas,
    Font,
    Text,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub width: u32,
    pub height: u32,
    pub origin: i32,
    pub top: i32,
    pub bottom: i32,
    pub scan: u32,
}
impl State {
    pub fn begin(width: u32, height: u32, capacity: usize, scan: u32) -> Result<Self, Error> {
        let capacity = capacity.min(MAX_PIXELS) as u32;
        let height = if width == 0 {
            height
        } else {
            height.min(capacity / width)
        };
        let state = Self {
            width,
            height,
            origin: 0,
            top: 0,
            bottom: height as i32,
            scan,
        };
        if state.valid(capacity as usize) {
            Ok(state)
        } else {
            Err(Error::Canvas)
        }
    }
    fn valid(self, capacity: usize) -> bool {
        self.width <= MAX_PIXELS as u32
            && self.height <= MAX_PIXELS as u32
            && self
                .width
                .checked_mul(self.height)
                .is_some_and(|n| n as usize <= capacity && n as usize <= MAX_PIXELS)
            && self.top >= 0
            && self.bottom >= self.top
            && self.bottom <= self.height as i32
    }
    pub fn band(&mut self, top: i32, height: i32) -> Result<(), Error> {
        let Some(bottom) = top.checked_add(height) else {
            return Err(Error::Canvas);
        };
        if top < 0 || bottom < top || bottom > self.height as i32 {
            return Err(Error::Canvas);
        }
        self.origin = top;
        self.top = top;
        self.bottom = bottom;
        Ok(())
    }
}
/// The hardware pool need not be initialized at reset. Write the complete
/// active strip before the narrow bridge forms a typed pixel slice over it.
pub fn initialize(
    pixels: &mut [MaybeUninit<u16>],
    width: u32,
    height: u32,
    color: u16,
    scan: u32,
) -> Result<State, Error> {
    let state = State::begin(width, height, pixels.len(), scan)?;
    for pixel in &mut pixels[..(state.width * state.height) as usize] {
        pixel.write(color.swap_bytes());
    }
    Ok(state)
}
pub struct Font<'a> {
    height: u8,
    pad: u8,
    first: u8,
    last: u8,
    advance: &'a [u8],
    width: &'a [u8],
    offset: &'a [u16],
    data: &'a [u8],
}
impl<'a> Font<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        height: u8,
        pad: u8,
        first: u8,
        last: u8,
        advance: &'a [u8],
        width: &'a [u8],
        offset: &'a [u16],
        data: &'a [u8],
    ) -> Result<Self, Error> {
        if first > b'?' || last < b'?' || last < first {
            return Err(Error::Font);
        }
        let count = usize::from(last - first) + 1;
        if advance.len() != count || width.len() != count || offset.len() != count {
            return Err(Error::Font);
        }
        for (i, w) in width.iter().enumerate() {
            let end = usize::from(offset[i]) + usize::from(height) * usize::from(*w).div_ceil(2);
            if end > data.len() {
                return Err(Error::Font);
            }
        }
        Ok(Self {
            height,
            pad,
            first,
            last,
            advance,
            width,
            offset,
            data,
        })
    }
    pub fn glyph(&self, mut byte: u8) -> usize {
        if byte.is_ascii_lowercase() && self.last < b'a' {
            byte -= 32;
        }
        if byte < self.first || byte > self.last || (127..160).contains(&byte) {
            byte = b'?';
        }
        usize::from(byte - self.first)
    }
    pub fn text_width(&self, text: &[u8]) -> Result<i32, Error> {
        if text.len() > TEXT_MAX {
            return Err(Error::Text);
        }
        let mut width = 0i32;
        for byte in text.iter().take_while(|byte| **byte != 0) {
            width = width.wrapping_add(i32::from(self.advance[self.glyph(*byte)]));
        }
        Ok(width)
    }
}
pub struct Canvas<'a> {
    pixels: &'a mut [u16],
    state: State,
    writes: u32,
}
impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u16], state: State) -> Result<Self, Error> {
        if !state.valid(pixels.len()) {
            return Err(Error::Canvas);
        }
        Ok(Self {
            pixels,
            state,
            writes: 0,
        })
    }
    pub fn begin(
        pixels: &'a mut [u16],
        width: u32,
        height: u32,
        background: u16,
        scan: u32,
    ) -> Result<Self, Error> {
        let state = State::begin(width, height, pixels.len(), scan)?;
        let mut canvas = Self::new(pixels, state)?;
        let n = (state.width * state.height) as usize;
        canvas.pixels[..n].fill(background.swap_bytes());
        canvas.writes = n as u32;
        Ok(canvas)
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn writes(&self) -> u32 {
        self.writes
    }
    fn index(&self, x: i64, y: i64) -> Option<usize> {
        let y = y + i64::from(self.state.origin);
        if x < 0
            || x >= i64::from(self.state.width)
            || y < i64::from(self.state.top)
            || y >= i64::from(self.state.bottom)
        {
            None
        } else {
            Some(y as usize * self.state.width as usize + x as usize)
        }
    }
    pub fn pixel(&mut self, x: i32, y: i32, color: u16) {
        self.pixel_wide(i64::from(x), i64::from(y), color);
    }
    fn pixel_wide(&mut self, x: i64, y: i64, color: u16) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color.swap_bytes();
            self.writes = self.writes.wrapping_add(1);
        }
    }
    pub fn rectangle(&mut self, x: i32, y: i32, width: i32, height: i32, color: u16) {
        let left = i64::from(x).max(0);
        let right = (i64::from(x) + i64::from(width)).min(i64::from(self.state.width));
        let top = (i64::from(y) + i64::from(self.state.origin)).max(i64::from(self.state.top));
        let bottom = (i64::from(y) + i64::from(height) + i64::from(self.state.origin))
            .min(i64::from(self.state.bottom));
        if left >= right || top >= bottom {
            return;
        }
        for row in top..bottom {
            let at = row as usize * self.state.width as usize;
            self.pixels[at + left as usize..at + right as usize].fill(color.swap_bytes());
            self.writes = self.writes.wrapping_add((right - left) as u32);
        }
    }
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u16) {
        let (mut x, mut y) = (i64::from(x0), i64::from(y0));
        let (x1, y1) = (i64::from(x1), i64::from(y1));
        let dx = (x1 - x).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let dy = -(y1 - y).abs();
        let sy = if y < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        for _ in 0..2000 {
            let e2 = 2 * error;
            self.pixel_wide(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            if e2 >= dy {
                error += dy;
                x += sx;
            }
            if e2 <= dx {
                error += dx;
                y += sy;
            }
        }
    }
    pub fn scanlines(&mut self, screen_y: u32, first_row: u32) {
        if self.state.scan == 0 {
            return;
        }
        let mut row = first_row.wrapping_add(if screen_y.wrapping_add(first_row) & 1 != 0 {
            0
        } else {
            1
        });
        while row < self.state.height {
            let at = row as usize * self.state.width as usize;
            for p in &mut self.pixels[at..at + self.state.width as usize] {
                let c = p.swap_bytes();
                *p = (c - ((c >> 2) & 0x39e7)).swap_bytes();
                self.writes = self.writes.wrapping_add(1);
            }
            row = row.wrapping_add(2);
        }
    }
    pub fn blend16(&mut self, x: i32, y: i32, color: u16, alpha: u32) {
        if alpha == 0 {
            return;
        }
        if alpha >= 16 {
            self.pixel(x, y, color);
            return;
        }
        if let Some(i) = self.index(i64::from(x), i64::from(y)) {
            let bg = u32::from(self.pixels[i].swap_bytes());
            let c = u32::from(color);
            let value = ((((c >> 11) * alpha + (bg >> 11) * (16 - alpha)) >> 4) << 11)
                | (((((c >> 5) & 63) * alpha + ((bg >> 5) & 63) * (16 - alpha)) >> 4) << 5)
                | (((c & 31) * alpha + (bg & 31) * (16 - alpha)) >> 4);
            self.pixels[i] = (value as u16).swap_bytes();
            self.writes = self.writes.wrapping_add(1);
        }
    }
    pub fn text(
        &mut self,
        mut x: i32,
        y: i32,
        font: &Font<'_>,
        text: &[u8],
        color: u16,
    ) -> Result<i32, Error> {
        if text.len() > TEXT_MAX {
            return Err(Error::Text);
        }
        let color32 = u32::from(color);
        let (r, g, b) = (color32 >> 11, (color32 >> 5) & 63, color32 & 31);
        for byte in text.iter().take_while(|byte| **byte != 0) {
            let glyph = font.glyph(*byte);
            let width = usize::from(font.width[glyph]);
            let stride = width.div_ceil(2);
            let offset = usize::from(font.offset[glyph]);
            for gy in 0..usize::from(font.height) {
                for gx in 0..width {
                    let packed = font.data[offset + gy * stride + gx / 2];
                    let alpha = u32::from(if gx & 1 != 0 {
                        packed & 15
                    } else {
                        packed >> 4
                    });
                    if alpha == 0 {
                        continue;
                    }
                    if let Some(index) = self.index(
                        i64::from(x) - i64::from(font.pad) + gx as i64,
                        i64::from(y) + gy as i64,
                    ) {
                        let value = if alpha == 15 {
                            color
                        } else {
                            let bg = u32::from(self.pixels[index].swap_bytes());
                            let nr = (r * alpha + (bg >> 11) * (15 - alpha)) / 15;
                            let ng = (g * alpha + ((bg >> 5) & 63) * (15 - alpha)) / 15;
                            let nb = (b * alpha + (bg & 31) * (15 - alpha)) / 15;
                            ((nr << 11) | (ng << 5) | nb) as u16
                        };
                        self.pixels[index] = value.swap_bytes();
                        self.writes = self.writes.wrapping_add(1);
                    }
                }
            }
            x = x.wrapping_add(i32::from(font.advance[glyph]));
        }
        Ok(x)
    }
}
pub const fn rgb(r: u8, g: u8, b: u8) -> u16 {
    (((r as u16) >> 3) << 11) | (((g as u16) >> 2) << 5) | ((b as u16) >> 3)
}
pub const PALETTES: [[u16; 5]; 5] = [
    [
        rgb(34, 34, 40),
        rgb(74, 74, 82),
        rgb(150, 150, 158),
        rgb(80, 220, 120),
        rgb(150, 255, 175),
    ],
    [
        rgb(34, 34, 40),
        rgb(74, 74, 82),
        rgb(150, 150, 158),
        rgb(232, 136, 30),
        rgb(255, 184, 70),
    ],
    [
        rgb(34, 34, 40),
        rgb(74, 74, 82),
        rgb(150, 150, 158),
        rgb(60, 180, 235),
        rgb(150, 225, 255),
    ],
    [
        rgb(34, 34, 40),
        rgb(74, 74, 82),
        rgb(150, 150, 158),
        rgb(232, 76, 60),
        rgb(255, 132, 112),
    ],
    [
        rgb(34, 34, 40),
        rgb(74, 74, 82),
        rgb(150, 150, 158),
        rgb(196, 196, 202),
        rgb(236, 236, 240),
    ],
];
pub fn palette(index: u32) -> &'static [u16; 5] {
    &PALETTES[index as usize % 5]
}
