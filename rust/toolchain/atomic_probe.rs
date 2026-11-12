// SPDX-License-Identifier: GPL-3.0-only
//! Private boot-time target atomic probe; not an instrument control path.
use fm1_rt::atomic::{fence, AtomicU32, Ordering};
static COUNTER: AtomicU32 = AtomicU32::new(0);
#[no_mangle]
pub extern "C" fn fm1_rust_atomic_reset() {
    COUNTER.store(0, Ordering::SeqCst);
}
#[no_mangle]
pub extern "C" fn fm1_rust_atomic_add(iterations: u32) -> u32 {
    for _ in 0..iterations {
        COUNTER.fetch_add(1, Ordering::Relaxed);
    }
    COUNTER.load(Ordering::Acquire)
}
#[no_mangle]
pub extern "C" fn fm1_rust_atomic_value() -> u32 {
    COUNTER.load(Ordering::Acquire)
}
#[no_mangle]
pub extern "C" fn fm1_rust_atomic_operations() -> u32 {
    let word = AtomicU32::new(u32::MAX);
    if word.fetch_add(1, Ordering::Relaxed) != u32::MAX || word.load(Ordering::Acquire) != 0 {
        return 0;
    }
    if word.fetch_sub(1, Ordering::SeqCst) != 0 {
        return 0;
    }
    if word.compare_exchange(0, 12, Ordering::AcqRel, Ordering::Acquire) != Err(u32::MAX) {
        return 0;
    }
    if word.compare_exchange_weak(u32::MAX, 12, Ordering::Release, Ordering::Relaxed)
        != Ok(u32::MAX)
    {
        return 0;
    }
    if word.fetch_max(8, Ordering::Relaxed) != 12 || word.fetch_or(3, Ordering::Relaxed) != 12 {
        return 0;
    }
    if word.fetch_and(7, Ordering::Relaxed) != 15 || word.fetch_xor(2, Ordering::Relaxed) != 7 {
        return 0;
    }
    if word.swap(4, Ordering::Relaxed) != 5 {
        return 0;
    }
    word.store(19, Ordering::Release);
    fence(Ordering::SeqCst);
    u32::from(word.load(Ordering::SeqCst) == 19)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operations_and_shared_counter_match_standard_rust_atomics() {
        assert_eq!(fm1_rust_atomic_operations(), 1);
        fm1_rust_atomic_reset();
        assert_eq!(fm1_rust_atomic_add(512), 512);
        assert_eq!(fm1_rust_atomic_add(512), 1024);
        assert_eq!(fm1_rust_atomic_value(), 1024);
    }
}
