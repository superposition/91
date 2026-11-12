// SPDX-License-Identifier: GPL-3.0-only
//! Scoped scheduling ABI. One render owner supplies initialized values and
//! disjoint writable outputs/RNG; no borrow or caller pointer is retained.
use fm1_engine::sequencer::Drift;
use fm1_engine::sequencer::{self, Hit, Lane, Rng};
use fm1_rt::atomic::{AtomicU32, Ordering};
static LANES: AtomicU32 = AtomicU32::new(0);
static MUTATIONS: AtomicU32 = AtomicU32::new(0);
static WAITS: AtomicU32 = AtomicU32::new(0);
static BOUNDARIES: AtomicU32 = AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn fm1_rust_seq_density(mask: u32, amount: i32) -> u32 {
    sequencer::density(mask as u16, amount) as u32
}
#[no_mangle]
pub extern "C" fn fm1_rust_seq_swing(step: u32, swing: f32, length: f32) -> i32 {
    sequencer::swing_delay(step, swing, length)
}
/// # Safety
/// `seed` is initialized, aligned, writable and uniquely held by the render
/// caller. It does not alias Rust state or any concurrently accessed bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_loose(
    wait: i32,
    amount: u32,
    ms: f32,
    seed: *mut u32,
) -> i32 {
    let mut rng = Rng(unsafe { seed.read() });
    let result = sequencer::loose(wait, amount, ms, &mut rng);
    unsafe { seed.write(rng.0) };
    result
}
/// # Safety
/// The request has valid firmware bounds (lane<6, genre<19, fill<8, section
/// length>0). Request/seed/output are initialized, aligned, live, and pairwise
/// disjoint. One serialized render caller owns mutations for this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_lane(request: *const Lane, seed: *mut u32, output: *mut Hit) {
    let request = unsafe { request.read() };
    let mut rng = Rng(unsafe { seed.read() });
    let result = sequencer::lane(&request, &mut rng);
    unsafe {
        seed.write(rng.0);
        output.write(result)
    };
    LANES.fetch_add(1, Ordering::Relaxed);
}
pub fn stat(index: u32) -> u32 {
    match index {
        18 => LANES.load(Ordering::Relaxed),
        19 => MUTATIONS.load(Ordering::Relaxed),
        20 => WAITS.load(Ordering::Relaxed),
        21 => BOUNDARIES.load(Ordering::Relaxed),
        _ => 0,
    }
}

/// # Safety
/// `waits` provides sixteen initialized readable i32 values for this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_earliest(waits: *const i32) -> i32 {
    sequencer::earliest(&unsafe { waits.cast::<[i32; 16]>().read_unaligned() })
}
/// # Safety
/// `wait` is initialized, aligned and exclusively writable by this caller.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_wait(wait: *mut i32, frames: i32) -> u32 {
    WAITS.fetch_add(1, Ordering::Relaxed);
    u32::from(sequencer::advance_wait(unsafe { &mut *wait }, frames))
}
/// # Safety
/// The disjoint output pointers provide one writable aligned scalar each.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_chain(
    n: u32,
    gap: i32,
    mul: f32,
    velocity: f32,
    wait: *mut i32,
    level: *mut f32,
) -> u32 {
    match sequencer::ratchet_chain(n, gap, mul, velocity) {
        Some((left, w, v)) => {
            unsafe {
                wait.write(w);
                level.write(v)
            };
            left
        }
        None => u32::MAX,
    }
}
/// # Safety
/// `patterns` provides 72 initialized readable bytes, not concurrently mutated.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_song_pool(patterns: *const u16) -> u32 {
    sequencer::song_pool(&unsafe { patterns.cast::<[[u16; 6]; 6]>().read_unaligned() })
}
#[no_mangle]
pub extern "C" fn fm1_rust_seq_drift_length(song: u32, genre: u32, every: u32) -> u32 {
    sequencer::drift_length(song != 0, genre as usize, every)
}
/// # Safety
/// The six-u16 pattern and u32 seed are initialized, live, disjoint and uniquely
/// writable by the serialized render owner. No foreign pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_mutate(pattern: *mut u16, morph: u32, seed: *mut u32) {
    let mut p = unsafe { pattern.cast::<[u16; 6]>().read_unaligned() };
    let mut rng = Rng(unsafe { seed.read() });
    sequencer::mutate(&mut p, morph != 0, &mut rng);
    unsafe {
        pattern.cast::<[u16; 6]>().write_unaligned(p);
        seed.write(rng.0)
    };
    MUTATIONS.fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// State and seed are initialized, aligned, disjoint, and exclusively writable.
/// Pool is 2..6; current/next pattern IDs have the existing firmware bounds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_drift(
    state: *mut Drift,
    length: u32,
    change: u32,
    mutation: u32,
    pool: u32,
    current: i32,
    seed: *mut u32,
) {
    let mut rng = Rng(unsafe { seed.read() });
    sequencer::drift(
        unsafe { &mut *state },
        length,
        change,
        mutation,
        pool,
        current,
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
}
#[no_mangle]
pub extern "C" fn fm1_rust_seq_boundary(
    position: u32,
    bars: u32,
    upload: u32,
    hold: u32,
    pending: u32,
) -> u32 {
    BOUNDARIES.fetch_add(1, Ordering::Relaxed);
    sequencer::phrase_boundary(position, bars, upload != 0, hold != 0, pending != 0)
}
#[no_mangle]
pub extern "C" fn fm1_rust_seq_time(
    player: u32,
    genre: u32,
    section: i32,
    groove: u32,
    build: i32,
) -> u32 {
    sequencer::time_mode(player, genre, section, groove, build)
}
#[no_mangle]
pub extern "C" fn fm1_rust_seq_fill(allowed: u32, bars: u32, genre: u32) -> u32 {
    sequencer::fill_kind(allowed, bars, genre)
}
/// # Safety
/// `index` is initialized, aligned and exclusively writable; chain is 1..9.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_pattern_chain(chain: u32, index: *mut u32) -> u32 {
    let (pattern, next) = sequencer::chain(chain, unsafe { index.read() });
    unsafe { index.write(next) };
    pattern
}
/// # Safety
/// The render owner uniquely owns the initialized/aligned u32 seed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_slice(request: u32, bars: u32, seed: *mut u32) -> u32 {
    let mut rng = Rng(unsafe { seed.read() });
    let value = sequencer::slice(request, bars, &mut rng);
    unsafe { seed.write(rng.0) };
    value
}
/// # Safety
/// The render owner uniquely owns the initialized/aligned u32 seed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_seq_section(
    command: u32,
    current: i32,
    degree: i32,
    nsteps: i32,
    evolve: f32,
    seed: *mut u32,
) -> u32 {
    let mut rng = Rng(unsafe { seed.read() });
    let value = sequencer::section(command, current, degree, nsteps, evolve, &mut rng);
    unsafe { seed.write(rng.0) };
    value
}
