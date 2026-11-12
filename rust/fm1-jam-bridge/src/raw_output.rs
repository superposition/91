// SPDX-License-Identifier: GPL-3.0-only
//! Sealed DMA publication: no reference into the external audio allocation is
//! formed, including ordinary output, integer fades and silence. Device readers
//! may advance after any check. Only copied bounded words cross this boundary.
use crate::platform::Mmio;
use core::cell::UnsafeCell;
use fm1_hal::{
    dma::{Audio, AudioIo, Phase, Token},
    drivers,
};
use fm1_rt::atomic::{AtomicU32, Ordering};
struct Runtime {
    audio: Audio,
    base: *mut i32,
    paused: bool,
}
#[cfg(target_pointer_width = "32")]
const _: [(); 24] = [(); core::mem::size_of::<Runtime>()];
struct Cell(UnsafeCell<Runtime>);
// SAFETY: registration and all active-mode operations are CPU0 only. Configure
// runs with IRQs disabled and no active sink; begin/end surround render/drain;
// nested TIMER5/USB/CPU1 never enter this module or access this private state.
// The active flag uses the platform's real atomic ABI, not a volatile substitute.
unsafe impl Sync for Cell {}
static STATE: Cell = Cell(UnsafeCell::new(Runtime {
    audio: Audio::new(),
    base: core::ptr::null_mut(),
    paused: false,
}));
static ACTIVE: AtomicU32 = AtomicU32::new(0);
extern "C" {
    fn fm1_hal_audio_stop_fence();
    fn fm1_hal_audio_stop_confirm() -> u32;
}
#[cfg(feature = "hal-test")]
extern "C" {
    fn fm1_hal_test_store_step(index: u32, value: i32);
}
struct Bus {
    base: *mut i32,
}
impl AudioIo for Bus {
    fn free_half(&mut self) -> u32 {
        drivers::audio_free_half(&mut Mmio)
    }
    fn store_word(&mut self, index: u32, word: i32) {
        // SAFETY: Audio validates the retained two-half index before this call;
        // registration retains the aligned allocation through unknown stop.
        unsafe {
            self.base.add(index as usize).write_volatile(word);
        }
        #[cfg(feature = "hal-test")]
        unsafe {
            fm1_hal_test_store_step(index, word);
        }
    }
    fn request_stop(&mut self) {
        drivers::audio_request_stop(&mut Mmio);
        unsafe {
            fm1_hal_audio_stop_fence();
        }
    }
    fn stopped(&mut self) -> bool {
        unsafe { fm1_hal_audio_stop_confirm() != 0 }
    }
}
pub(crate) struct Sink<'a> {
    inner: Kind<'a>,
}
enum Kind<'a> {
    Plain {
        out: *mut i32,
        words: usize,
    },
    Dma {
        runtime: &'a mut Runtime,
        bus: Bus,
        token: Token,
        first: u32,
        words: usize,
        out: *mut i32,
    },
    Rejected,
}
impl<'a> Sink<'a> {
    /// # Safety
    /// Output is an aligned live external span of `words` i32s, disjoint from
    /// DSP/USB inputs/state. No Rust reference to it is live. In active hardware
    /// mode the caller obeys the CPU0 registration/render transaction contract.
    pub(crate) unsafe fn new(out: *mut i32, words: usize) -> Self {
        Self {
            inner: unsafe { Self::create(out, words) },
        }
    }
    unsafe fn create(out: *mut i32, words: usize) -> Kind<'a> {
        if ACTIVE.load(Ordering::Acquire) == 0 {
            return Kind::Plain { out, words };
        }
        let runtime = unsafe { &mut *STATE.0.get() };
        let base = runtime.base.addr();
        let total = runtime.audio.half_words() as usize * 2 * 4;
        // A failed first registration has no admitted external allocation.
        // It must not turn the unknown output back into a plain writable sink.
        if runtime.base.is_null() || total == 0 {
            return Kind::Rejected;
        }
        let Some(end) = (out.addr()).checked_add(words.saturating_mul(4)) else {
            return Kind::Rejected;
        };
        let mut bus = Bus { base: runtime.base };
        if end <= base || out.addr() >= base + total {
            return Kind::Plain { out, words };
        }
        let Some(token) = runtime.audio.token() else {
            return Kind::Rejected;
        };
        let start = base + token.half() as usize * runtime.audio.half_words() as usize * 4;
        if (out.addr()) < start
            || end > start + runtime.audio.half_words() as usize * 4
            || (out.addr()) & 3 != 0
        {
            runtime.audio.abort(&mut bus);
            return Kind::Rejected;
        }
        Kind::Dma {
            runtime,
            bus,
            token,
            first: ((out.addr() - start) / 4) as u32,
            words,
            out,
        }
    }
    pub(crate) fn write(&mut self, index: usize, value: i32) -> bool {
        match &mut self.inner {
            Kind::Plain { out, words } => {
                if index >= *words {
                    return false;
                }
                unsafe {
                    out.add(index).write_volatile(value);
                }
                true
            }
            Kind::Dma {
                runtime,
                bus,
                token,
                first,
                words,
                ..
            } => {
                index < *words
                    && runtime
                        .audio
                        .publish(bus, *token, *first + index as u32, value)
            }
            Kind::Rejected => false,
        }
    }
    pub(crate) fn read(&mut self, index: usize) -> Option<i32> {
        match &mut self.inner {
            Kind::Plain { out, words } => {
                if index >= *words {
                    None
                } else {
                    Some(unsafe { out.add(index).read_volatile() })
                }
            }
            Kind::Dma {
                runtime,
                bus,
                token,
                words,
                out,
                ..
            } => {
                if index >= *words || !runtime.audio.check(bus, *token) {
                    None
                } else {
                    Some(unsafe { out.add(index).read_volatile() })
                }
            }
            Kind::Rejected => None,
        }
    }
}
/// # Safety
/// CPU0, IRQs disabled, no active sink/call. Two aligned retained halves exist
/// for the complete DMA lifetime, including quarantine; quiescent is a genuinely
/// backend-confirmed fact. Initialize their words before starting DMA/render.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_config(
    base: *mut i32,
    half_words: u32,
    quiescent: u32,
) -> u32 {
    let runtime = unsafe { &mut *STATE.0.get() };
    let address = base.addr();
    let shape = [512, 1024].contains(&half_words)
        && address & 3 == 0
        && !base.is_null()
        && address.checked_add(half_words as usize * 8).is_some();
    #[cfg(not(feature = "hal-test"))]
    let shape = shape && address >= 0x01c00000 && address + half_words as usize * 8 <= 0x01c74000;
    let allowed = runtime.audio.configure(half_words, quiescent != 0 && shape);
    if allowed {
        runtime.base = base;
        runtime.paused = false;
    }
    ACTIVE.store(1, Ordering::Release);
    u32::from(allowed)
}
/// # Safety
/// Registration succeeded on the valid physical HAL map or explicit fake bus.
/// Only the serialized CPU0 audio ISR calls begin/end. No nested code can enter
/// an active sink. Generation is a stale-call check, not a DMA exclusion lease.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_begin(half: u32) -> u32 {
    let runtime = unsafe { &mut *STATE.0.get() };
    match runtime.audio.begin(half) {
        Some(t) => t.generation(),
        None => {
            let mut bus = Bus { base: runtime.base };
            runtime.audio.retire(&mut bus);
            0
        }
    }
}
/// # Safety
/// Same CPU0 synchronous transaction as begin; no output borrow is live.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_end(generation: u32) -> u32 {
    let runtime = unsafe { &mut *STATE.0.get() };
    let mut bus = Bus { base: runtime.base };
    match runtime.audio.token() {
        Some(t) if t.generation() == generation => u32::from(runtime.audio.finish(&mut bus, t)),
        _ => 0,
    }
}
/// # Safety
/// CPU0 IRQ exclusion covers this control query and any subsequent zero/reinit.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_reusable() -> u32 {
    if ACTIVE.load(Ordering::Acquire) == 0 {
        return 1;
    }
    let runtime = unsafe { &mut *STATE.0.get() };
    let mut bus = Bus { base: runtime.base };
    u32::from(runtime.audio.poll_stopped(&mut bus))
}
/// # Safety
/// CPU0 only, outside live sink borrows. Used to stop a second render after loss.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_rendering() -> u32 {
    u32::from(unsafe { (&*STATE.0.get()).audio.phase() == Phase::Rendering })
}
/// # Safety
/// Same output/CPU0 contract as Sink::new. USB is deliberately not touched here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_silence(out: *mut i32, frames: u32) {
    let mut sink = unsafe { Sink::new(out, frames as usize * 2) };
    for i in 0..frames as usize * 2 {
        sink.write(i, 0);
    }
}
#[cfg(feature = "hal-test")]
/// # Safety
/// Host harness has no live Rust/hardware sink and confirms peripheral reset.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_test_reset() {
    unsafe {
        STATE.0.get().write(Runtime {
            audio: Audio::new(),
            base: core::ptr::null_mut(),
            paused: false,
        });
    }
    ACTIVE.store(0, Ordering::Release);
}

