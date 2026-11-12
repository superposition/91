// SPDX-License-Identifier: GPL-3.0-only
//! Transitional ABI: serialized emulator source capture, Rust-owned copies and
//! staging arbitration. Physical IRQ exclusion is deliberately not enabled.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_rt::handoff::{SnapshotCell, StagingLease, StagingOwner};

pub const SNAPSHOT_BYTES: usize = 220;
static SNAPSHOT: SnapshotCell<[u8; SNAPSHOT_BYTES]> = SnapshotCell::new([0; SNAPSHOT_BYTES]);
static STAGING: StagingLease = StagingLease::new();
const _: [(); 224] = [(); core::mem::size_of::<SnapshotCell<[u8; SNAPSHOT_BYTES]>>()];
const _: [(); 4] = [(); core::mem::size_of::<StagingLease>()];
static PUBLISHED: AtomicU32 = AtomicU32::new(0);
static COPIED: AtomicU32 = AtomicU32::new(0);
static CLAIMED: AtomicU32 = AtomicU32::new(0);
static ACTIVATIONS: AtomicU32 = AtomicU32::new(0);
static REJECTED: AtomicU32 = AtomicU32::new(0);

pub unsafe fn reset_exclusive() {
    // SAFETY: the boot caller excludes all C staging readers and writers.
    unsafe { STAGING.reset_exclusive() };
    let _ = SNAPSHOT.try_publish([0; SNAPSHOT_BYTES]);
    for counter in [&PUBLISHED, &COPIED, &CLAIMED, &ACTIVATIONS, &REJECTED] {
        counter.store(0, Ordering::Relaxed);
    }
}

/// # Safety
/// `input`/`output` identify 220 initialized readable/writable bytes. They may
/// alias each other, but not Rust static storage. The caller owns or excludes
/// mutations of the source during this bounded copy. The ABI has one serialized
/// source caller, so the read belongs to this publication. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_snapshot_copy(
    input: *const u8,
    output: *mut u8,
    len: u32,
) -> i32 {
    if input.is_null() || output.is_null() || len as usize != SNAPSHOT_BYTES {
        return 0;
    }
    // SAFETY: the source contract covers the complete initialized byte array.
    let value = unsafe { input.cast::<[u8; SNAPSHOT_BYTES]>().read_unaligned() };
    if SNAPSHOT.try_publish(value).is_err() {
        return 0;
    }
    PUBLISHED.fetch_add(1, Ordering::Relaxed);
    let Some(value) = SNAPSHOT.try_read() else {
        return 0;
    };
    // SAFETY: input has been copied; in-place foreign input/output is allowed.
    unsafe { output.cast::<[u8; SNAPSHOT_BYTES]>().write_unaligned(value) };
    COPIED.fetch_add(1, Ordering::Relaxed);
    1
}

#[no_mangle]
pub extern "C" fn fm1_rust_staging_owner() -> u32 {
    STAGING.owner() as u32
}

fn owner(n: u32) -> Option<StagingOwner> {
    Some(match n {
        0 => StagingOwner::Idle,
        1 => StagingOwner::Receive,
        2 => StagingOwner::Activation,
        3 => StagingOwner::Readback,
        _ => return None,
    })
}

/// # Safety
/// The sole C foreground owner serializes calls and all accesses to its single
/// staging buffer. Receive/readback writes require that lease; activation bytes
/// are immutable until en_jam_busy becomes false. Release never invalidates a
/// Rust guard. The external buffer itself remains C-owned during this port.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_staging_transition(from: u32, to: u32) -> i32 {
    let (Some(from), Some(to)) = (owner(from), owner(to)) else {
        return 0;
    };
    // SAFETY: the foreign foreground owner satisfies this buffer's protocol.
    if !unsafe { STAGING.transition_external(from, to) } {
        REJECTED.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    if from == StagingOwner::Idle {
        CLAIMED.fetch_add(1, Ordering::Relaxed);
    }
    if to == StagingOwner::Activation {
        ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
    }
    1
}

pub fn stat(index: u32) -> u32 {
    match index {
        13 => PUBLISHED.load(Ordering::Relaxed),
        14 => COPIED.load(Ordering::Relaxed),
        15 => CLAIMED.load(Ordering::Relaxed),
        16 => ACTIVATIONS.load(Ordering::Relaxed),
        17 => REJECTED.load(Ordering::Relaxed),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_snapshot_is_transactional_and_allows_in_place_copy() {
        let mut value = core::array::from_fn::<_, SNAPSHOT_BYTES, _>(|i| i as u8);
        let mut output = [0xa5; SNAPSHOT_BYTES];
        // SAFETY: initialized arrays, exact bounds and no concurrent mutation.
        unsafe {
            assert_eq!(
                fm1_rust_snapshot_copy(
                    core::ptr::null(),
                    output.as_mut_ptr(),
                    SNAPSHOT_BYTES as u32
                ),
                0
            );
            assert_eq!(
                fm1_rust_snapshot_copy(value.as_ptr(), output.as_mut_ptr(), 219),
                0
            );
            assert_eq!(output, [0xa5; SNAPSHOT_BYTES]);
            assert_eq!(
                fm1_rust_snapshot_copy(value.as_ptr(), output.as_mut_ptr(), SNAPSHOT_BYTES as u32),
                1
            );
            assert_eq!(value, output);
            let p = value.as_mut_ptr();
            assert_eq!(fm1_rust_snapshot_copy(p, p, SNAPSHOT_BYTES as u32), 1);
            assert_eq!(value, output);
        }
    }

    #[test]
    fn foreign_staging_rejects_illegal_and_stale_transitions() {
        // SAFETY: this test is the sole external owner; no buffer or safe guard
        // is borrowed, and each release happens after simulated consumption.
        unsafe {
            assert_eq!(fm1_rust_staging_owner(), 0);
            assert_eq!(fm1_rust_staging_transition(1, 0), 0);
            assert_eq!(fm1_rust_staging_transition(0, 2), 0);
            assert_eq!(fm1_rust_staging_transition(0, 1), 1);
            assert_eq!(fm1_rust_staging_transition(0, 3), 0);
            assert_eq!(fm1_rust_staging_transition(1, 3), 0);
            assert_eq!(fm1_rust_staging_transition(1, 2), 1);
            assert_eq!(fm1_rust_staging_owner(), 2);
            assert_eq!(fm1_rust_staging_transition(2, 0), 1);
            assert_eq!(fm1_rust_staging_transition(0, 3), 1);
            assert_eq!(fm1_rust_staging_transition(3, 1), 0);
            assert_eq!(fm1_rust_staging_transition(3, 0), 1);
            assert_eq!(fm1_rust_staging_owner(), 0);
        }
    }
}
