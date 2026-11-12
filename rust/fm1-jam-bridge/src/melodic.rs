// SPDX-License-Identifier: GPL-3.0-only
//! Scoped, serialized scheduling calls. Every pointer is live/aligned and
//! initialized unless documented as output; distinct arguments do not alias.
//! C owns its source for the call and no pointer or callback borrow survives.
#![allow(clippy::too_many_arguments)]
use fm1_engine::melodic::{
    self, Acid, AcidConfig, AcidNote, Bass, BassConfig, BassNote, Harmony, Pluck, PluckConfig,
    PluckNote,
};
use fm1_engine::scheduling_tables::{self, Section};
use fm1_engine::sequencer::Rng;
use fm1_rt::atomic::{AtomicU32, Ordering};
#[no_mangle]
pub static FM1_RUST_SECTIONS: [Section; 19] = scheduling_tables::SECTIONS;
#[no_mangle]
pub static FM1_RUST_FILL_OK: [u8; 19] = scheduling_tables::FILL_OK;
const _: [(); 9] = [(); core::mem::size_of::<Section>()];
const _: [(); 1] = [(); core::mem::align_of::<Section>()];

/// # Safety
/// Config is initialized/aligned and uniquely writable. Genre is already mapped
/// to0..18 and config.mode is the validated0..16 firmware value.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_recipe(config: *mut BassConfig, genre: u32) {
    let config = unsafe { &mut *config };
    let (root, figure) = scheduling_tables::bass_recipe(genre as usize, config.mode);
    config.root_mask = root as u32;
    config.riff_mask = figure.mask as u32;
    config.intervals = figure.intervals.map(i32::from);
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_comp(jam: u32, comp: i32, genre: u32) -> u32 {
    scheduling_tables::comp_mask(jam != 0, comp, genre as usize) as u32
}
static COUNTERS: [AtomicU32; 6] = [const { AtomicU32::new(0) }; 6];
fn count(index: usize) {
    COUNTERS[index].fetch_add(1, Ordering::Relaxed);
}
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.wrapping_sub(22) as usize)
        .map(|n| n.load(Ordering::Relaxed))
        .unwrap_or(0)
}
const _: [(); 92] = [(); core::mem::size_of::<Harmony>()];
const _: [(); 64] = [(); core::mem::size_of::<Acid>()];
const _: [(); 56] = [(); core::mem::size_of::<AcidConfig>()];
const _: [(); 28] = [(); core::mem::size_of::<AcidNote>()];
const _: [(); 200] = [(); core::mem::size_of::<Pluck>()];
const _: [(); 48] = [(); core::mem::size_of::<PluckConfig>()];
const _: [(); 52] = [(); core::mem::size_of::<PluckNote>()];
const _: [(); 44] = [(); core::mem::size_of::<Bass>()];
const _: [(); 64] = [(); core::mem::size_of::<BassConfig>()];
const _: [(); 16] = [(); core::mem::size_of::<BassNote>()];
const _: [(); 28] = [(); core::mem::size_of::<melodic::Start>()];

