// SPDX-License-Identifier: GPL-3.0-only
//! Foreground DJ state/buffer borrows, independent of worker storage.
use fm1_engine::{
    sequencer::Rng,
    voices::{dj, Mix},
};
use fm1_rt::atomic::{AtomicU32, Ordering};
static COUNTERS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.saturating_sub(72) as usize)
        .map_or(0, |v| v.load(Ordering::Relaxed))
}
#[no_mangle]
pub extern "C" fn fm1_rust_voice_dj_name(key: i32) -> *const u8 {
    dj::name(key).as_ptr()
}
/// # Safety
/// Four initialized/aligned48-byte voices and the age word are disjoint and
/// exclusively writable in the foreground command handler. No pointer,
/// reference or callback survives return. Rejected keys leave state unchanged.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_dj_note(
    states: *mut dj::State,
    age: *mut u32,
    key: i32,
    on: u32,
    octave: i32,
    hold: u32,
) {
    if dj::note(
        unsafe { &mut *states.cast::<[dj::State; 4]>() },
        unsafe { &mut *age },
        key,
        on != 0,
        octave,
        hold != 0,
    )
    .is_some()
    {
        COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    }
}
/// # Safety
/// Four initialized/aligned voices, immutable config and1025 sine values,
/// seed, four-pointer table and n-f32 outputs are live/disjoint. Voice/seed and
/// output regions are exclusively writable by the foreground for this call.
/// n<=256. Worker/tank/FM use separate buffers and independent RNG streams;
/// main control cannot mutate inputs while this call runs. No pointer,
/// reference or callback is retained, and no C callback occurs inside a borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_dj(
    states: *mut dj::State,
    c: *const dj::Config,
    seed: *mut u32,
    table: *const f32,
    outputs: *const *mut f32,
    n: u32,
) -> u32 {
    let p = unsafe { &*outputs.cast::<[*mut f32; 4]>() };
    let mut rng = Rng(unsafe { seed.read() });
    let n = n as usize;
    let result = dj::process(
        unsafe { &mut *states.cast::<[dj::State; 4]>() },
        unsafe { &*c },
        &mut rng,
        unsafe { &*table.cast::<[f32; 1025]>() },
        Mix {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        },
    );
    unsafe { seed.write(rng.0) };
    COUNTERS[1].fetch_add(result.samples, Ordering::Relaxed);
    result.active
}
const _: [(); 48] = [(); core::mem::size_of::<dj::State>()];
const _: [(); 20] = [(); core::mem::size_of::<dj::Config>()];
