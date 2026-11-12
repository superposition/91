// SPDX-License-Identifier: GPL-3.0-only
//! Transitional C ABI for exercising the portable Rust planner in the instrument.
//! No input borrow or engine pointer survives a call. Remove this seam after the
//! application and render owner have migrated to Rust.
#![cfg_attr(not(feature = "std"), no_std)]

use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::planner::{bass_event, plan_into, Phrase, Seed, Settings};
use fm1_formats::jam::JamPack;
use fm1_formats::session::JamSession;
mod commands;
mod factory;
mod formats;
mod handoff;
mod sequencer;
mod transport;

static VALIDATIONS: AtomicU32 = AtomicU32::new(0);
static PLANS: AtomicU32 = AtomicU32::new(0);
static REJECTIONS: AtomicU32 = AtomicU32::new(0);
static FALLBACKS: AtomicU32 = AtomicU32::new(0);
static BASS_EVENTS: AtomicU32 = AtomicU32::new(0);
static SESSIONS: AtomicU32 = AtomicU32::new(0);

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    #[cfg(not(target_arch = "wasm32"))]
    loop {
        core::hint::spin_loop();
    }
}

/// Match legacy structural pack validation without duplicating the C wire ABI.
///
/// # Safety
/// `bytes` must identify `len` readable bytes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_jam_validate(bytes: *const u8, len: u32) -> i32 {
    VALIDATIONS.fetch_add(1, Ordering::Relaxed);
    if bytes.is_null() || len as usize > fm1_types::STORE_MAX {
        return 0;
    }
    // SAFETY: the caller guarantees the readable byte range; length is bounded.
    let input = unsafe { core::slice::from_raw_parts(bytes, len as usize) };
    i32::from(JamPack::parse_structural(input).is_ok())
}

/// Validate a saved session using the shared codec.
///
/// # Safety
/// `bytes` must identify `len` readable bytes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_session_validate(bytes: *const u8, len: u32) -> i32 {
    SESSIONS.fetch_add(1, Ordering::Relaxed);
    if bytes.is_null() || len as usize != fm1_formats::session::SESSION_LEN {
        return 0;
    }
    // SAFETY: caller guarantees the readable input; length is exactly twenty.
    let input = unsafe { core::slice::from_raw_parts(bytes, len as usize) };
    i32::from(JamSession::decode(input).is_ok())
}

/// Encode a newly saved session with the legacy source marker and zero tail.
///
/// # Safety
/// Settings must be readable and initialized; output must provide at least
/// `capacity` writable bytes disjoint from settings. No concurrent mutation.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_session_encode(
    settings: *const Settings,
    tempo: u32,
    source: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    SESSIONS.fetch_add(1, Ordering::Relaxed);
    if settings.is_null()
        || output.is_null()
        || capacity < fm1_formats::session::SESSION_LEN as u32
        || tempo > u16::MAX as u32
        || source > u8::MAX as u32
    {
        REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    let session = JamSession {
        // SAFETY: settings is initialized and live for this call.
        settings: unsafe { settings.read_unaligned() },
        tempo: tempo as u16,
        trailer: [source as u8, b'S', 0, 0],
    };
    let mut encoded = [0; fm1_formats::session::SESSION_LEN];
    match session.encode_into(&mut encoded) {
        Ok(length) => {
            // SAFETY: output is writable, disjoint, and has sufficient capacity.
            unsafe { core::ptr::copy_nonoverlapping(encoded.as_ptr(), output, length) };
            length as i32
        }
        Err(_) => {
            REJECTIONS.fetch_add(1, Ordering::Relaxed);
            0
        }
    }
}

// C initializes the header and active beats only. Copying the whole C value
// would read an uninitialized tail when the previous planner was still C.
// SAFETY: source must have an initialized header and all of its active beats.
unsafe fn read_phrase(source: *const Phrase) -> Phrase {
    let mut phrase = Phrase::EMPTY;
    unsafe {
        core::ptr::copy_nonoverlapping(
            source.cast::<u8>(),
            (&mut phrase as *mut Phrase).cast::<u8>(),
            4,
        );
    }
    if (8..=16).contains(&phrase.bars) {
        for index in 0..phrase.bars as usize * 4 {
            // SAFETY: only initialized active beats are read, without alignment
            // assumptions; the destination is a distinct, fully initialized copy.
            phrase.beats[index] =
                unsafe { core::ptr::addr_of!((*source).beats[index]).read_unaligned() };
        }
    }
    phrase
}