/// # Safety
/// CPU0 IRQs are masked, no engine/sink borrow is live. Only an explicit mute
/// from Idle may restart automatically; an earlier overrun remains latched.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_pause() -> u32 {
    if ACTIVE.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let runtime = unsafe { &mut *STATE.0.get() };
    if runtime.audio.phase() != Phase::Idle {
        return 0;
    }
    let mut bus = Bus { base: runtime.base };
    runtime.audio.abort(&mut bus);
    runtime.paused = runtime.audio.phase() == Phase::Stopped;
    u32::from(runtime.paused)
}
/// # Safety
/// Called only after a successful pause and completed raw zeroing, with CPU0 IRQ
/// exclusion maintained. The caller enables the existing DMA configuration only
/// after this returns1; no other bus actor may retain an unconfirmed access.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_restart() -> u32 {
    if ACTIVE.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let runtime = unsafe { &mut *STATE.0.get() };
    if !runtime.paused {
        return 0;
    }
    runtime.paused = false;
    let words = runtime.audio.half_words();
    u32::from(runtime.audio.configure(words, true))
}

/// # Safety
/// The C0 DSP adapter supplies one aligned initialized external output word,
/// disjoint from live DSP state and under the same CPU0 HAL transaction.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_audio_write_word(out: *mut i32, value: i32) {
    let mut sink = unsafe { Sink::new(out, 1) };
    sink.write(0, value);
}

