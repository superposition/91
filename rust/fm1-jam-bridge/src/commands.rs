// SPDX-License-Identifier: GPL-3.0-only
//! C render/application roles call scoped operations; no C reference is kept.
use fm1_rt::atomic::{AtomicU32, Ordering};
use fm1_rt::CommandQueue;
use fm1_types::CommandWord;

static QUEUE: CommandQueue = CommandQueue::new();
static ACCEPTED: AtomicU32 = AtomicU32::new(0);
static CONSUMED: AtomicU32 = AtomicU32::new(0);
static DROPPED: AtomicU32 = AtomicU32::new(0);
static HIGH_WATER: AtomicU32 = AtomicU32::new(0);

/// # Safety
/// Boot only, with all producer/consumer access excluded.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_commands_reset() {
    // SAFETY: the C boot owner excludes render/input calls during initialization.
    unsafe { QUEUE.reset_exclusive() };
    // SAFETY: the same boot exclusion covers staging and snapshot users.
    unsafe { super::handoff::reset_exclusive() };
    for counter in [&ACCEPTED, &CONSUMED, &DROPPED, &HIGH_WATER] {
        counter.store(0, Ordering::Relaxed);
    }
}

/// # Safety
/// One serialized producer; no reset and at most one concurrent consumer.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_commands_push(bits: u32) -> i32 {
    // SAFETY: the emulator serializes input/render. The MCU post adapter masks
    // CPU0 IRQs for this entire call: render-side Naiwan notes can also post.
    // CPU1 uses a separate deferred MIDI path and never publishes here.
    match unsafe { QUEUE.push_from_producer(CommandWord::from_bits(bits)) } {
        Ok(depth) => {
            ACCEPTED.fetch_add(1, Ordering::Relaxed);
            HIGH_WATER.fetch_max(depth, Ordering::Relaxed);
            1
        }
        Err(_) => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
            0
        }
    }
}

/// # Safety
/// One render consumer, no reset. `output` is writable, initialized on success,
/// and does not alias queue storage or a concurrently accessed allocation.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_commands_pop(output: *mut u32) -> i32 {
    if output.is_null() {
        return 0;
    }
    // SAFETY: the C render owner is the unique consumer. The copied word no
    // longer borrows a slot when C dispatches it or posts a subsequent command.
    if let Some(word) = unsafe { QUEUE.pop_from_consumer() } {
        unsafe { output.write_unaligned(word.bits()) };
        CONSUMED.fetch_add(1, Ordering::Relaxed);
        1
    } else {
        0
    }
}

pub fn stat(index: u32) -> u32 {
    match index {
        6 => ACCEPTED.load(Ordering::Relaxed),
        7 => CONSUMED.load(Ordering::Relaxed),
        8 => DROPPED.load(Ordering::Relaxed),
        9 => HIGH_WATER.load(Ordering::Relaxed),
        _ => 0,
    }
}
