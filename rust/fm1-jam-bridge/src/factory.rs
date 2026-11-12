// SPDX-License-Identifier: GPL-3.0-only
//! Copy-only musical payload seam plus immutable metadata/string C ABI views.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::factory;
use fm1_types::{AcidLine, AcidStep, GenreId, PieceId, PluckLine, PluckStep, StepId};
static ACCESSES: AtomicU32 = AtomicU32::new(0);
static CODECS: AtomicU32 = AtomicU32::new(0);
static REJECTIONS: AtomicU32 = AtomicU32::new(0);
static BYTES: AtomicU32 = AtomicU32::new(0);
pub fn stat(index: u32) -> u32 {
    match index {
        32 => ACCESSES.load(Ordering::Relaxed),
        33 => CODECS.load(Ordering::Relaxed),
        34 => REJECTIONS.load(Ordering::Relaxed),
        35 => BYTES.load(Ordering::Relaxed),
        _ => 0,
    }
}
fn reject() -> i32 {
    REJECTIONS.fetch_add(1, Ordering::Relaxed);
    0
}
fn indices(g: i32, i: i32) -> Option<(GenreId, PieceId)> {
    Some((
        GenreId::new(g as usize).ok()?,
        PieceId::new(i as usize).ok()?,
    ))
}
#[repr(C)]
pub struct ParameterAbi {
    pub name: *const u8,
    pub min: i16,
    pub max: i16,
    pub default: i16,
}
// SAFETY: generated pointers refer only to immutable, static, NUL-terminated
// byte strings. C receives a const view; neither side mutates these objects.
unsafe impl Sync for ParameterAbi {}
#[repr(transparent)]
struct NamePointers<const N: usize>([*const u8; N]);
// SAFETY: same immutable static-string contract, including pointer-array storage.
unsafe impl<const N: usize> Sync for NamePointers<N> {}
include!("factory_data.rs");
#[no_mangle]
pub extern "C" fn fm1_rust_factory_parameters() -> *const ParameterAbi {
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    PARAMETERS.as_ptr()
}
#[no_mangle]
pub extern "C" fn fm1_rust_factory_names(category: u32) -> *const *const u8 {
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    match name_pointers(category as usize) {
        Some(p) => p,
        None => {
            reject();
            core::ptr::null()
        }
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_factory_name(category: u32, index: u32) -> *const u8 {
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    match factory::name(category as usize, index as usize) {
        Some(p) => p.as_ptr(),
        None => {
            reject();
            core::ptr::null()
        }
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_factory_modes() -> *const u16 {
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    factory::mode_pools().as_ptr()
}
#[no_mangle]
pub extern "C" fn fm1_rust_factory_donor(genre: i32) -> i32 {
    match GenreId::new(genre as usize) {
        Ok(g) => {
            ACCESSES.fetch_add(1, Ordering::Relaxed);
            factory::donor(g) as i32
        }
        Err(_) => {
            reject();
            -1
        }
    }
}
/// # Safety
/// `output` holds176 writable aligned i16s and `pool` one disjoint writable u16.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_factory_piece(
    g: i32,
    i: i32,
    output: *mut i16,
    capacity: u32,
    pool: *mut u16,
) -> i32 {
    let Some((g, i)) = indices(g, i) else {
        return reject();
    };
    if output.is_null() || pool.is_null() || capacity < 176 {
        return reject();
    }
    let preset = factory::preset(g, i);
    // SAFETY: exact bounded copy into caller-provided, disjoint storage.
    unsafe {
        core::ptr::copy_nonoverlapping(preset.parameters.as_ptr(), output, 176);
        pool.write_unaligned(preset.pool);
    }
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    BYTES.fetch_add(354, Ordering::Relaxed);
    1
}
/// # Safety
/// Output has `capacity` writable bytes, aligned for u16 for patterns/boot.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_factory_bank(
    g: i32,
    i: i32,
    kind: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    let Some((g, i)) = indices(g, i) else {
        return reject();
    };
    if output.is_null() {
        return reject();
    }
    match kind {
        0 | 2 => {
            if capacity < 72 {
                return reject();
            }
            let bank = if kind == 2 {
                *factory::boot_patterns()
            } else {
                factory::patterns(g, i)
            };
            let mut words = [0u16; 36];
            for (index, mask) in bank.iter().flatten().enumerate() {
                words[index] = mask.bits();
            }
            // SAFETY: caller provides72 writable bytes; temporary word array is disjoint.
            unsafe {
                core::ptr::copy_nonoverlapping(words.as_ptr().cast::<u8>(), output, 72);
            }
            BYTES.fetch_add(72, Ordering::Relaxed);
        }
        1 => {
            if capacity < 96 {
                return reject();
            }
            let bank = factory::modifiers(g, i);
            let mut bytes = [0u8; 96];
            for (index, value) in bank.iter().flatten().enumerate() {
                bytes[index] = value.bits();
            }
            // SAFETY: bounded caller storage is disjoint from the temporary modifier array.
            unsafe {
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), output, 96);
            }
            BYTES.fetch_add(96, Ordering::Relaxed);
        }
        _ => return reject(),
    }
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    1
}
/// # Safety
/// Output points to9 disjoint writable bytes: progression then chord kinds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_factory_arc(piece: i32, output: *mut u8, capacity: u32) -> i32 {
    let Ok(piece) = PieceId::new(piece as usize) else {
        return reject();
    };
    if output.is_null() || capacity < 9 {
        return reject();
    }
    let (p, k) = factory::arc(piece);
    // SAFETY: two exact nonoverlapping segments within the checked9-byte output.
    unsafe {
        core::ptr::copy_nonoverlapping(p.as_ptr(), output, 6);
        core::ptr::copy_nonoverlapping(k.as_ptr(), output.add(6), 3);
    }
    ACCESSES.fetch_add(1, Ordering::Relaxed);
    BYTES.fetch_add(9, Ordering::Relaxed);
    1
}
/// # Safety
/// Input/output cover their supplied lengths and are disjoint. Word input and
/// output may be unaligned; they are copied, never retained. kind0=acid,1=pluck.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_factory_line(
    kind: u32,
    decode: u32,
    input: *const u8,
    length: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if input.is_null() || output.is_null() || kind > 1 || decode > 1 {
        return reject();
    }
    let packed = if kind == 0 { 18 } else { 12 };
    if (decode == 0 && (length != 32 || capacity < packed))
        || (decode == 1 && (length != packed || capacity < 32))
    {
        return reject();
    }
    if kind == 0 {
        let mut line = AcidLine::EMPTY;
        if decode == 0 {
            for slot in 0..16 {
                // SAFETY: input is exactly16 initialized words; unaligned reads are explicit.
                let word = unsafe { input.cast::<u16>().add(slot).read_unaligned() };
                line.set(StepId::new(slot).unwrap(), AcidStep::from_bits(word));
            }
            // SAFETY: output has18 disjoint writable bytes.
            unsafe {
                core::ptr::copy_nonoverlapping(line.bytes().as_ptr(), output, 18);
            }
        } else {
            let mut bytes = [0; 18];
            // SAFETY: length is18, copied into a distinct initialized fixed array.
            unsafe {
                core::ptr::copy_nonoverlapping(input, bytes.as_mut_ptr(), 18);
            }
            line = AcidLine::from_bytes(bytes);
            for slot in 0..16 {
                // SAFETY: output has32 bytes; each of16 unaligned writes stays inside it.
                unsafe {
                    output
                        .cast::<u16>()
                        .add(slot)
                        .write_unaligned(line.step(StepId::new(slot).unwrap()).bits());
                }
            }
        }
    } else {
        let mut line = PluckLine::EMPTY;
        if decode == 0 {
            for slot in 0..16 {
                // SAFETY: input is exactly16 readable words.
                let word = unsafe { input.cast::<u16>().add(slot).read_unaligned() };
                line.set(StepId::new(slot).unwrap(), PluckStep::from_bits(word as u8));
            }
            // SAFETY: output has12 disjoint writable bytes.
            unsafe {
                core::ptr::copy_nonoverlapping(line.bytes().as_ptr(), output, 12);
            }
        } else {
            let mut bytes = [0; 12];
            // SAFETY: input is exactly12 bytes and bytes is a distinct initialized array.
            unsafe {
                core::ptr::copy_nonoverlapping(input, bytes.as_mut_ptr(), 12);
            }
            line = PluckLine::from_bytes(bytes);
            for slot in 0..16 {
                // SAFETY: output has32 bytes, including this unaligned word.
                unsafe {
                    output
                        .cast::<u16>()
                        .add(slot)
                        .write_unaligned(line.step(StepId::new(slot).unwrap()).bits() as u16);
                }
            }
        }
    }
    let copied = if decode == 0 { packed } else { 32 };
    CODECS.fetch_add(1, Ordering::Relaxed);
    BYTES.fetch_add(copied, Ordering::Relaxed);
    copied as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_ffi_inputs_never_mutate_outputs_and_unaligned_codecs_are_bounded() {
        let before = stat(34);
        let mut parameters = [i16::MIN; 176];
        let mut pool = 0xa5a5;
        // SAFETY: all non-null pointers below identify live initialized arrays;
        // rejected capacities/indices/null arguments are checked before any access.
        unsafe {
            assert_eq!(
                fm1_rust_factory_piece(-1, 0, parameters.as_mut_ptr(), 176, &mut pool),
                0
            );
            assert_eq!(
                fm1_rust_factory_piece(0, 10, parameters.as_mut_ptr(), 176, &mut pool),
                0
            );
            assert_eq!(
                fm1_rust_factory_piece(0, 0, parameters.as_mut_ptr(), 175, &mut pool),
                0
            );
            assert_eq!(
                fm1_rust_factory_piece(0, 0, core::ptr::null_mut(), 176, &mut pool),
                0
            );
        }
        assert_eq!(parameters, [i16::MIN; 176]);
        assert_eq!(pool, 0xa5a5);
        let mut bank = [0xa5; 96];
        let mut arc = [0xa5; 9];
        // SAFETY: exact array bounds; invalid enum/range/capacity values must reject.
        unsafe {
            assert_eq!(fm1_rust_factory_bank(19, 0, 0, bank.as_mut_ptr(), 96), 0);
            assert_eq!(fm1_rust_factory_bank(0, 0, 3, bank.as_mut_ptr(), 96), 0);
            assert_eq!(fm1_rust_factory_bank(0, 0, 0, bank.as_mut_ptr(), 71), 0);
            assert_eq!(fm1_rust_factory_arc(10, arc.as_mut_ptr(), 9), 0);
            assert_eq!(fm1_rust_factory_arc(0, arc.as_mut_ptr(), 8), 0);
        }
        assert_eq!(bank, [0xa5; 96]);
        assert_eq!(arc, [0xa5; 9]);
        assert!(fm1_rust_factory_names(27).is_null());
        assert!(fm1_rust_factory_name(0, 12).is_null());
        assert_eq!(fm1_rust_factory_donor(-1), -1);
        let words = [511u16; 16];
        let mut bytes = [0xa5; 35];
        // SAFETY: source is32 readable bytes; each destination range is within bytes.
        unsafe {
            assert_eq!(
                fm1_rust_factory_line(0, 0, words.as_ptr().cast(), 31, bytes.as_mut_ptr(), 18),
                0
            );
            assert_eq!(
                fm1_rust_factory_line(0, 0, words.as_ptr().cast(), 32, bytes.as_mut_ptr(), 17),
                0
            );
            assert_eq!(
                fm1_rust_factory_line(2, 0, words.as_ptr().cast(), 32, bytes.as_mut_ptr(), 18),
                0
            );
        }
        assert_eq!(bytes, [0xa5; 35]);
        assert!(stat(34) >= before + 15);
        let mut packed = [0; 18];
        // SAFETY: disjoint exact arrays; decoder intentionally receives an unaligned
        // byte destination, for which the bridge uses individual unaligned writes.
        unsafe {
            assert_eq!(
                fm1_rust_factory_line(0, 0, words.as_ptr().cast(), 32, packed.as_mut_ptr(), 18),
                18
            );
            assert_eq!(
                fm1_rust_factory_line(0, 1, packed.as_ptr(), 18, bytes.as_mut_ptr().add(1), 32),
                32
            );
        }
        assert_eq!(packed, [255; 18]);
        assert_eq!(bytes[0], 0xa5);
        assert_eq!(&bytes[33..], &[0xa5; 2]);
        for slot in 0..16 {
            assert_eq!(
                u16::from_ne_bytes([bytes[1 + 2 * slot], bytes[2 + 2 * slot]]),
                511
            );
        }
    }
}
