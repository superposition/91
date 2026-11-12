// SPDX-License-Identifier: GPL-3.0-only
//! Scoped ABI calls into the render-owned transport; no pointer is retained.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::transport::{self, Transport};

static PULSES: AtomicU32 = AtomicU32::new(0);
static CHUNKS: AtomicU32 = AtomicU32::new(0);
static CONTROLS: AtomicU32 = AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn fm1_rust_transport_step_length(tempo: u32) -> f32 {
    transport::step_length(tempo as u16)
}

/// # Safety
/// `state` is aligned, initialized (all-zero is valid), live, and exclusively
/// owned by the serialized render caller for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_start(state: *mut Transport, from_top: u32) {
    unsafe { &mut *state }.start(from_top != 0);
}

/// # Safety
/// The same exclusive initialized state contract as `fm1_rust_transport_start`.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_pulse(state: *mut Transport) {
    unsafe { &mut *state }.finish_pulse();
    PULSES.fetch_add(1, Ordering::Relaxed);
}

/// # Safety
/// `state` is aligned, initialized, live, readable, and not concurrently mutated.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_tick(state: *const Transport) -> u32 {
    u32::from(unsafe { &*state }.at_sixteenth())
}

/// # Safety
/// The same exclusive initialized state contract as `fm1_rust_transport_start`.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_receive(state: *mut Transport) {
    unsafe { &mut *state }.receive_external();
}

/// # Safety
/// The same exclusive initialized state contract as `fm1_rust_transport_start`.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_expired(state: *mut Transport, external: u32) -> u32 {
    u32::from(unsafe { &mut *state }.external_expired(external != 0))
}

/// # Safety
/// `state` is aligned, initialized, live, readable, and not concurrently mutated.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_due(state: *const Transport) -> u32 {
    u32::from(unsafe { &*state }.pulse_due())
}

/// # Safety
/// The same exclusive initialized state contract as `fm1_rust_transport_start`.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_schedule(state: *mut Transport, step_length: f32) {
    unsafe { &mut *state }.schedule_next(step_length);
}

/// # Safety
/// `state` is aligned, initialized, live, readable, and not concurrently mutated.
/// The caller schedules a due pulse before requesting the chunk boundary.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_clock_limit(
    state: *const Transport,
    frames: u32,
    step_length: f32,
    coalesce: u32,
) -> u32 {
    unsafe { &*state }.clock_limit(frames, step_length, coalesce != 0)
}

#[no_mangle]
pub extern "C" fn fm1_rust_transport_wait_limit(frames: u32, wait: i32, minimum: u32) -> u32 {
    transport::wait_limit(frames, wait, minimum)
}

/// # Safety
/// The same exclusive initialized state contract as `fm1_rust_transport_start`.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_transport_advance(
    state: *mut Transport,
    frames: u32,
    timed: u32,
) -> u32 {
    CHUNKS.fetch_add(1, Ordering::Relaxed);
    if let Some(elapsed) = unsafe { &mut *state }.advance(frames, timed != 0) {
        CONTROLS.fetch_add(1, Ordering::Relaxed);
        elapsed
    } else {
        0
    }
}

pub fn stat(index: u32) -> u32 {
    match index {
        10 => PULSES.load(Ordering::Relaxed),
        11 => CHUNKS.load(Ordering::Relaxed),
        12 => CONTROLS.load(Ordering::Relaxed),
        _ => 0,
    }
}
