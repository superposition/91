// SPDX-License-Identifier: GPL-3.0-only
//! Primitive calls never invoke a callback or retain a font/pixel/state borrow.
use core::mem::MaybeUninit;
use fm1_app::graphics::{self, Canvas, Font, State, MAX_PIXELS, TEXT_MAX};
use fm1_rt::atomic::{AtomicU32, Ordering};
static COUNTS: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
pub fn stat(index: u32) -> u32 {
    COUNTS
        .get(index.wrapping_sub(80) as usize)
        .map_or(0, |c| c.load(Ordering::Relaxed))
}
pub(crate) fn reject() -> i32 {
    COUNTS[3].fetch_add(1, Ordering::Relaxed);
    -1
}
pub(crate) fn game_call() {
    COUNTS[2].fetch_add(1, Ordering::Relaxed);
}
pub(crate) fn frame_writes(writes: u32) {
    COUNTS[0].fetch_add(1, Ordering::Relaxed);
    COUNTS[1].fetch_add(writes, Ordering::Relaxed);
}
#[repr(C)]
pub struct RawFont {
    height: u32,
    pad: u32,
    first: u32,
    last: u32,
    advance: *const u8,
    width: *const u8,
    offset: *const u16,
    data: *const u8,
    length: u32,
}
pub(crate) unsafe fn font<'a>(raw: *const RawFont) -> Result<Font<'a>, ()> {
    if raw.is_null() {
        return Err(());
    }
    // SAFETY: caller's initialized font descriptor and immutable table spans.
    let f = unsafe { &*raw };
    if f.height > 255
        || f.pad > 255
        || f.first > 255
        || f.last > 255
        || f.last < f.first
        || f.length > 100_000
        || f.advance.is_null()
        || f.width.is_null()
        || f.offset.is_null()
        || f.data.is_null()
    {
        return Err(());
    }
    let n = (f.last - f.first + 1) as usize;
    // SAFETY: C constructs the descriptor from live immutable generated fonts;
    // n table entries and length data bytes are readable, offsets aligned.
    unsafe {
        Font::new(
            f.height as u8,
            f.pad as u8,
            f.first as u8,
            f.last as u8,
            core::slice::from_raw_parts(f.advance, n),
            core::slice::from_raw_parts(f.width, n),
            core::slice::from_raw_parts(f.offset, n),
            core::slice::from_raw_parts(f.data, f.length as usize),
        )
        .map_err(|_| ())
    }
}
/// # Safety
/// state/args are initialized/aligned and disjoint. pixels identifies capacity
/// exclusive u16 slots (possibly uninitialized only for op0). Other operations
/// require the active width*height prefix initialized by begin. DMA must already
/// have completed or been confirmed stopped before any mutating call.
/// Text/descriptor/font tables cover
/// their declared initialized spans, are immutable/disjoint from state/pixels,
/// and live throughout this call. No callback, reentry or retained borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_canvas(
    op: u32,
    pixels: *mut u16,
    capacity: u32,
    state: *mut State,
    args: *const [i32; 6],
    raw_font: *const RawFont,
    text: *const u8,
    length: u32,
) -> i32 {
    COUNTS[0].fetch_add(1, Ordering::Relaxed);
    if state.is_null()
        || args.is_null()
        || capacity as usize > MAX_PIXELS
        || (pixels.is_null() && capacity != 0)
    {
        return reject();
    }
    // SAFETY: initialized/aligned call-local descriptors, no pointer retained.
    let (state, args) = unsafe { (&mut *state, &*args) };
    if op == 0 {
        // Only MaybeUninit is borrowed until every active pixel has been written.
        // The hardware pool is not assumed to be zero-initialized at reset.
        let buffer = if capacity == 0 {
            &mut []
        } else {
            unsafe {
                core::slice::from_raw_parts_mut(
                    pixels.cast::<MaybeUninit<u16>>(),
                    capacity as usize,
                )
            }
        };
        return match graphics::initialize(
            buffer,
            args[0] as u32,
            args[1] as u32,
            args[2] as u16,
            state.scan,
        ) {
            Ok(next) => {
                *state = next;
                COUNTS[1].fetch_add(next.width * next.height, Ordering::Relaxed);
                0
            }
            Err(_) => reject(),
        };
    }
    if op == 1 {
        return if state.band(args[0], args[1]).is_ok() {
            0
        } else {
            reject()
        };
    }
    if op == 5 && state.scan == 0 {
        return 0;
    }
    if length as usize > TEXT_MAX || (text.is_null() && length != 0) {
        return reject();
    }
    let Some(active) = state.width.checked_mul(state.height) else {
        return reject();
    };
    if active > capacity {
        return reject();
    }
    // SAFETY: C guarantees that the active prefix was initialized by begin and
    // DMA completed before entry; the inactive pool tail is never referenced.
    let buffer = if active == 0 {
        &mut []
    } else {
        unsafe { core::slice::from_raw_parts_mut(pixels, active as usize) }
    };
    let Ok(mut canvas) = Canvas::new(buffer, *state) else {
        return reject();
    };
    let result = match op {
        2 => {
            canvas.pixel(args[0], args[1], args[2] as u16);
            0
        }
        3 => {
            canvas.rectangle(args[0], args[1], args[2], args[3], args[4] as u16);
            0
        }
        4 => {
            canvas.line(args[0], args[1], args[2], args[3], args[4] as u16);
            0
        }
        5 => {
            canvas.scanlines(args[0] as u32, args[1] as u32);
            0
        }
        6 => {
            // SAFETY: font and text spans are initialized, immutable, distinct
            // from pixels/state, and live until this function returns.
            let Ok(font) = (unsafe { font(raw_font) }) else {
                return reject();
            };
            let bytes = if length == 0 {
                &[]
            } else {
                unsafe { core::slice::from_raw_parts(text, length as usize) }
            };
            match canvas.text(args[0], args[1], &font, bytes, args[2] as u16) {
                Ok(x) => x,
                Err(_) => return reject(),
            }
        }
        8 => {
            canvas.blend16(args[0], args[1], args[2] as u16, args[3] as u32);
            0
        }
        _ => return reject(),
    };
    COUNTS[1].fetch_add(canvas.writes(), Ordering::Relaxed);
    result
}
/// # Safety
/// Font/descriptor/text spans are initialized immutable readable ranges, live
/// throughout the call; tables match their counts and data length. No retention.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_text_width(
    raw: *const RawFont,
    text: *const u8,
    length: u32,
) -> i32 {
    COUNTS[0].fetch_add(1, Ordering::Relaxed);
    if length as usize > TEXT_MAX || (text.is_null() && length != 0) {
        return reject();
    }
    // SAFETY: caller's immutable bounded text/font spans, no callback.
    let Ok(font) = (unsafe { font(raw) }) else {
        return reject();
    };
    let bytes = if length == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(text, length as usize) }
    };
    font.text_width(bytes).unwrap_or_else(|_| reject())
}
/// # Safety
/// output covers5 distinct writable initialized u16s, no concurrent access.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_palette(index: u32, output: *mut [u16; 5]) {
    if output.is_null() {
        reject();
        return;
    }
    // SAFETY: fixed exclusive destination, initialized immutable palette source.
    unsafe { output.write(*graphics::palette(index)) };
}