/// Fill one C-owned phrase using Rust's bounded planner.
///
/// # Safety
/// Pack/settings/seed must be readable and output must hold a writable Phrase.
/// Seed must also be writable and disjoint from pack/settings/output.
/// Settings/header and the previous active beats must be initialized. Output
/// must not overlap the pack/settings/seed. No concurrent access is permitted.
/// Previous/output aliasing is supported by copying only initialized input.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_jam_plan(
    bytes: *const u8,
    len: u32,
    settings: *const Settings,
    section: u32,
    previous: *const Phrase,
    output: *mut Phrase,
    seed: *mut u32,
) -> i32 {
    PLANS.fetch_add(1, Ordering::Relaxed);
    if bytes.is_null()
        || settings.is_null()
        || output.is_null()
        || seed.is_null()
        || len as usize > fm1_types::STORE_MAX
    {
        REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    // SAFETY: caller guarantees initialized settings and a readable bounded pack.
    let input = unsafe { core::slice::from_raw_parts(bytes, len as usize) };
    let settings = unsafe { settings.read_unaligned() };
    let pack = match JamPack::parse_structural(input) {
        Ok(pack) => pack,
        Err(_) => {
            REJECTIONS.fetch_add(1, Ordering::Relaxed);
            return 0;
        }
    };
    let prior = if previous.is_null() {
        None
    } else {
        // SAFETY: the caller guarantees initialized header and active beats.
        Some(unsafe { read_phrase(previous) })
    };
    let mut result = Phrase::EMPTY;
    // SAFETY: caller supplies an initialized u32, possibly unaligned.
    let mut random = Seed::new(unsafe { seed.read_unaligned() });
    if plan_into(
        pack,
        &settings,
        section,
        prior.as_ref(),
        &mut result,
        &mut random,
    )
    .is_err()
    {
        REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    // SAFETY: caller provides writable output and seed without overlapping inputs.
    unsafe {
        output.write_unaligned(result);
        seed.write_unaligned(random.state());
    }
    1
}

/// The initialized four-byte result crosses the ABI; it is not a wire format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct BassOutput {
    pub note: i16,
    pub velocity: u8,
    pub gate: u8,
}

const _: [(); 4] = [(); core::mem::size_of::<BassOutput>()];

/// Resolve one bass step, including approaches across the phrase boundary.
///
/// # Safety
/// Pack/settings/phrases must be readable and initialized as for the planner.
/// Output must hold a writable BassOutput disjoint from all inputs. Phrase
/// pointers may alias each other (Hold). No concurrent mutation is permitted.
#[no_mangle]
#[allow(clippy::too_many_arguments)] // Explicit pointer/length pairs at the C ABI.
pub unsafe extern "C" fn fm1_rust_jam_bass(
    bytes: *const u8,
    len: u32,
    settings: *const Settings,
    phrase: *const Phrase,
    next: *const Phrase,
    bar: u32,
    step: u32,
    output: *mut BassOutput,
) -> i32 {
    BASS_EVENTS.fetch_add(1, Ordering::Relaxed);
    if bytes.is_null()
        || settings.is_null()
        || phrase.is_null()
        || next.is_null()
        || output.is_null()
        || len as usize > fm1_types::STORE_MAX
    {
        REJECTIONS.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    // SAFETY: the caller supplies readable initialized input; no borrow escapes.
    let input = unsafe { core::slice::from_raw_parts(bytes, len as usize) };
    let settings = unsafe { settings.read_unaligned() };
    let phrase = unsafe { read_phrase(phrase) };
    let next = unsafe { read_phrase(next) };
    let event = JamPack::parse_structural(input).and_then(|pack| {
        bass_event(pack, &settings, &phrase, &next, bar as usize, step as usize)
            .map_err(|_| fm1_formats::jam::PackError::Bass)
    });
    match event {
        Ok(event) => {
            // SAFETY: output is writable, disjoint from the input allocations.
            unsafe {
                output.write_unaligned(BassOutput {
                    note: event.note,
                    velocity: event.velocity,
                    gate: event.gate,
                });
            }
            1
        }
        Err(_) => {
            REJECTIONS.fetch_add(1, Ordering::Relaxed);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn fm1_rust_jam_fallback() {
    FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn fm1_rust_jam_stat(index: u32) -> u32 {
    match index {
        0 => VALIDATIONS.load(Ordering::Relaxed),
        1 => PLANS.load(Ordering::Relaxed),
        2 => REJECTIONS.load(Ordering::Relaxed),
        3 => FALLBACKS.load(Ordering::Relaxed),
        4 => BASS_EVENTS.load(Ordering::Relaxed),
        5 => SESSIONS.load(Ordering::Relaxed),
        6..=9 => commands::stat(index),
        10..=12 => transport::stat(index),
        13..=17 => handoff::stat(index),
        18..=23 => sequencer::stat(index),
        32..=35 => factory::stat(index),
        36..=39 => formats::stat(index),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_owned_in_place_phrase_matches_direct_rust_planning() {
        let bytes = include_bytes!("../../fm1-engine/tests/packs/velvet-changes.jmp");
        let settings = Settings::default();
        let mut seed = 4;
        let mut result = Phrase::EMPTY;
        // SAFETY: all pointed-to values are live, initialized, valid and distinct.
        assert_eq!(
            unsafe {
                fm1_rust_jam_plan(
                    bytes.as_ptr(),
                    bytes.len() as u32,
                    &settings,
                    0,
                    core::ptr::null(),
                    &mut result,
                    &mut seed,
                )
            },
            1
        );
        let previous = result;
        let mut expected = Phrase::EMPTY;
        let mut expected_seed = Seed::new(seed);
        plan_into(
            JamPack::parse(bytes).unwrap(),
            &settings,
            1,
            Some(&previous),
            &mut expected,
            &mut expected_seed,
        )
        .unwrap();
        let pointer = &mut result as *mut Phrase;
        // SAFETY: the boundary explicitly supports previous/output aliasing.
        assert_eq!(
            unsafe {
                fm1_rust_jam_plan(
                    bytes.as_ptr(),
                    bytes.len() as u32,
                    &settings,
                    1,
                    pointer,
                    pointer,
                    &mut seed,
                )
            },
            1
        );
        assert_eq!(result, expected);
        assert_eq!(seed, expected_seed.state());
    }

    #[test]
    fn bass_boundary_supports_hold_and_preserves_outputs_on_rejection() {
        for bytes in [
            include_bytes!("../../fm1-engine/tests/packs/harris-citypop.jmp").as_slice(),
            include_bytes!("../../fm1-engine/tests/packs/velvet-changes.jmp").as_slice(),
        ] {
            let pack = JamPack::parse(bytes).unwrap();
            let settings = Settings {
                bars: 8,
                bass: 3,
                ..Settings::default()
            };
            let mut phrase = Phrase::EMPTY;
            plan_into(pack, &settings, 0, None, &mut phrase, &mut Seed::new(91)).unwrap();
            for bar in 0..settings.bars as usize {
                for step in 0..16 {
                    let expected =
                        bass_event(pack, &settings, &phrase, &phrase, bar, step).unwrap();
                    let mut output = BassOutput::default();
                    // SAFETY: inputs are live, initialized and immutable. Hold
                    // intentionally supplies the same phrase for both inputs.
                    assert_eq!(
                        unsafe {
                            fm1_rust_jam_bass(
                                bytes.as_ptr(),
                                bytes.len() as u32,
                                &settings,
                                &phrase,
                                &phrase,
                                bar as u32,
                                step as u32,
                                &mut output,
                            )
                        },
                        1
                    );
                    assert_eq!(output.note, expected.note);
                    assert_eq!(output.velocity, expected.velocity);
                    assert_eq!(output.gate, expected.gate);
                }
            }
            let mut output = BassOutput {
                note: 123,
                velocity: 45,
                gate: 67,
            };
            let unchanged = output;
            // SAFETY: initialized live inputs; invalid indices must be rejected.
            assert_eq!(
                unsafe {
                    fm1_rust_jam_bass(
                        bytes.as_ptr(),
                        bytes.len() as u32,
                        &settings,
                        &phrase,
                        &phrase,
                        8,
                        16,
                        &mut output,
                    )
                },
                0
            );
            assert_eq!(output, unchanged);
        }
    }
}
