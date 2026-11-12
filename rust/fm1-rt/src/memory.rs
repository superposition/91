// SPDX-License-Identifier: GPL-3.0-only
//! Compiler-runtime byte movement, independent of instrument state.
use core::mem::MaybeUninit;

/// Move object-representation bytes, including uninitialized padding.
/// Volatile operations prevent the compiler from replacing this runtime
/// implementation with a recursive call to the memmove symbol it supplies.
///
/// # Safety
/// Source and destination cover `length` bytes of live readable/writable
/// storage. The ranges may overlap. Exclude every concurrent conflicting
/// access. Zero length performs no access and permits null pointers.
pub unsafe fn move_bytes(destination: *mut u8, source: *const u8, length: usize) {
    let destination = destination.cast::<MaybeUninit<u8>>();
    let source = source.cast::<MaybeUninit<u8>>();
    if destination as usize <= source as usize {
        for i in 0..length {
            // SAFETY: caller-promised ranges; forward copy protects overlap
            // when destination precedes source. MaybeUninit permits padding.
            unsafe {
                destination
                    .add(i)
                    .write_volatile(source.add(i).read_volatile())
            };
        }
    } else {
        for i in (0..length).rev() {
            // SAFETY: reverse copy protects overlapping later destinations.
            unsafe {
                destination
                    .add(i)
                    .write_volatile(source.add(i).read_volatile())
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_small_overlap_matches_standard_memmove() {
        for length in 0..=32 {
            for source in 0..=32 - length {
                for destination in 0..=32 - length {
                    let mut actual: [u8; 32] = core::array::from_fn(|i| i as u8);
                    let mut expected = actual;
                    expected.copy_within(source..source + length, destination);
                    // SAFETY: both complete ranges are within this live array.
                    unsafe {
                        move_bytes(
                            actual.as_mut_ptr().add(destination),
                            actual.as_ptr().add(source),
                            length,
                        )
                    };
                    assert_eq!(actual, expected);
                }
            }
        }
        // SAFETY: zero length performs no access.
        unsafe { move_bytes(core::ptr::null_mut(), core::ptr::null(), 0) };
    }
    #[test]
    fn object_padding_can_move_without_becoming_a_typed_value() {
        let mut bytes = [MaybeUninit::<u8>::uninit(); 8];
        for (i, byte) in bytes[..4].iter_mut().enumerate() {
            *byte = MaybeUninit::new(i as u8);
        }
        // SAFETY: all storage is live, including the uninitialized padding;
        // MaybeUninit keeps its validity independent of the byte contents.
        unsafe {
            move_bytes(
                bytes.as_mut_ptr().cast::<u8>().add(2),
                bytes.as_ptr().cast::<u8>(),
                6,
            )
        };
        for (i, byte) in bytes[2..6].iter().enumerate() {
            // SAFETY: these four bytes were initialized before the move.
            assert_eq!(unsafe { byte.assume_init() }, i as u8);
        }
    }
}
