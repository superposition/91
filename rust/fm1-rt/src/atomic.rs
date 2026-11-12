// SPDX-License-Identifier: GPL-3.0-only
//! Target atomics. PI32v2 has no rustc atomic target; its checked HAL backend
//! supplies stronger sequentially consistent operations through one ABI.
pub use core::sync::atomic::Ordering;
#[cfg(not(feature = "pi32v2"))]
pub use core::sync::atomic::{fence, AtomicU32};

#[cfg(feature = "pi32v2")]
mod mcu {
    use super::Ordering;
    use core::cell::UnsafeCell;
    extern "C" {
        fn fm1_mcu_atomic(operation: u32, word: *mut u32, expected: u32, value: u32) -> u32;
        fn fm1_mcu_fence();
    }
    #[repr(transparent)]
    pub struct AtomicU32(UnsafeCell<u32>);
    // SAFETY: every shared access uses the same HAL protocol. Its cross-core
    // hardware lock, IRQ save/restore and barriers provide SC even for Relaxed.
    // No direct Rust reference into the cell is ever exposed.
    unsafe impl Sync for AtomicU32 {}
    const _: [(); 4] = [(); core::mem::size_of::<AtomicU32>()];
    const _: [(); 4] = [(); core::mem::align_of::<AtomicU32>()];
    impl AtomicU32 {
        pub const fn new(value: u32) -> Self {
            Self(UnsafeCell::new(value))
        }
        fn apply(&self, operation: u32, expected: u32, value: u32) -> u32 {
            // SAFETY: this cell stays aligned/live for the synchronous call.
            // All accesses, including from the other core, use the HAL ABI.
            unsafe { fm1_mcu_atomic(operation, self.0.get(), expected, value) }
        }
        pub fn load(&self, order: Ordering) -> u32 {
            assert!(matches!(
                order,
                Ordering::Relaxed | Ordering::Acquire | Ordering::SeqCst
            ));
            self.apply(0, 0, 0)
        }
        pub fn store(&self, value: u32, order: Ordering) {
            assert!(matches!(
                order,
                Ordering::Relaxed | Ordering::Release | Ordering::SeqCst
            ));
            self.apply(1, 0, value);
        }
        pub fn fetch_add(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(2, 0, value)
        }
        pub fn fetch_sub(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(3, 0, value)
        }
        pub fn fetch_max(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(4, 0, value)
        }
        pub fn compare_exchange(
            &self,
            current: u32,
            new: u32,
            _success: Ordering,
            failure: Ordering,
        ) -> Result<u32, u32> {
            assert!(matches!(
                failure,
                Ordering::Relaxed | Ordering::Acquire | Ordering::SeqCst
            ));
            let old = self.apply(5, current, new);
            if old == current {
                Ok(old)
            } else {
                Err(old)
            }
        }
        pub fn compare_exchange_weak(
            &self,
            current: u32,
            new: u32,
            success: Ordering,
            failure: Ordering,
        ) -> Result<u32, u32> {
            self.compare_exchange(current, new, success, failure)
        }
        pub fn swap(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(6, 0, value)
        }
        pub fn fetch_or(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(7, 0, value)
        }
        pub fn fetch_and(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(8, 0, value)
        }
        pub fn fetch_xor(&self, value: u32, _order: Ordering) -> u32 {
            self.apply(9, 0, value)
        }
    }
    pub fn fence(order: Ordering) {
        assert!(order != Ordering::Relaxed);
        // SAFETY: the HAL executes a compiler and hardware memory barrier.
        unsafe { fm1_mcu_fence() }
    }
}
#[cfg(feature = "pi32v2")]
pub use mcu::{fence, AtomicU32};

#[cfg(all(test, feature = "pi32v2"))]
mod abi_tests {
    use super::*;
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // Hosted test implementation of the same opaque HAL ABI. This verifies the
    // Rust facade and existing concurrent queues; target instructions are a
    // separate physical gate.
    #[no_mangle]
    unsafe extern "C" fn fm1_mcu_atomic(op: u32, word: *mut u32, expected: u32, value: u32) -> u32 {
        let _guard = LOCK.lock().unwrap();
        // SAFETY: the facade supplies a live aligned cell, and LOCK excludes
        // every other test-backend access to that storage.
        let old = unsafe { word.read() };
        let next = match op {
            0 => old,
            1 | 6 => value,
            2 => old.wrapping_add(value),
            3 => old.wrapping_sub(value),
            4 => old.max(value),
            5 => {
                if old == expected {
                    value
                } else {
                    old
                }
            }
            7 => old | value,
            8 => old & value,
            9 => old ^ value,
            _ => unreachable!(),
        };
        // SAFETY: same exclusive mutex-protected cell as above.
        unsafe { word.write(next) };
        old
    }
    #[no_mangle]
    extern "C" fn fm1_mcu_fence() {
        core::sync::atomic::fence(Ordering::SeqCst);
    }
    #[test]
    fn opaque_hal_abi_preserves_wrapping_rmw_and_compare_results() {
        let word = AtomicU32::new(u32::MAX);
        assert_eq!(word.fetch_add(1, Ordering::Relaxed), u32::MAX);
        assert_eq!(word.load(Ordering::Acquire), 0);
        assert_eq!(word.fetch_sub(1, Ordering::SeqCst), 0);
        assert_eq!(
            word.compare_exchange(0, 12, Ordering::AcqRel, Ordering::Acquire),
            Err(u32::MAX)
        );
        assert_eq!(
            word.compare_exchange_weak(u32::MAX, 12, Ordering::Release, Ordering::Relaxed),
            Ok(u32::MAX)
        );
        assert_eq!(word.fetch_max(8, Ordering::Relaxed), 12);
        assert_eq!(word.fetch_or(3, Ordering::Relaxed), 12);
        assert_eq!(word.fetch_and(7, Ordering::Relaxed), 15);
        assert_eq!(word.fetch_xor(2, Ordering::Relaxed), 7);
        assert_eq!(word.swap(4, Ordering::Relaxed), 5);
        word.store(19, Ordering::Release);
        fence(Ordering::SeqCst);
        assert_eq!(word.load(Ordering::SeqCst), 19);
    }
}
