// SPDX-License-Identifier: GPL-3.0-only
//! Scoped voice-owner borrows; no pointer/callback/state copy survives a block.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::voices::{self, Drum, DrumsConfig, Mix, Pluck, PluckConfig};
static COUNTERS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
use voices::setup;
/// # Safety
/// Four initialized/aligned pad states are exclusively writable. No reentry.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_handoff(s: *mut voices::pad::State, osc: u32) {
    setup::pad_handoff(
        unsafe { &mut *s.cast::<[voices::pad::State; 4]>() },
        osc as u8,
    );
}
/// # Safety
/// Four initialized pad states and count<=4 pitch classes are aligned, live,
/// disjoint. Only pads are written, with exclusive access. Nothing escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_leave(
    s: *mut voices::pad::State,
    pcs: *const i32,
    count: u32,
) {
    setup::pad_leave(
        unsafe { &mut *s.cast::<[voices::pad::State; 4]>() },
        unsafe { core::slice::from_raw_parts(pcs, count as usize) },
    );
}
/// # Safety
/// Four initialized pad states and output are aligned/disjoint/exclusively
/// writable. MIDI dispatch occurs after return; no callback crosses a borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_stab(
    s: *mut voices::pad::State,
    osc: u32,
    level: f32,
    release: i32,
    out: *mut setup::Events,
) -> f32 {
    let (decay, events) = setup::stab(
        unsafe { &mut *s.cast::<[voices::pad::State; 4]>() },
        osc as u8,
        level,
        release,
    );
    unsafe { out.write(events) };
    decay
}
/// # Safety
/// Initialized/aligned voice and output are disjoint and exclusively writable.
/// MIDI dispatch occurs after return; no callback crosses a borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_bass_to(
    s: *mut voices::drone::State,
    pc: i32,
    octave: i32,
    line: u32,
    playing: u32,
    out: *mut setup::Events,
) {
    unsafe { out.write(setup::bass_to(&mut *s, pc, octave, line != 0, playing != 0)) };
}
/// # Safety
/// Initialized/aligned voice, line and output are disjoint and exclusively
/// writable. No callback/reference/pointer survives return.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fm1_rust_voice_bass_note(
    s: *mut voices::drone::State,
    line: *mut voices::drone::Line,
    note: i32,
    flags: i32,
    glide: i32,
    length: i32,
    step: f32,
    out: *mut setup::Events,
) -> u32 {
    let (slide, events) = setup::bass_note(
        unsafe { &mut *s },
        unsafe { &mut *line },
        note,
        flags,
        glide,
        length,
        step,
    );
    unsafe { out.write(events) };
    slide as u32
}
#[repr(C)]
pub struct KeyResult {
    pub held: u8,
    pub glow: u8,
    pub padding: [u8; 2],
    pub events: setup::Events,
}
const _: [(); 44] = [(); core::mem::size_of::<setup::KeyConfig>()];
const _: [(); 104] = [(); core::mem::size_of::<KeyResult>()];
const _: [(); 20] = [(); core::mem::size_of::<setup::DrumConfig>()];
const _: [(); 36] = [(); core::mem::size_of::<setup::AcidConfig>()];
/// # Safety
/// Four pads, drone, acid,24held bytes,3counts, sequence, config and output are
/// initialized/aligned and pairwise disjoint. All mutable regions are owned by
/// the serialized audio command handler. Config voice0..2/counts0..8 and C0
/// allocator order limits were validated by the caller. MIDI/FM dispatch occurs
/// after return. No pointer/reference/callback survives the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fm1_rust_voice_key(
    pads: *mut voices::pad::State,
    bass: *mut voices::drone::State,
    acid: *mut voices::acid::State,
    held: *mut u8,
    counts: *mut u8,
    seq: *mut u32,
    c: *const setup::KeyConfig,
    out: *mut KeyResult,
) {
    let (held, glow, events) = setup::key(
        unsafe { &mut *pads.cast::<[voices::pad::State; 4]>() },
        unsafe { &mut *bass },
        unsafe { &mut *acid },
        unsafe { &mut *held.cast::<[[u8; 8]; 3]>() },
        unsafe { &mut *counts.cast::<[u8; 3]>() },
        unsafe { &mut *seq },
        unsafe { &*c },
    );
    unsafe {
        out.write(KeyResult {
            held,
            glow: glow as u8,
            padding: [0; 2],
            events,
        })
    };
}
/// # Safety
/// Initialized/aligned pads, drone, acid, six drums and3counts are disjoint and
/// exclusively writable by the serialized audio command handler. No callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_quiet(
    pads: *mut voices::pad::State,
    bass: *mut voices::drone::State,
    acid: *mut voices::acid::State,
    drums: *mut Drum,
    counts: *mut u8,
) {
    setup::quiet(
        unsafe { &mut *pads.cast::<[voices::pad::State; 4]>() },
        unsafe { &mut *bass },
        unsafe { &mut *acid },
        unsafe { &mut *drums.cast::<[Drum; 6]>() },
        unsafe { &mut *counts.cast::<[u8; 3]>() },
    );
}
const _: [(); 100] = [(); core::mem::size_of::<setup::Events>()];
#[no_mangle]
pub extern "C" fn fm1_rust_voice_time(kind: u32, value: i32) -> f32 {
    setup::time(kind, value)
}
/// # Safety
/// Ten initialized motion states and seed are aligned, disjoint, exclusively
/// writable for this serialized call. No references or callback escape.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_motion(s: *mut setup::Motion, n: u32, seed: *mut u32) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    setup::motion(
        unsafe { &mut *s.cast::<[setup::Motion; 10]>() },
        n,
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
    COUNTERS[5].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// State and seed are initialized, aligned, disjoint and exclusively writable.