/// # Safety
/// `harmony` provides an initialized Harmony with nsteps1..12/chord_count0..4.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_melodic_avoid(note: i32, harmony: *const Harmony) -> i32 {
    melodic::avoid(note, unsafe { &*harmony })
}
#[no_mangle]
pub extern "C" fn fm1_rust_melodic_fold(note: i32, low: i32, high: i32) -> i32 {
    melodic::fold(note, low, high)
}
/// # Safety
/// State/config/harmony and seed follow the module contract; step<16 and the
/// config/harmonic scalar bounds match validated firmware parameters.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_roll(
    state: *mut Acid,
    config: *const AcidConfig,
    harmony: *const Harmony,
    step: u32,
    what: u32,
    seed: *mut u32,
) {
    let mut rng = Rng(unsafe { seed.read() });
    melodic::acid_roll(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        step as usize,
        what as i32,
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
}
/// # Safety
/// The module pointer contract applies; config.length is1..16.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_mutate(
    state: *mut Acid,
    config: *const AcidConfig,
    harmony: *const Harmony,
    seed: *mut u32,
) {
    let mut rng = Rng(unsafe { seed.read() });
    melodic::acid_mutate(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
    count(1);
}
/// # Safety
/// Inputs follow the readable module contract. State/config have firmware bounds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_preview(
    state: *const Acid,
    config: *const AcidConfig,
    harmony: *const Harmony,
    step: i32,
    value: i32,
) -> i32 {
    melodic::acid_preview(
        unsafe { &*state },
        unsafe { &*config },
        unsafe { &*harmony },
        step,
        value,
    )
}
/// # Safety
/// Inputs/state follow the module contract and Acid.position is within length.
/// Output provides one disjoint writable AcidNote; no C callback is made here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_step(
    state: *mut Acid,
    config: *const AcidConfig,
    harmony: *const Harmony,
    bar: u32,
    fill: u32,
    played: u32,
    held: u32,
    output: *mut AcidNote,
) {
    let result = melodic::acid_step(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        bar,
        fill != 0,
        played != 0,
        held != 0,
    );
    unsafe { output.write(result) };
    count(0);
}
/// # Safety
/// The initialized Acid state is exclusively writable and length is1..16.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_advance(state: *mut Acid, length: i32) -> u32 {
    u32::from(melodic::acid_advance(unsafe { &mut *state }, length))
}
#[no_mangle]
pub extern "C" fn fm1_rust_acid_answer(
    response: i32,
    every: i32,
    count: u32,
    origin: u32,
    fill: u32,
    drift: i32,
) -> u32 {
    count_bar();
    u32::from(melodic::answer_due(
        response,
        every,
        count,
        origin,
        fill != 0,
        drift,
    ))
}
fn count_bar() {
    count(4);
}
/// # Safety
/// Readable config/harmony follow the module contract; off/octave are disjoint
/// writable i32 outputs. Scale has1..12 entries and chord degree is valid.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_capture(
    config: *const AcidConfig,
    harmony: *const Harmony,
    note: i32,
    off: *mut i32,
    octave: *mut i32,
) {
    let (bo, bc) = melodic::acid_capture(unsafe { &*config }, unsafe { &*harmony }, note);
    unsafe {
        off.write(bo);
        octave.write(bc)
    };
}
/// # Safety
/// Inputs/state/seed follow the module contract and step is0..15. The output is
/// disjoint and writable. Slots, chord and parameter bounds are validated by C.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_pluck_step(
    state: *mut Pluck,
    config: *const PluckConfig,
    harmony: *const Harmony,
    step: i32,
    bars: u32,
    section: i32,
    seed: *mut u32,
    output: *mut PluckNote,
) {
    let mut rng = Rng(unsafe { seed.read() });
    let result = melodic::pluck_step(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        step,
        bars,
        section,
        &mut rng,
    );
    unsafe {
        seed.write(rng.0);
        output.write(result)
    };
    count(2);
}
/// # Safety
/// Module contract applies; slot is0..15 and scale/chord bounds are valid.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_pluck_capture(
    state: *mut Pluck,
    config: *const PluckConfig,
    harmony: *const Harmony,
    slot: u32,
    note: i32,
) {
    melodic::pluck_capture(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        slot as usize,
        note,
    );
}
/// # Safety
/// The readable module contract applies. `output` provides12 writable i32 words;
/// `length` provides one disjoint writable i32.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_pluck_ladder(
    config: *const PluckConfig,
    harmony: *const Harmony,
    output: *mut i32,
    length: *mut i32,
) {
    let c = unsafe { &*config };
    let (a, n) = melodic::ladder(unsafe { &*harmony }, c.range, c.octave);
    unsafe {
        output.cast::<[i32; 12]>().write(a);
        length.write(n as i32)
    };
}
/// # Safety
/// State/config/harmony/seed follow the module contract; step is0..15, config
/// bounds select a valid figure and output provides a disjoint writable result.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_step(
    state: *mut Bass,
    config: *const BassConfig,
    harmony: *const Harmony,
    step: i32,
    seed: *mut u32,
    output: *mut BassNote,
) {
    let mut rng = Rng(unsafe { seed.read() });
    let result = melodic::bass_step(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        step,
        &mut rng,
    );
    unsafe {
        seed.write(rng.0);
        output.write(result)
    };
    count(3);
}
/// # Safety
/// The module state contract applies to the initialized, exclusive Bass value.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_capture(state: *mut Bass) {
    melodic::bass_capture(unsafe { &mut *state });
}
/// # Safety
/// State/harmony follow the module contract; step0..15 and pitch are bounded.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_record(
    state: *mut Bass,
    harmony: *const Harmony,
    step: i32,
    note: i32,
    root: i32,
) {
    melodic::bass_record(unsafe { &mut *state }, step, note, root, unsafe {
        &*harmony
    });
}
/// # Safety
/// Harmony follows the module contract and degree/step selects a valid scale.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_semis(step: i32, harmony: *const Harmony) -> i32 {
    melodic::bass_semis(step, unsafe { &*harmony })
}
/// # Safety
/// Initialized config/harmony/state/seed follow the module contract and bounds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_regrow(
    state: *mut Acid,
    config: *const AcidConfig,
    harmony: *const Harmony,
    seed: *mut u32,
) {
    let mut rng = Rng(unsafe { seed.read() });
    melodic::acid_regrow(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &*harmony },
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
    count(1);
}
/// # Safety
/// Exclusive state has firmware bounds; step<16 and fields have decoded ranges.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_edit(
    state: *mut Acid,
    step: u32,
    offset: i32,
    octave: i32,
    gate: u32,
    accent: u32,
    slide: u32,
) {
    melodic::acid_edit(
        unsafe { &mut *state },
        step as usize,
        offset,
        octave,
        gate != 0,
        accent != 0,
        slide != 0,
    );
}
/// # Safety
/// Exclusive state has firmware bounds; step<16 and fields have decoded ranges.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_pluck_edit(state: *mut Pluck, step: u32, rung: i32, level: i32) {
    melodic::pluck_edit(unsafe { &mut *state }, step as usize, rung, level);
}
/// # Safety
/// Exclusive initialized state follows the module contract; the command fields
/// were decoded from the unchanged packed command word by its C dispatcher.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_bass_edit(state: *mut Bass, command: u32, value: u32) -> u32 {
    u32::from(melodic::bass_edit(unsafe { &mut *state }, command, value))
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_source(request: i32, current: i32) -> u32 {
    count(4);
    melodic::source(request, current)
}
/// # Safety
/// The remaining bar counter is initialized/aligned/exclusively writable.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_build(section: i32, left: *mut i32) -> u32 {
    count(4);
    u32::from(melodic::build_due(section, unsafe { &mut *left }))
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_groove(section: i32, bars: u32) -> u32 {
    melodic::groove_bar(section, bars)
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_wobble(flags: u32, section: i32, groove: u32, bars: u32) -> i32 {
    melodic::wobble(flags, section, groove, bars)
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_dj(
    sync: u32,
    section: i32,
    playing: u32,
    frozen: u32,
    pending: u32,
    length: i32,
    left: i32,
    total: f32,
    remaining: f32,
    step_length: f32,
) -> u32 {
    count(4);
    melodic::dj_action(
        sync != 0,
        section,
        playing != 0,
        frozen != 0,
        pending != 0,
        length,
        left,
        total,
        remaining,
        step_length,
    )
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_snare(section: i32, length: i32, left: i32, step: i32) -> f32 {
    melodic::snare_rise(section, length, left, step)
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_perc(
    hold: u32,
    fill: u32,
    drift: u32,
    section: i32,
    kind: u32,
    step: i32,
) -> f32 {
    melodic::perc_fall(hold != 0, fill != 0, drift != 0, section, kind, step)
}
/// # Safety
/// The render owner uniquely owns the initialized/aligned shared RNG word.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_pad(
    jam: u32,
    stabs: u32,
    manual: u32,
    playing: u32,
    mask: u32,
    step: i32,
    section: i32,
    comp: i32,
    evolve: f32,
    seed: *mut u32,
) -> f32 {
    let mut rng = Rng(unsafe { seed.read() });
    let result = melodic::pad_step(
        jam != 0,
        stabs != 0,
        manual != 0,
        playing != 0,
        mask,
        step,
        section,
        comp,
        evolve,
        &mut rng,
    );
    unsafe { seed.write(rng.0) };
    result
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_acid_link(
    jam: u32,
    linked: u32,
    perc: u32,
    section: i32,
    hit: u32,
) -> u32 {
    u32::from(melodic::acid_link(
        jam != 0,
        linked != 0,
        perc != 0,
        section,
        hit != 0,
    ))
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_harmonic_due(jam: u32, step: i32, pending: u32, frozen: u32) -> u32 {
    u32::from(melodic::harmonic_due(
        jam != 0,
        step,
        pending != 0,
        frozen != 0,
    ))
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_drift_arm(
    every: i32,
    change: i32,
    mutation: i32,
    countdown: i32,
) -> i32 {
    melodic::drift_arm(every, change, mutation, countdown)
}
/// # Safety
/// `left` is initialized/aligned and uniquely writable for this call. The C
/// render owner gates this clock while Songbook, stopped or frozen.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_harmonic_samples(
    pending: u32,
    harmonic: u32,
    left: *mut f32,
    frames: u32,
) -> u32 {
    count(4);
    u32::from(melodic::harmonic_samples(
        pending != 0,
        harmonic != 0,
        unsafe { &mut *left },
        frames,
    ))
}
/// # Safety
/// Counter and two float outputs are disjoint, initialized/aligned and uniquely
/// writable; all scalar inputs follow validated render bounds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_harmonic(
    jam: u32,
    step: i32,
    playing: u32,
    frozen: u32,
    length: i32,
    left: *mut i32,
    step_length: f32,
    total: *mut f32,
    remaining: *mut f32,
) -> u32 {
    if let Some((request, t, r)) = melodic::harmonic_advance(
        jam != 0,
        step,
        playing != 0,
        frozen != 0,
        length,
        unsafe { &mut *left },
        step_length,
    ) {
        unsafe {
            total.write(t);
            remaining.write(r)
        };
        0x80000000 | u32::from(request)
    } else {
        0
    }
}
/// # Safety
/// Readable pointers provide eight note bytes/eight i32 deadlines, no mutation.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_midi_slot(
    notes: *const u8,
    waits: *const i32,
    note: i32,
) -> u32 {
    count(5);
    melodic::midi_slot(
        &unsafe { notes.cast::<[u8; 8]>().read_unaligned() },
        &unsafe { waits.cast::<[i32; 8]>().read_unaligned() },
        note,
    )
}
/// # Safety
/// The initialized/aligned deadline is exclusively writable by the caller.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_midi_due(present: u32, wait: *mut i32, frames: i32) -> u32 {
    count(5);
    u32::from(melodic::midi_due(
        present != 0,
        unsafe { &mut *wait },
        frames,
    ))
}
/// # Safety
/// The disjoint16-deadline/six-ratchet arrays are initialized and exclusively
/// writable. No voice/render/input user accesses cancelled slots concurrently.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_cancel(waits: *mut i32, ratchets: *mut u32) {
    melodic::clear_pending(unsafe { &mut *waits.cast::<[i32; 16]>() }, unsafe {
        &mut *ratchets.cast::<[u32; 6]>()
    });
    count(5);
}
/// # Safety
/// State is initialized/aligned/exclusively writable; next is a valid pattern.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_start(state: *mut melodic::Start, next: i32) {
    melodic::start(unsafe { &mut *state }, next);
    count(5);
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_message(external: u32, from_top: u32, stop: u32) -> u32 {
    melodic::transport_message(external != 0, from_top != 0, stop != 0)
}
/// # Safety
/// Output provides five writable u32 words without concurrent mutation. FX
/// callbacks retain the documented independent arm-flag contract.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mel_tick_actions(
    scatter: u32,
    reverse: u32,
    exit: u32,
    repeat: u32,
    output: *mut u32,
) {
    unsafe {
        output
            .cast::<[u32; 5]>()
            .write(melodic::tick_actions(scatter, reverse, exit, repeat))
    };
    count(4);
}
/// # Safety
/// State is initialized/aligned/exclusively writable; operation identifies the
/// loop/lock parameter hook and value is its validated current parameter.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_pluck_parameter(state: *mut Pluck, operation: u32, value: i32) {
    melodic::pluck_parameter(unsafe { &mut *state }, operation, value);
}
/// # Safety
/// State is initialized/aligned/exclusively writable; length is1..16.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_acid_length(state: *mut Acid, length: i32) {
    melodic::acid_length(unsafe { &mut *state }, length);
}
#[no_mangle]
pub extern "C" fn fm1_rust_mel_origin(count: u32, origin: u32, resync: u32) -> u32 {
    melodic::answer_origin(count, origin, resync != 0)
}
