// SPDX-License-Identifier: GPL-3.0-only
//! Scoped voice-owner borrows; no pointer/callback/state copy survives a block.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::voices::{self, Drum, DrumsConfig, Mix, Pluck, PluckConfig};
static COUNTERS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
const _: [(); 148] = [(); core::mem::size_of::<voices::pad::State>()];
const _: [(); 24] = [(); core::mem::size_of::<voices::pad::Runtime>()];
const _: [(); 164] = [(); core::mem::size_of::<voices::pad::Config>()];
const _: [(); 28] = [(); core::mem::size_of::<voices::pad::Publication>()];
const _: [(); 40] = [(); core::mem::size_of::<voices::pad::BusConfig>()];
/// # Safety
/// Four initialized pad states, runtime/config, seed,1025 sine values, two
/// writable n-f32 outputs and one publication are aligned and pairwise disjoint.
/// The pad worker exclusively holds all writable regions until return. Callback
/// may only touch the disjoint C MIDI queue: no voice/runtime/config/seed/table/
/// output access or render reentry. The MIDI queue is never borrowed by Rust.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fm1_rust_voice_pad(
    s: *mut voices::pad::State,
    rt: *mut voices::pad::Runtime,
    c: *const voices::pad::Config,
    seed: *mut u32,
    table: *const f32,
    l: *mut f32,
    r: *mut f32,
    result: *mut voices::pad::Publication,
    n: u32,
    midi: unsafe extern "C" fn(u32, u32, u32),
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    let mut sink = |s: u8, a: u8, b: u8| unsafe { midi(s as u32, a as u32, b as u32) };
    let publication = voices::pad::process(
        unsafe { &mut *s.cast::<[voices::pad::State; 4]>() },
        unsafe { &mut *rt },
        unsafe { &*c },
        &mut rng,
        unsafe { &*table.cast::<[f32; 1025]>() },
        unsafe { core::slice::from_raw_parts_mut(l, n as usize) },
        unsafe { core::slice::from_raw_parts_mut(r, n as usize) },
        &mut sink,
    );
    unsafe {
        seed.write(rng.0);
        result.write(publication)
    };
    COUNTERS[2].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// Initialized/aligned state/config/seed are disjoint and writable state/seed
/// are exclusively held. Callback only touches the disjoint C MIDI queue and
/// cannot read/write these regions or reenter render. No pointer/callback is
/// retained; scale is the validated per-voice breath factor.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_breathe(
    s: *mut voices::pad::State,
    c: *const voices::pad::Breath,
    scale: f32,
    seed: *mut u32,
    midi: unsafe extern "C" fn(u32, u32, u32),
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    let mut sink = |s: u8, a: u8, b: u8| unsafe { midi(s as u32, a as u32, b as u32) };
    voices::pad::breathe(
        unsafe { &mut *s },
        unsafe { &*c },
        scale,
        &mut rng,
        &mut sink,
    );
    unsafe { seed.write(rng.0) };
}
/// # Safety
/// Four filter values, config, four n-f32 buffer regions and the512-f32 dry
/// ring are aligned/disjoint. Slots0/1 are read-only; slots2/3, filter and ring
/// are exclusively writable for this call. No references or callbacks escape.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_bus(
    state: *mut f32,
    c: *const voices::pad::BusConfig,
    p: *const *mut f32,
    dry: *mut f32,
    n: u32,
) {
    let p = unsafe { &*p.cast::<[*mut f32; 4]>() };
    let n = n as usize;
    voices::pad::bus(
        unsafe { &mut *state.cast::<[f32; 4]>() },
        unsafe { &*c },
        voices::pad::Bus {
            left: unsafe { core::slice::from_raw_parts(p[0], n) },
            right: unsafe { core::slice::from_raw_parts(p[1], n) },
            out_left: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            out_right: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
            dry: unsafe { &mut *dry.cast::<[f32; 512]>() },
        },
    );
    COUNTERS[6].fetch_add(1, Ordering::Relaxed);
}
const _: [(); 96] = [(); core::mem::size_of::<voices::drone::State>()];
const _: [(); 12] = [(); core::mem::size_of::<voices::drone::Line>()];
const _: [(); 124] = [(); core::mem::size_of::<voices::drone::Config>()];
const _: [(); 108] = [(); core::mem::size_of::<voices::acid::State>()];
const _: [(); 88] = [(); core::mem::size_of::<voices::acid::Config>()];

