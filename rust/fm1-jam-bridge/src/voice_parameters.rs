// SPDX-License-Identifier: GPL-3.0-only
//! Serialized voice parameter/init calls; no reference or callback is retained.
use crate::voice_data::*;
use fm1_engine::voices::{self, parameters, setup};
#[no_mangle]
pub extern "C" fn fm1_rust_voice_parameter(id: u32, value: i32) -> f32 {
    parameters::value(id as usize, value).unwrap_or(0.0)
}
#[no_mangle]
pub extern "C" fn fm1_rust_voice_acid_decay(value: i32) -> f32 {
    parameters::acid_decay(value)
}
/// # Safety
/// Slot and output are aligned/disjoint and exclusively writable for a bounded
/// two-voice allocation and20-byte note conversion. Slot is the valid0/1 state.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pluck_note(
    slot: *mut i32,
    decay: i32,
    fm: u32,
    velocity: f32,
    output: *mut parameters::PluckNote,
) {
    unsafe { output.write(parameters::pluck_note(&mut *slot, decay, fm != 0, velocity)) };
}
/// # Safety
/// params contains176 initialized/aligned i16 values, live and immutable for
/// this call. kind selects stabs0 or oscillator1; no reference escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_identity(
    params: *const i16,
    jam: u32,
    kind: u32,
) -> u32 {
    let (stabs, osc) = voices::config::pad_identity(
        unsafe { &*params.cast::<voices::config::Params>() },
        jam,
        &FM1_VOICE_PAD_SOUND,
    );
    if kind == 0 {
        stabs
    } else {
        osc
    }
}
/// # Safety
/// output points to four aligned/exclusively writable floats, disjoint from
/// the immutable recipe table. No reference or callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_hat_setup(metal: f32, output: *mut f32) {
    unsafe {
        output
            .cast::<[f32; 4]>()
            .write(parameters::hat_increments(metal, &FM1_VOICE_HAT_INC))
    };
}
/// # Safety
/// Four initialized pad states and ten motion states are aligned, disjoint,
/// exclusively writable in the serialized init handler. This preserves the
/// original partial-reset semantics. No callback/reference/pointer escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_initialize(
    pads: *mut voices::pad::State,
    motion: *mut setup::Motion,
) {
    setup::initialize(
        unsafe { &mut *pads.cast::<[voices::pad::State; 4]>() },
        unsafe { &mut *motion.cast::<[setup::Motion; 10]>() },
        &FM1_VOICE_PAD_PAN,
    );
}
/// # Safety
/// Four initialized pads, breath config and seed are aligned and pairwise
/// disjoint; pads/seed exclusively writable. Callback only writes the separate
/// C MIDI queue and cannot access these regions or reenter any voice/render
/// path. It retains no references; no borrow survives this serialized call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_restart(
    pads: *mut voices::pad::State,
    c: *const voices::pad::Breath,
    seed: *mut u32,
    quiet: u32,
    midi: unsafe extern "C" fn(u32, u32, u32),
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    let mut sink = |s: u8, a: u8, b: u8| unsafe { midi(s as u32, a as u32, b as u32) };
    setup::restart(
        unsafe { &mut *pads.cast::<[voices::pad::State; 4]>() },
        unsafe { &*c },
        &FM1_VOICE_START_T,
        &FM1_VOICE_BREATH,
        quiet != 0,
        &mut rng,
        &mut sink,
    );
    unsafe { seed.write(rng.0) };
}
/// # Safety
/// Initialized/aligned state is exclusively writable until return. No callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_start_bass(bass: *mut voices::drone::State) {
    setup::start_bass(unsafe { &mut *bass });
}
/// # Safety
/// Initialized/aligned state is exclusively writable until return. No callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_acid_release(acid: *mut voices::acid::State) {
    setup::acid_release(unsafe { &mut *acid });
}
/// # Safety
/// Initialized/aligned state is exclusively writable until return. No callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_acid_manual(acid: *mut voices::acid::State, on: u32) {
    setup::acid_manual(unsafe { &mut *acid }, on != 0);
}
/// # Safety
/// Initialized/aligned line is exclusively writable until return. No callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_line_reset(line: *mut voices::drone::Line) {
    setup::reset_line(unsafe { &mut *line });
}
/// # Safety
/// Initialized/aligned LFOs are disjoint and exclusively writable until return.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_lfo_reset(a: *mut setup::Lfo, b: *mut setup::Lfo) {
    setup::reset_lfos(unsafe { &mut *a }, unsafe { &mut *b });
}
const _: [(); 20] = [(); core::mem::size_of::<parameters::PluckNote>()];
/// # Safety
///176 initialized/aligned parameters and aligned exclusively writable output
/// are disjoint/live. No borrow/reference/pointer/callback survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_manual_pluck(
    params: *const i16,
    key: i32,
    velocity: f32,
    out: *mut parameters::ManualPluck,
) {
    let p = unsafe { &*params.cast::<voices::config::Params>() };
    unsafe { out.write(parameters::manual_pluck(p, key, velocity)) };
}
/// # Safety
/// Initialized/aligned links are exclusively writable. No callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_links_hit(
    s: *mut setup::Links,
    lane: u32,
    velocity: f32,
    on: u32,
) {
    setup::links_hit(unsafe { &mut *s }, lane, velocity, on != 0);
}
/// # Safety
/// Initialized/aligned links are exclusively writable. No callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_links_step(
    s: *mut setup::Links,
    n: u32,
    kick: i32,
    snare: i32,
    on: u32,
) {
    setup::links_step(unsafe { &mut *s }, n, kick, snare, on != 0);
}
/// # Safety
///176 initialized/aligned parameters, initialized/aligned LFOs and aligned
/// exclusively writable output are live/disjoint. No references escape.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_lfo_routes(
    params: *const i16,
    pad: *const setup::Lfo,
    drone: *const setup::Lfo,
    out: *mut setup::LfoRoutes,
) {
    unsafe {
        out.write(setup::lfo_routes(
            &*params.cast::<voices::config::Params>(),
            &*pad,
            &*drone,
        ))
    };
}
const _: [(); 8] = [(); core::mem::size_of::<setup::Links>()];
const _: [(); 32] = [(); core::mem::size_of::<setup::LfoRoutes>()];
