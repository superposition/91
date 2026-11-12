// SPDX-License-Identifier: GPL-3.0-only
//! One instance on one browser audio thread. No callback while Audio is borrowed.
use core::cell::UnsafeCell;
use fm1_app::web_audio::Audio;

#[cfg(feature = "pi32v2")]
compile_error!("The browser audio owner cannot be enabled on the physical MCU");

struct Owner(UnsafeCell<Audio>);
// SAFETY: this feature's C adapter is serialized on one worklet thread. It may
// reenter Rust only after the prior call returns. C/JS output views are read-only
// and may be accessed only between calls, never by a concurrent worker.
unsafe impl Sync for Owner {}
static AUDIO: Owner = Owner(UnsafeCell::new(Audio::new()));

/// # Safety
/// Serialize this call with every other browser audio call/view access.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_available() -> u32 {
    // SAFETY: caller excludes mutation/view access for this call.
    unsafe { (&*AUDIO.0.get()).available() }
}
/// # Safety
/// block covers frames*2 initialized aligned i32 values, disjoint from Audio.
/// All browser audio calls/views are serialized; no callbacks or reentry.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_push(block: *const i32, frames: u32) -> i32 {
    if frames == 0 {
        return 0;
    }
    let Some(length) = (frames as usize).checked_mul(2) else {
        return -1;
    };
    if block.is_null() || !block.is_aligned() || length > isize::MAX as usize / 4 {
        return -1;
    }
    // SAFETY: caller's disjoint live source span and exclusive owner access.
    unsafe { (&mut *AUDIO.0.get()).push(core::slice::from_raw_parts(block, length)) as i32 }
}
/// # Safety
/// Serialize all browser audio calls/views; no callbacks or reentry.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_drain(frames: u32) -> i32 {
    // SAFETY: call-local exclusive borrow, released before returning to C.
    unsafe { (&mut *AUDIO.0.get()).drain(frames).map_or(-1, |n| n as i32) }
}
/// # Safety
/// Serialize the call. The returned 1024-float view is read-only; access it only
/// between serialized calls, never while Rust borrows Audio or from a worker.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_out_l() -> *const f32 {
    // SAFETY: caller excludes mutation; only a raw pointer survives the call.
    unsafe { (&*AUDIO.0.get()).left().as_ptr() }
}
/// # Safety
/// Same read-only view and serialization contract as fm1_rust_web_out_l.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_out_r() -> *const f32 {
    // SAFETY: caller excludes mutation; only a raw pointer survives the call.
    unsafe { (&*AUDIO.0.get()).right().as_ptr() }
}
/// # Safety
/// Serialize all calls/views. The returned 12-float view is read-only and may
/// be accessed only between calls. No borrow survives the returned raw pointer.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_bg(
    pad: u32,
    accent: u32,
    ground: u32,
    acid: u32,
) -> *const f32 {
    // SAFETY: exclusive owner access ends before the pointer reaches C/JS.
    unsafe {
        (&mut *AUDIO.0.get())
            .background([pad as u16, accent as u16, ground as u16, acid as u16])
            .as_ptr()
    }
}
