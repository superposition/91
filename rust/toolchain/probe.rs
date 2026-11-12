// SPDX-License-Identifier: GPL-3.0-only
//! R02 proof module; this is not the production engine entry point.
#![no_std]
extern crate fm1_engine;
extern crate fm1_formats;
extern crate fm1_types;
use core::cell::UnsafeCell;
use fm1_engine::planner::{bass_event, plan_into, Phrase, Seed, Settings};
use fm1_formats::jam::JamPack;

struct ProbeCell(UnsafeCell<u32>);
// SAFETY: the private cell is reachable only through the explicitly unsafe,
// exclusive-access probe entry below. It is not an application synchronization primitive.
unsafe impl Sync for ProbeCell {}
static PROBE_CELL: ProbeCell = ProbeCell(UnsafeCell::new(17));

/// Exercise interior-mutable storage when testing read-only static emission.
///
/// # Safety
/// The caller must exclude every other access to this probe cell.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mutability_probe() -> u32 {
    // SAFETY: the caller supplies exclusive access to the private probe cell.
    unsafe {
        let p = PROBE_CELL.0.get();
        *p += 1;
        *p
    }
}

/// Read a 32-bit peripheral value without eliding or combining the load.
///
/// # Safety
/// `address` must be aligned and valid for a 32-bit volatile read, with the
/// peripheral's access and synchronization requirements satisfied by the caller.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_read_volatile(address: *const u32) -> u32 {
    // SAFETY: the caller supplies a valid peripheral or memory address.
    unsafe { core::ptr::read_volatile(address) }
}

#[no_mangle]
pub extern "C" fn fm1_rust_probe(seed: u32) -> u32 {
    let bytes = include_bytes!("../fm1-engine/tests/packs/velvet-changes.jmp");
    let pack = match JamPack::parse(bytes) {
        Ok(pack) => pack,
        Err(_) => return 0,
    };
    let settings = Settings {
        key: 6,
        minor: 1,
        bars: 8,
        bass: 3,
        ..Settings::default()
    };
    let mut rng = Seed::new(seed);
    let mut first = Phrase::EMPTY;
    let mut next = Phrase::EMPTY;
    if plan_into(pack, &settings, 0, None, &mut first, &mut rng).is_err() {
        return 0;
    }
    if plan_into(pack, &settings, 1, Some(&first), &mut next, &mut rng).is_err() {
        return 0;
    }
    let mut h = 2_166_136_261u32;
    for beat in first.active_beats() {
        for note in beat.notes {
            h = (h ^ note as u32).wrapping_mul(16_777_619);
        }
        h = (h ^ beat.node as u32).wrapping_mul(16_777_619);
        h = (h ^ beat.passing as u32).wrapping_mul(16_777_619);
    }
    for bar in 0..8 {
        for step in 0..16 {
            let event = match bass_event(pack, &settings, &first, &next, bar, step) {
                Ok(event) => event,
                Err(_) => return 0,
            };
            for value in event.note.to_le_bytes() {
                h = (h ^ value as u32).wrapping_mul(16_777_619);
            }
            h = (h ^ event.velocity as u32).wrapping_mul(16_777_619);
            h = (h ^ event.gate as u32).wrapping_mul(16_777_619);
        }
    }
    h ^ rng.state()
}

#[no_mangle]
pub extern "C" fn fm1_rust_f32(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}

#[no_mangle]
pub extern "C" fn fm1_rust_callback(callback: extern "C" fn(u32) -> u32, value: u32) -> u32 {
    callback(value)
}

#[no_mangle]
#[link_section = ".pool.rust_probe"]
pub static FM1_RUST_LAYOUT_PROBE: [u8; 16] = [0; 16];

#[no_mangle]
pub extern "C" fn fm1_rust_layout_address() -> *const u8 {
    &FM1_RUST_LAYOUT_PROBE as *const [u8; 16] as *const u8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_matches_the_frozen_c_instrument() {
        assert_eq!(fm1_rust_probe(4), 3_766_679_758);
    }
    #[test]
    fn f32_contract_and_callback_abi_are_explicit() {
        let a = f32::from_bits(0x3f80_0001);
        let b = f32::from_bits(0x3f7f_fffe);
        assert_eq!(fm1_rust_f32(a, b, -1.0).to_bits(), 0);
        extern "C" fn callback(v: u32) -> u32 {
            v.rotate_left(7) ^ 0xa5a5_a5a5
        }
        assert_eq!(fm1_rust_callback(callback, 0x1234_5678), 0xbf8e_99ac);
    }

    #[test]
    fn interior_mutable_static_is_writable() {
        // SAFETY: no other test or API accesses this private proof cell.
        assert_eq!(unsafe { fm1_rust_mutability_probe() }, 18);
        let value = 0x91ab_cdef;
        // SAFETY: this aligned local remains live for the duration of the read.
        assert_eq!(unsafe { fm1_rust_read_volatile(&value) }, value);
    }
}
