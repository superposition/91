// SPDX-License-Identifier: GPL-3.0-only
//! Bounded copies and an exclusive staging lease. Neither operation waits.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

/// A small copied view. A contended read/publication returns immediately.
/// This protects its own storage; the publisher must separately own its source.
pub struct SnapshotCell<T: Copy> {
    held: AtomicU32,
    value: UnsafeCell<T>,
}

// SAFETY: all shared access to value is protected by the acquire/release gate.
// Copying T between threads requires Send; no reference into storage escapes.
unsafe impl<T: Copy + Send> Sync for SnapshotCell<T> {}

struct Gate<'a>(&'a AtomicU32);
impl Drop for Gate<'_> {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

impl<T: Copy> SnapshotCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            held: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    fn enter(&self) -> Option<Gate<'_>> {
        self.held
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .ok()?;
        Some(Gate(&self.held))
    }

    pub fn try_publish(&self, value: T) -> Result<(), T> {
        let Some(_gate) = self.enter() else {
            return Err(value);
        };
        // SAFETY: this gate uniquely excludes readers and other publishers.
        unsafe { self.value.get().write(value) };
        Ok(())
    }

    pub fn try_read(&self) -> Option<T> {
        let _gate = self.enter()?;
        // SAFETY: the initialized value cannot be changed while the gate lives.
        Some(unsafe { self.value.get().read() })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StagingOwner {
    Idle = 0,
    Receive = 1,
    Activation = 2,
    Readback = 3,
}

/// Arbitration for one external staging buffer, with no second payload copy.
/// Safe users retain a non-clonable guard until the entire transfer ends.
pub struct StagingLease {
    owner: AtomicU32,
}

impl Default for StagingLease {
    fn default() -> Self {
        Self::new()
    }
}

impl StagingLease {
    pub const fn new() -> Self {
        Self {
            owner: AtomicU32::new(0),
        }
    }
    pub fn owner(&self) -> StagingOwner {
        match self.owner.load(Ordering::Acquire) {
            1 => StagingOwner::Receive,
            2 => StagingOwner::Activation,
            3 => StagingOwner::Readback,
            _ => StagingOwner::Idle,
        }
    }

    pub fn try_claim(&self, owner: StagingOwner) -> Option<StagingGuard<'_>> {
        if !matches!(owner, StagingOwner::Receive | StagingOwner::Readback) {
            return None;
        }
        self.owner
            .compare_exchange(0, owner as u32, Ordering::Acquire, Ordering::Relaxed)
            .ok()?;
        Some(StagingGuard { lease: self, owner })
    }

    /// Change a lease held by the foreign foreground owner, not a Rust guard.
    ///
    /// # Safety
    /// All accesses to the associated C buffer follow this lease. Only its
    /// serialized owner calls this method. Release activation only after render
    /// has finished reading it; no live safe Rust guard may be invalidated.
    pub unsafe fn transition_external(&self, from: StagingOwner, to: StagingOwner) -> bool {
        let allowed = matches!(
            (from, to),
            (
                StagingOwner::Idle,
                StagingOwner::Receive | StagingOwner::Readback
            ) | (
                StagingOwner::Receive,
                StagingOwner::Activation | StagingOwner::Idle
            ) | (
                StagingOwner::Activation | StagingOwner::Readback,
                StagingOwner::Idle
            )
        );
        allowed
            && self
                .owner
                .compare_exchange(from as u32, to as u32, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
    }

    /// # Safety
    /// Boot-only: exclude all buffer users and discard all foreign leases first.
    pub unsafe fn reset_exclusive(&self) {
        self.owner.store(0, Ordering::Relaxed);
    }
}

pub struct StagingGuard<'a> {
    lease: &'a StagingLease,
    owner: StagingOwner,
}
impl StagingGuard<'_> {
    /// Switch receiving to activation. The caller keeps the associated external
    /// bytes immutable until this guard is released; it does not own the bytes.
    pub fn await_activation(mut self) -> Self {
        if self.owner == StagingOwner::Receive {
            self.lease
                .owner
                .store(StagingOwner::Activation as u32, Ordering::Release);
            self.owner = StagingOwner::Activation;
        }
        self
    }
    pub fn owner(&self) -> StagingOwner {
        self.owner
    }
}
impl Drop for StagingGuard<'_> {
    fn drop(&mut self) {
        self.lease.owner.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    fn patterned(n: u32) -> [u32; 55] {
        core::array::from_fn(|i| n.wrapping_mul(0x9e37_79b9).rotate_left(i as u32) ^ i as u32)
    }

    #[test]
    fn snapshot_contention_never_waits_or_changes_an_output() {
        let cell = SnapshotCell::new(patterned(1));
        let gate = cell.enter().unwrap();
        assert_eq!(cell.try_publish(patterned(2)), Err(patterned(2)));
        assert!(cell.try_read().is_none());
        drop(gate);
        assert_eq!(cell.try_read(), Some(patterned(1)));
        cell.try_publish(patterned(2)).unwrap();
        assert_eq!(cell.try_read(), Some(patterned(2)));
    }

    #[test]
    fn concurrent_220_byte_snapshots_are_coherent() {
        // First word encodes the sequence; every other word verifies it.
        fn frame(n: u32) -> [u32; 55] {
            let mut v = patterned(n);
            v[0] = n;
            v
        }
        let cell = Arc::new(SnapshotCell::new(frame(0)));
        let rendezvous = Arc::new(Barrier::new(2));
        let c = cell.clone();
        let b = rendezvous.clone();
        let writer = thread::spawn(move || {
            b.wait();
            let deadline = Instant::now() + Duration::from_secs(15);
            for n in 1..=100_000 {
                while c.try_publish(frame(n)).is_err() {
                    assert!(Instant::now() < deadline, "snapshot writer stalled");
                    thread::yield_now();
                }
            }
        });
        rendezvous.wait();
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut reads = 0;
        loop {
            if let Some(v) = cell.try_read() {
                assert_eq!(v, frame(v[0]));
                reads += 1;
                if v[0] == 100_000 {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "snapshot reader stalled");
            thread::yield_now();
        }
        writer.join().unwrap();
        assert!(reads > 0);
    }

    #[test]
    fn activation_and_readback_hold_one_lease_to_the_end() {
        let lease = StagingLease::new();
        assert!(lease.try_claim(StagingOwner::Activation).is_none());
        let receiving = lease.try_claim(StagingOwner::Receive).unwrap();
        assert!(lease.try_claim(StagingOwner::Readback).is_none());
        let active = receiving.await_activation();
        assert_eq!(active.owner(), StagingOwner::Activation);
        assert!(lease.try_claim(StagingOwner::Receive).is_none());
        drop(active);
        let readback = lease.try_claim(StagingOwner::Readback).unwrap();
        for _chunk in 0..60 {
            assert!(lease.try_claim(StagingOwner::Receive).is_none());
        }
        drop(readback);
        assert_eq!(lease.owner(), StagingOwner::Idle);
        assert!(lease.try_claim(StagingOwner::Receive).is_some());
    }

    #[test]
    fn concurrent_claims_never_overlap_activation_or_readback() {
        let lease = Arc::new(StagingLease::new());
        let inside = Arc::new(AtomicU32::new(0));
        let workers: [_; 2] = core::array::from_fn(|role| {
            let lease = lease.clone();
            let inside = inside.clone();
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                let owner = if role == 0 {
                    StagingOwner::Receive
                } else {
                    StagingOwner::Readback
                };
                for _ in 0..50_000 {
                    let guard = loop {
                        if let Some(guard) = lease.try_claim(owner) {
                            break guard;
                        }
                        assert!(Instant::now() < deadline, "staging claim stalled");
                        thread::yield_now();
                    };
                    assert_eq!(inside.fetch_add(1, Ordering::Relaxed), 0);
                    let guard = if role == 0 {
                        guard.await_activation()
                    } else {
                        guard
                    };
                    assert_eq!(lease.owner(), guard.owner());
                    assert!(lease.try_claim(StagingOwner::Receive).is_none());
                    assert_eq!(inside.fetch_sub(1, Ordering::Relaxed), 1);
                    drop(guard);
                }
            })
        });
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(lease.owner(), StagingOwner::Idle);
    }
}