#[cfg(test)]
mod link_fallback {
    #[no_mangle]
    extern "C" fn fm1_hal_audio_stop_fence() {}
    #[no_mangle]
    extern "C" fn fm1_hal_audio_stop_confirm() -> u32 {
        0
    }
    #[cfg(feature = "hal-test")]
    static CONTROL: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0x8800);
    #[cfg(feature = "hal-test")]
    #[no_mangle]
    extern "C" fn fm1_hal_test_mmio(kind: u32, address: u32, value: u32) -> u32 {
        use core::sync::atomic::Ordering;
        assert_eq!(address, 0x12e00);
        match kind {
            16 => CONTROL.load(Ordering::Relaxed),
            144 => {
                CONTROL.store(value, Ordering::Relaxed);
                0
            }
            _ => panic!("unexpected MMIO width"),
        }
    }
    #[cfg(feature = "hal-test")]
    #[no_mangle]
    extern "C" fn fm1_hal_test_store_step(_index: u32, _value: i32) {}
    #[cfg(feature = "hal-test")]
    #[test]
    fn retained_dma_root_pointer_remains_valid_between_call_scoped_updates() {
        use super::*;
        let allocation = core::cell::UnsafeCell::new([0i32; 2048]);
        let out = allocation.get().cast::<i32>();
        let left = [0.1f32, 0.2, 0.3, 0.4];
        let right = [-0.1f32, -0.2, -0.3, -0.4];
        let mut expected = [0; 8];
        fm1_engine::effects::output(&left, &right, &mut expected, None, 1.0, 1.0, false);
        // SAFETY: a single serialized owner retains this initialized allocation;
        // no Rust slice/reference to the DMA words survives any foreign call.
        unsafe {
            fm1_rust_hal_test_reset();
            assert_eq!(fm1_rust_hal_audio_config(out, 1024, 1), 1);
            let generation = fm1_rust_hal_audio_begin(0);
            assert_ne!(generation, 0);
            crate::effects::fm1_rust_fx_output(
                left.as_ptr(),
                right.as_ptr(),
                4,
                out,
                core::ptr::null_mut(),
                1.0,
                1.0,
                0,
            );
            for (i, value) in expected.iter().enumerate() {
                assert_eq!(out.add(i).read(), *value);
            }
            let mut state = [1.0, 1.0];
            crate::effects::fm1_rust_fx_fades(
                state.as_mut_ptr(),
                1,
                1,
                out,
                core::ptr::null_mut(),
                4,
            );
            assert_ne!(out.read(), expected[0]);
            assert_eq!(fm1_rust_hal_audio_end(generation), 1);
            // This explicit fake-peripheral reset ends all device access before
            // the stack allocation is dropped. Production requires stop proof.
            fm1_rust_hal_test_reset();
        }
    }
}
