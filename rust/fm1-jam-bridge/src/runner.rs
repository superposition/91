// SPDX-License-Identifier: GPL-3.0-only
//! Synchronous game-state calls; no engine/platform callback while borrowed.
use crate::app::{font, frame_writes, RawFont};
use crate::app::{game_call, reject};
use core::mem::MaybeUninit;
use fm1_app::runner::{Observation, State};
use fm1_app::{graphics, runner_draw};
/// # Safety
/// state, observation and8 args are initialized aligned disjoint foreground-owned
/// packets, live for the call. No concurrent/reentrant game access. No retention.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_runner(
    op: u32,
    state: *mut State,
    observation: *const Observation,
    args: *mut [u32; 8],
) -> i32 {
    if state.is_null() || observation.is_null() || args.is_null() {
        return reject();
    }
    // SAFETY: complete initialized/aligned call-local packets and owned game state.
    let (state, clock, args) = unsafe { (&mut *state, *observation, &mut *args) };
    if !state.valid() {
        return reject();
    }
    game_call();
    match op {
        0 => {
            state.start(clock, [args[0], args[1], args[2]]);
            0
        }
        1 => {
            state.reset();
            0
        }
        2 => {
            if state.advance(clock).is_ok() {
                0
            } else {
                reject()
            }
        }
        3 => match state.resolve(args[0] as usize, clock.now) {
            Ok(v) => i32::from(v),
            Err(_) => reject(),
        },
        4 => {
            if state
                .steer(args[0] as i32, args[1] != 0, args[2] != 0)
                .is_ok()
            {
                0
            } else {
                reject()
            }
        }
        5 => match state.masks(args[0], &[args[1], args[2], args[3], args[4], args[5]]) {
            Ok(v) => {
                args[6] = v[0];
                args[7] = v[1];
                0
            }
            Err(_) => reject(),
        },
        7 => {
            state.on = 0;
            0
        }
        8 => i32::from(state.restart_candidate(args[0], args[1])),
        9 => i32::from(state.restart(clock.now)),
        10 => State::buttons(args[0], args[1], [args[2], args[3], args[4], args[5]]) as i32,
        _ => reject(),
    }
}
/// # Safety
/// All packets cover their complete initialized/aligned spans: params4 words,
/// glow count<=11 floats, white8/black11 bytes, out2 words; disjoint writable out.
/// Game state is immutable for this call. No callback or retained pointer.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_runner_leds(
    state: *const State,
    params: *const [u32; 4],
    glow: *const f32,
    count: u32,
    white: *const [u8; 8],
    black: *const [u8; 11],
    out: *mut [u32; 2],
) -> i32 {
    if state.is_null()
        || params.is_null()
        || white.is_null()
        || black.is_null()
        || out.is_null()
        || count > 11
        || (glow.is_null() && count != 0)
    {
        return reject();
    }
    // SAFETY: caller's initialized fixed packets and bounded copied glow values.
    let (state, params, white, black) = unsafe { (&*state, &*params, &*white, &*black) };
    if !state.valid() {
        return reject();
    }
    let glow = if count == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(glow, count as usize) }
    };
    game_call();
    match state.leds(
        params[0] != 0,
        params[1],
        [params[2], params[3]],
        glow,
        white,
        black,
    ) {
        Ok(value) => {
            unsafe { out.write(value) };
            0
        }
        Err(_) => reject(),
    }
}

/// # Safety
/// All descriptors/game/theme/fonts are initialized aligned immutable/disjoint
/// from the writable canvas state/pool. pixels has capacity exclusive u16 slots;
/// LCD DMA completed or was confirmed stopped before entry. Only the active
/// initialized prefix becomes a typed slice. Foreground only; no callback,
/// retention or reentrant game/canvas access during this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_runner_draw(
    state: *const State,
    now: u32,
    playing: u32,
    half: u32,
    pixels: *mut u16,
    capacity: u32,
    canvas_state: *mut graphics::State,
    theme: *const [u16; 4],
    fonts: *const [RawFont; 3],
) -> i32 {
    if state.is_null()
        || pixels.is_null()
        || canvas_state.is_null()
        || theme.is_null()
        || fonts.is_null()
        || capacity < 240 * 120
        || capacity as usize > graphics::MAX_PIXELS
        || half > 1
    {
        return reject();
    }
    // SAFETY: caller's fully initialized immutable game/descriptor packets.
    let (state, canvas_state, theme) = unsafe { (&*state, &mut *canvas_state, *theme) };
    if !runner_draw::valid(state) {
        return reject();
    }
    let (font0, font1, font2) = unsafe {
        (
            font(fonts.cast()),
            font(fonts.cast::<RawFont>().add(1)),
            font(fonts.cast::<RawFont>().add(2)),
        )
    };
    let (Ok(font0), Ok(font1), Ok(font2)) = (font0, font1, font2) else {
        return reject();
    };
    // SAFETY: exclusive uninitialized pool is borrowed only until begin writes
    // the complete active prefix; the inactive tail is never read as u16.
    let next = {
        let pool = unsafe {
            core::slice::from_raw_parts_mut(pixels.cast::<MaybeUninit<u16>>(), capacity as usize)
        };
        graphics::initialize(
            pool,
            240,
            120,
            runner_draw::background(state),
            canvas_state.scan,
        )
    };
    let Ok(mut next) = next else {
        return reject();
    };
    next.origin = if half == 0 { 0 } else { -120 };
    // SAFETY: previous MaybeUninit borrow ended and all28,800 active pixels are initialized.
    let buffer = unsafe { core::slice::from_raw_parts_mut(pixels, 240 * 120) };
    let Ok(mut canvas) = graphics::Canvas::new(buffer, next) else {
        return reject();
    };
    if runner_draw::draw(
        &mut canvas,
        state,
        now,
        playing != 0,
        theme,
        [&font0, &font1, &font2],
    )
    .is_err()
    {
        return reject();
    }
    *canvas_state = canvas.state();
    frame_writes(240 * 120 + canvas.writes());
    game_call();
    0
}