/// # Safety
/// Initialized state/config/line/control and1025-value sine table are aligned
/// and pairwise disjoint from the n-f32 output. Voice owner exclusively holds
/// all writable regions until return; no callback or pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drone(
    s: *mut voices::drone::State,
    c: *const voices::drone::Config,
    line: *mut voices::drone::Line,
    control: *mut u32,
    table: *const f32,
    out: *mut f32,
    n: u32,
) -> f32 {
    let level = voices::drone::process(
        unsafe { &mut *s },
        unsafe { &*c },
        unsafe { &mut *line },
        unsafe { &mut *control },
        unsafe { &*table.cast::<[f32; 1025]>() },
        unsafe { core::slice::from_raw_parts_mut(out, n as usize) },
    );
    COUNTERS[3].fetch_add(1, Ordering::Relaxed);
    level
}
/// # Safety
/// The module's disjoint scoped voice/control/output contract applies. Table
/// provides1025 initialized f32 values and state has firmware-valid bounds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_acid(
    s: *mut voices::acid::State,
    c: *const voices::acid::Config,
    control: *mut u32,
    table: *const f32,
    out: *mut f32,
    n: u32,
) -> f32 {
    let level = voices::acid::process(
        unsafe { &mut *s },
        unsafe { &*c },
        unsafe { &mut *control },
        unsafe { &*table.cast::<[f32; 1025]>() },
        unsafe { core::slice::from_raw_parts_mut(out, n as usize) },
    );
    COUNTERS[4].fetch_add(1, Ordering::Relaxed);
    level
}
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.wrapping_sub(40) as usize)
        .map(|v| v.load(Ordering::Relaxed))
        .unwrap_or(0)
}
const _: [(); 104] = [(); core::mem::size_of::<Drum>()];
const _: [(); 108] = [(); core::mem::size_of::<DrumsConfig>()];
const _: [(); 28] = [(); core::mem::size_of::<Pluck>()];
const _: [(); 44] = [(); core::mem::size_of::<PluckConfig>()];
/// # Safety
/// State/config/table/seed and four initialized n-f32 mix arrays are aligned and
/// pairwise disjoint. The audio owner exclusively holds state/seed/mix while
/// table/config stay read-only. Six states and1025 sine values are provided.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drums(
    s: *mut Drum,
    c: *const DrumsConfig,
    t: *const f32,
    seed: *mut u32,
    p: *const *mut f32,
    n: u32,
) {
    let p = unsafe { &*p.cast::<[*mut f32; 4]>() };
    let n = n as usize;
    voices::drums(
        unsafe { &mut *s.cast::<[Drum; 6]>() },
        unsafe { &*c },
        unsafe { &*t.cast::<[f32; 1025]>() },
        unsafe { &mut *seed },
        Mix {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        },
    );
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// Two initialized voice states and1025 sine values are provided. The module's
/// disjoint block-borrow contract applies, without the shared drum seed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pluck(
    s: *mut Pluck,
    c: *const PluckConfig,
    t: *const f32,
    p: *const *mut f32,
    n: u32,
) -> f32 {
    let p = unsafe { &*p.cast::<[*mut f32; 4]>() };
    let n = n as usize;
    let level = voices::pluck(
        unsafe { &mut *s.cast::<[Pluck; 2]>() },
        unsafe { &*c },
        unsafe { &*t.cast::<[f32; 1025]>() },
        Mix {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        },
    );
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    level
}
