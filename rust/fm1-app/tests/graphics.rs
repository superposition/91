// SPDX-License-Identifier: GPL-3.0-only
use core::mem::MaybeUninit;
use fm1_app::{graphics::*, text::Text};
#[test]
fn first_strip_initializes_only_the_active_pool_prefix() {
    let mut pool = [MaybeUninit::<u16>::uninit(); 40];
    pool[0].write(0x1234);
    pool[39].write(0xabcd);
    let state = initialize(&mut pool[1..39], 5, 7, 0x1f, 0).unwrap();
    assert_eq!((state.width, state.height), (5, 7));
    // SAFETY: the two canaries were initialized above and35 active pixels by begin.
    unsafe {
        assert_eq!(pool[0].assume_init(), 0x1234);
        assert_eq!(pool[39].assume_init(), 0xabcd);
        for p in &pool[1..36] {
            assert_eq!(p.assume_init(), 0x1f00);
        }
    }
}
#[test]
fn hostile_coordinates_and_state_keep_canaries_and_capacity() {
    let mut bytes = [0x55aa; 66];
    let state = State {
        width: 8,
        height: 8,
        origin: 0,
        top: 0,
        bottom: 8,
        scan: 1,
    };
    let mut canvas = Canvas::new(&mut bytes[1..65], state).unwrap();
    canvas.line(i32::MIN, i32::MAX, i32::MAX, i32::MIN, 0xffff);
    canvas.rectangle(i32::MIN, i32::MIN, i32::MAX, i32::MAX, 0);
    canvas.pixel(8, 0, 0);
    canvas.pixel(0, 8, 0);
    canvas.blend16(-1, 0, 0xffff, 15);
    canvas.scanlines(u32::MAX, u32::MAX);
    assert_eq!(bytes[0], 0x55aa);
    assert_eq!(bytes[65], 0x55aa);
    assert!(Canvas::new(
        &mut bytes,
        State {
            width: u32::MAX,
            height: 2,
            ..state
        }
    )
    .is_err());
    assert!(Canvas::new(&mut bytes, State { top: -1, ..state }).is_err());
    // Empty strips still have bounded dimensions/work, not billions of rows.
    assert!(State::begin(0, u32::MAX, 0, 1).is_err());
    assert!(State::begin(u32::MAX, 0, 0, 1).is_err());
}
#[test]
fn font_offsets_and_text_are_checked_before_writes() {
    let (advance, width, offset) = ([3; 64], [1; 64], [0; 64]);
    let data = [0xf0];
    let font = Font::new(1, 0, 32, 95, &advance, &width, &offset, &data).unwrap();
    assert_eq!(font.glyph(b'a'), font.glyph(b'A'));
    assert_eq!(font.glyph(128), font.glyph(b'?'));
    assert_eq!(font.text_width(b"Ab\0ignored"), Ok(6));
    assert!(Font::new(2, 0, 32, 95, &advance, &width, &offset, &data).is_err());
    assert!(Font::new(1, 0, 64, 127, &advance, &width, &offset, &data).is_err());
    let mut pixels = [0x5a5a; 16];
    let mut canvas = Canvas::begin(&mut pixels, 4, 4, 0, 0).unwrap();
    assert_eq!(
        canvas.text(0, 0, &font, &[b'A'; 257], 0xffff),
        Err(Error::Text)
    );
    assert_eq!(canvas.writes(), 16);
}
#[test]
fn bounded_bytes_keep_nul_and_handle_full_width_signed_values() {
    let mut output = [0xaa; 16];
    let mut text = Text::new(&mut output);
    text.signed(i32::MIN);
    assert_eq!(text.bytes(), b"-2147483648");
    assert!(!text.truncated());
    let mut small = [0xaa; 5];
    let mut text = Text::new(&mut small);
    text.capitals(b"aBcdef");
    assert_eq!(text.bytes(), b"ABCD");
    assert!(text.truncated());
    assert_eq!(small[4], 0);
    let mut empty = [];
    let mut text = Text::new(&mut empty);
    text.unsigned(u32::MAX);
    assert_eq!(text.bytes(), b"");
    assert!(text.truncated());
}