/// step is the engine's finite positive sixteenth duration; no reentry occurs.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_lfo(
    s: *mut setup::Lfo,
    rate: i32,
    shape: i32,
    n: f32,
    step: f32,
    seed: *mut u32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    setup::lfo(unsafe { &mut *s }, rate, shape, n, step, &mut rng);
    unsafe { seed.write(rng.0) };
}
/// # Safety
/// The initialized/aligned drone is exclusively writable until return.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_envelope(
    s: *mut voices::drone::State,
    n: u32,
    attack: i32,
    decay: i32,
) {
    setup::drone_envelope(unsafe { &mut *s }, n, attack, decay);
}
/// # Safety
/// Six initialized drum states, config, pointer array and four immutable kit
/// rows are aligned/live and disjoint. The drum bank is exclusively writable;
/// lane is validated0..5. No pointer/reference/callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drum_setup(
    s: *mut Drum,
    lane: u32,
    c: *const setup::DrumConfig,
    kits: *const *const voices::data::Kit,
) {
    let p = unsafe { &*kits.cast::<[*const voices::data::Kit; 4]>() };
    setup::drum(
        unsafe { &mut *s.cast::<[Drum; 6]>() },
        lane as usize,
        unsafe { &*c },
        p.map(|v| unsafe { &*v }),
    );
}
/// # Safety
/// The initialized/aligned pluck state is exclusively writable until return.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pluck_setup(
    s: *mut Pluck,
    note: i32,
    vel: f32,
    decay: f32,
    tuning: f32,
) {
    setup::pluck(unsafe { &mut *s }, note, vel, decay, tuning);
}
/// # Safety
/// Initialized state/config and writable result are aligned/disjoint. State
/// and result are exclusively writable. Bounded MIDI is dispatched by C only
/// after return, so no callback crosses the mutable voice borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_acid_setup(
    s: *mut voices::acid::State,
    c: *const setup::AcidConfig,
    result: *mut setup::Events,
) {
    unsafe { result.write(setup::acid(&mut *s, &*c)) };
}
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
