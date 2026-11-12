// SPDX-License-Identifier: GPL-3.0-only
//! Scoped audio-owner borrows. State, input/output and delay regions are aligned,
//! initialized and disjoint; the worker must not access these master arrays.
use fm1_engine::effects::{self, master};
use fm1_rt::atomic::{AtomicU32, Ordering};
static MASTER_SAMPLES: AtomicU32 = AtomicU32::new(0);
static ECHO_SAMPLES: AtomicU32 = AtomicU32::new(0);
static REVERB_SAMPLES: AtomicU32 = AtomicU32::new(0);
static COLOUR_SAMPLES: AtomicU32 = AtomicU32::new(0);

/// Valid effect setting values are bounded by the C parameter schema.
#[no_mangle]
pub extern "C" fn fm1_rust_fx_setting(kind: u32, value: i32) -> f32 {
    match kind {
        0 => effects::config::send(value),
        1 => effects::config::pump(value),
        2 => effects::config::trim(value),
        _ => 0.0,
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_fx_duck_decay(step: f32) -> f32 {
    effects::config::duck_decay(step)
}
#[no_mangle]
pub extern "C" fn fm1_rust_fx_reduction(hold: f32) -> f32 {
    effects::config::reduction(hold)
}
/// # Safety
/// Output is an aligned exclusively writable pair; value is in 0..=100.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_size(value: i32, output: *mut f32) {
    unsafe {
        output
            .cast::<[f32; 2]>()
            .write(effects::config::size(value));
    }
}
/// # Safety
/// Output provides three aligned writable f32s and is exclusively held.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_gains(
    previous: f32,
    target: f32,
    muted: u32,
    q12: u32,
    output: *mut f32,
) {
    unsafe {
        output
            .cast::<[f32; 3]>()
            .write(effects::config::gains(previous, target, muted != 0, q12));
    }
}
/// # Safety
/// State provides two initialized exclusively writable f32s. Output/optional
/// non-null USB cover 2*n initialized i32/i16s. Output is a raw external span,
/// not a Rust reference; an active DMA reader is permitted by the sealed sink.
/// DSP/state/USB spans are aligned, disjoint and exclusively held; in hardware
/// mode the caller obeys the CPU0 HAL registration/render contract. No worker
/// accesses these regions and the sink never reenters engine/UI code.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_fades(
    state: *mut f32,
    gap: u32,
    hush: u32,
    out: *mut i32,
    usb: *mut i16,
    n: u32,
) -> u32 {
    let frames = n as usize;
    let mut sink = unsafe { crate::raw_output::Sink::new(out, frames * 2) };
    let mut usb = if usb.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts_mut(usb, frames * 2) })
    };
    u32::from(effects::config::fades_each(
        unsafe { &mut *state.cast::<[f32; 2]>() },
        gap != 0,
        hush != 0,
        frames,
        |j, gain| {
            for ch in 0..2 {
                let index = 2 * j + ch;
                if let Some(word) = sink.read(index) {
                    sink.write(index, (word as f32 * gain) as i32);
                }
            }
            if let Some(b) = &mut usb {
                for ch in 0..2 {
                    b[2 * j + ch] = (b[2 * j + ch] as f32 * gain) as i16;
                }
            }
        },
    ))
}

/// # Safety
/// Eight initialized aligned start/end values are disjoint, with start
/// exclusively writable. Frames is nonzero, as enforced by render chunking.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_hold(start: *mut f32, end: *const f32, frames: u32) -> f32 {
    effects::config::hold(
        unsafe { &mut *start.cast::<[f32; 8]>() },
        unsafe { &*end.cast::<[f32; 8]>() },
        frames,
    )
}

/// # Safety
/// Initialized scalar state/config, performance ring(42000i16), tape/flanger
/// (each2x512f32), seed and two n-f32 outputs are aligned/pairwise disjoint.
/// Foreground owns all writable spans until return; spectrum/UI readers must
/// remain serialized. No worker accesses these spans. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_render(
    sampler: *mut effects::sampler::State,
    sampler_config: *const effects::sampler::Config,
    tone: *mut effects::tone::State,
    tone_config: *const effects::tone::Config,
    ring: *mut i16,
    tape: *mut f32,
    flanger: *mut f32,
    seed: *mut u32,
    left: *mut f32,
    right: *mut f32,
    n: u32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    effects::render::process(
        effects::render::State {
            sampler: unsafe { &mut *sampler },
            tone: unsafe { &mut *tone },
            rng: &mut rng,
        },
        unsafe { &*sampler_config },
        unsafe { &*tone_config },
        effects::render::Buffers {
            ring: unsafe { &mut *ring.cast::<[i16; 42000]>() },
            tape: unsafe { &mut *tape.cast::<[[f32; 512]; 2]>() },
            flanger: unsafe { &mut *flanger.cast::<[[f32; 512]; 2]>() },
            left: unsafe { core::slice::from_raw_parts_mut(left, n as usize) },
            right: unsafe { core::slice::from_raw_parts_mut(right, n as usize) },
        },
    );
    unsafe {
        seed.write(rng.0);
    }
    COLOUR_SAMPLES.fetch_add(n.wrapping_mul(2), Ordering::Relaxed);
}

/// # Safety
/// State/config/tank33383i16/shimmer2048f32 and n-f32 outputs are
/// initialized, aligned and pairwise disjoint. Worker owns writable spans until
/// stage release. Input borrows ONLY the current consumed predelay span, which
/// cannot overlap the foreground's current write span, including ring wrap.
/// `input` covers `first` i16s and `wrapped` covers n-first; first<=n<=256.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_reverb_block(
    state: *mut effects::reverb::State,
    config: *const effects::reverb::Config,
    tank: *mut i16,
    shimmer: *mut f32,
    input: *const i16,
    first: u32,
    wrapped: *const i16,
    left: *mut f32,
    right: *mut f32,
    n: u32,
) {
    let first = first as usize;
    let n = n as usize;
    let state = unsafe { &mut *state };
    let config = unsafe { &*config };
    let tank = unsafe { &mut *tank.cast::<[i16; 33383]>() };
    let shimmer = unsafe { &mut *shimmer.cast::<[f32; 2048]>() };
    let (l0, l1) = unsafe { core::slice::from_raw_parts_mut(left, n) }.split_at_mut(first);
    let (r0, r1) = unsafe { core::slice::from_raw_parts_mut(right, n) }.split_at_mut(first);
    effects::reverb::process(
        state,
        config,
        tank,
        shimmer,
        unsafe { core::slice::from_raw_parts(input, first) },
        l0,
        r0,
    );
    effects::reverb::process(
        state,
        config,
        tank,
        shimmer,
        unsafe { core::slice::from_raw_parts(wrapped, n - first) },
        l1,
        r1,
    );
    REVERB_SAMPLES.fetch_add(n as u32, Ordering::Relaxed);
}
/// # Safety
/// State and tank33383i16 are aligned, initialized, disjoint and exclusively
/// held after confirmed join/stop or at serialized boot. No live worker.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_reverb_reset(
    state: *mut effects::reverb::State,
    tank: *mut i16,
) {
    effects::reverb::reset(unsafe { &mut *state }, unsafe {
        &mut *tank.cast::<[i16; 33383]>()
    });
}
/// # Safety
/// Scalar state/head packets, tape21800i16 and plex7100i16 are initialized,
/// aligned and disjoint. The serialized engine owns them all with no live job.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_echo_reset(
    state: *mut effects::echo::State,
    heads: *mut effects::echo::Heads,
    tape: *mut i16,
    plex: *mut i16,
) {
    effects::echo::reset(
        unsafe { &mut *state },
        unsafe { &mut *heads },
        unsafe { &mut *tape.cast::<[i16; 21800]>() },
        unsafe { &mut *plex.cast::<[i16; 7100]>() },
    );
}

// Six distinct foreground working arrays; these never include a shared ring.
unsafe fn mix_buffers<'a>(p: *const *mut f32, n: u32) -> effects::mix::Buffers<'a> {
    let p = unsafe { &*p.cast::<[*mut f32; 6]>() };
    let n = n as usize;
    effects::mix::Buffers {
        left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
        right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
        reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
        echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        bus_left: unsafe { core::slice::from_raw_parts_mut(p[4], n) },
        bus_right: unsafe { core::slice::from_raw_parts_mut(p[5], n) },
    }
}
/// # Safety
/// Six initialized/aligned n-f32 working spans are disjoint and exclusively
/// held by render. The initialized pointer table is readable for this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_route_plain(p: *const *mut f32, n: u32) {
    effects::mix::route_plain(unsafe { mix_buffers(p, n) });
}
/// # Safety
/// The route_plain contract applies. The two initialized before spans are
/// readable n-f32 regions, aligned and disjoint from all writable spans.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_route_drums(
    p: *const *mut f32,
    l: *const f32,
    r: *const f32,
    n: u32,
) {
    effects::mix::route_drums(
        unsafe { mix_buffers(p, n) },
        unsafe { core::slice::from_raw_parts(l, n as usize) },
        unsafe { core::slice::from_raw_parts(r, n as usize) },
    );
}
/// # Safety
/// Output spans/table obey route_plain. Config and seven input pointers are
/// initialized/aligned/readable. Each input covers exactly n f32s, disjoint
/// from output. Pad pointers describe only the consumed contiguous ring span;
/// the worker's current write span is disjoint, including at ring wrap. No
/// borrow of the rest of that shared ring is formed or retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_pad_fm(
    c: *const effects::mix::MergeConfig,
    out: *const *mut f32,
    input: *const *const f32,
    n: u32,
) {
    let p = unsafe { &*input.cast::<[*const f32; 7]>() };
    let len = n as usize;
    effects::mix::pad_fm(
        unsafe { mix_buffers(out, n) },
        unsafe { &*c },
        effects::mix::PadFm {
            pad_left: unsafe { core::slice::from_raw_parts(p[0], len) },
            pad_right: unsafe { core::slice::from_raw_parts(p[1], len) },
            pad_dry: unsafe { core::slice::from_raw_parts(p[2], len) },
            fm_left: unsafe { core::slice::from_raw_parts(p[3], len) },
            fm_right: unsafe { core::slice::from_raw_parts(p[4], len) },
            fm_reverb: unsafe { core::slice::from_raw_parts(p[5], len) },
            fm_echo: unsafe { core::slice::from_raw_parts(p[6], len) },
        },
    );
}
/// # Safety
/// Output spans/table obey route_plain. Config and four n-f32 input spans are
/// initialized/aligned/readable and disjoint from output. C has joined/fenced
/// the bass/acid stage before creating these scoped immutable input borrows.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_bass_acid(
    c: *const effects::mix::MergeConfig,
    out: *const *mut f32,
    input: *const *const f32,
    n: u32,
) {
    let p = unsafe { &*input.cast::<[*const f32; 4]>() };
    let len = n as usize;
    effects::mix::bass_acid(
        unsafe { mix_buffers(out, n) },
        unsafe { &*c },
        effects::mix::BassAcid {
            bass: unsafe { core::slice::from_raw_parts(p[0], len) },
            acid: unsafe { core::slice::from_raw_parts(p[1], len) },
            duck: unsafe { core::slice::from_raw_parts(p[2], len) },
            gate: unsafe { core::slice::from_raw_parts(p[3], len) },
        },
    );
}
#[no_mangle]
pub extern "C" fn fm1_rust_mix_reverb_gain(send: f32, gain: f32, arc: f32, breakdown: f32) -> f32 {
    effects::mix::reverb_gain(send, gain, arc, breakdown)
}
#[no_mangle]
pub extern "C" fn fm1_rust_mix_duck_depth(kit: f32, pulse: f32, kick: f32) -> f32 {
    effects::mix::duck_depth(kit, pulse, kick)
}
/// # Safety
/// Output spans/table obey route_plain. Echo state, config, sends, the writable
/// 21800/7100-i16 delays and n-f32 duck span are initialized/aligned and disjoint
/// from those outputs and each other. Render exclusively holds mutation. No
/// shared predelay borrow/callback is created. Reverb output slots hold values
/// for C to commit later in original order into disjoint predelay write slots.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_echo_block(
    s: *mut effects::echo::State,
    c: *const effects::echo::Config,
    tape: *mut i16,
    plex: *mut i16,
    sends: *const effects::mix::EchoSends,
    out: *const *mut f32,
    duck: *mut f32,
    n: u32,
) {
    effects::mix::echoes(
        unsafe { &mut *s },
        unsafe { &*c },
        unsafe { &mut *tape.cast::<[i16; 21800]>() },
        unsafe { &mut *plex.cast::<[i16; 7100]>() },
        unsafe { &*sends },
        unsafe { mix_buffers(out, n) },
        unsafe { core::slice::from_raw_parts_mut(duck, n as usize) },
    );
    ECHO_SAMPLES.fetch_add(n, Ordering::Relaxed);
}
/// # Safety
/// Output spans/table obey route_plain. Three n-f32 inputs are initialized,
/// aligned/readable and disjoint from outputs. C has joined/fenced the tank
/// stage before the immutable output borrows, even if the FM job is still live.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_mix_tank(
    out: *const *mut f32,
    l: *const f32,
    r: *const f32,
    duck: *const f32,
    n: u32,
) {
    effects::mix::tank_return(
        unsafe { mix_buffers(out, n) },
        unsafe { core::slice::from_raw_parts(l, n as usize) },
        unsafe { core::slice::from_raw_parts(r, n as usize) },
        unsafe { core::slice::from_raw_parts(duck, n as usize) },
    );
}

/// # Safety
/// Initialized/aligned scalar packets and two writable n-f32 arrays are
/// pairwise disjoint, exclusively held by render until return. No retained
/// pointer or shared drum RNG borrow is involved: vinyl owns its two seeds.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_vinyl(
    s: *mut effects::vinyl::State,
    c: *const effects::vinyl::Config,
    l: *mut f32,
    r: *mut f32,
    n: u32,
) {
    effects::vinyl::process(
        unsafe { &mut *s },
        unsafe { &*c },
        unsafe { core::slice::from_raw_parts_mut(l, n as usize) },
        unsafe { core::slice::from_raw_parts_mut(r, n as usize) },
    );
}

/// # Safety
/// Packets and ten pointer slots are initialized/aligned. Each slot provides n
/// initialized f32s. All ten regions are pairwise disjoint; render holds slots
/// 0..8 exclusively for writes and slots 8..10 read-only throughout the call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_duck(
    s: *mut effects::mix::Duck,
    c: *const effects::mix::DuckConfig,
    p: *const *mut f32,
    n: u32,
) {
    let p = unsafe { &*p.cast::<[*mut f32; 10]>() };
    let n = n as usize;
    effects::mix::duck(
        unsafe { &mut *s },
        unsafe { &*c },
        effects::mix::DuckBuffers {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
            bus_left: unsafe { core::slice::from_raw_parts_mut(p[4], n) },
            bus_right: unsafe { core::slice::from_raw_parts_mut(p[5], n) },
            factors: unsafe { core::slice::from_raw_parts_mut(p[6], n) },
            gates: unsafe { core::slice::from_raw_parts_mut(p[7], n) },
            noise_left: unsafe { core::slice::from_raw_parts(p[8], n) },
            noise_right: unsafe { core::slice::from_raw_parts(p[9], n) },
        },
    );
}
/// # Safety
/// Initialized scalar state and four n-f32 regions are aligned/disjoint. Render
/// exclusively holds state and left/right, and holds before arrays read-only.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_crush(
    s: *mut effects::mix::Crush,
    l: *mut f32,
    r: *mut f32,
    bl: *const f32,
    br: *const f32,
    n: u32,
    amount: f32,
) {
    effects::mix::crush(
        unsafe { &mut *s },
        effects::mix::CrushBuffers {
            left: unsafe { core::slice::from_raw_parts_mut(l, n as usize) },
            right: unsafe { core::slice::from_raw_parts_mut(r, n as usize) },
            before_left: unsafe { core::slice::from_raw_parts(bl, n as usize) },
            before_right: unsafe { core::slice::from_raw_parts(br, n as usize) },
        },
        amount,
    );
}

/// # Safety
/// Initialized aligned scalar packets and RNG word are pairwise disjoint;
/// render holds state/RNG exclusively until return. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_heads(
    s: *mut effects::echo::Heads,
    c: *const effects::echo::HeadConfig,
    n: u32,
    seed: *mut u32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    effects::echo::heads(unsafe { &mut *s }, unsafe { &*c }, n, &mut rng);
    unsafe { seed.write(rng.0) };
}
/// # Safety
/// Beats provides six initialized f32 values and output two writable f32s.
/// Both are aligned/disjoint; index is a valid table index, render owns output.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_echo_length(
    index: u32,
    tempo: i32,
    current: f32,
    beats: *const f32,
    out: *mut f32,
) -> u32 {
    if let Some(value) = effects::echo::length(index as usize, tempo, current, unsafe {
        &*beats.cast::<[f32; 6]>()
    }) {
        unsafe { out.cast::<[f32; 2]>().write(value) };
        1
    } else {
        0
    }
}
/// # Safety
/// Output is an aligned exclusively writable Settings packet.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_reverb_set(
    out: *mut effects::reverb::Settings,
    kind: i32,
    damp: i32,
    pre: i32,
    x: i32,
) {
    unsafe { out.write(effects::reverb::settings(kind, damp, pre, x)) };
}
/// # Safety
/// State provides three initialized aligned writable f32s, exclusively held.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_decay(state: *mut f32, target: f32, frozen: u32) {
    effects::reverb::decay(
        unsafe { &mut *state.cast::<[f32; 3]>() },
        target,
        frozen != 0,
    );
}

/// # Safety
/// Render exclusively holds initialized aligned state and RNG, disjoint from
/// initialized config. Kind/argument come from the bounded C callback adapter;
/// repeat indices and Flex bytes are valid. No buffer pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_engage(
    s: *mut effects::control_abi::Engage,
    c: *const effects::control_abi::Config,
    seed: *mut u32,
    kind: u32,
    arg: i32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    effects::engage::apply(unsafe { &mut *s }, unsafe { &*c }, &mut rng, kind, arg);
    unsafe { seed.write(rng.0) };
}

/// # Safety
/// Aligned initialized state/grid/config/seed packets are disjoint and owned
/// by the foreground. Table provides the immutable 27x16 Flex bytes. Pattern,
/// arm and repeat indices obey existing engine bounds; no pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_grid(
    s: *mut effects::control_abi::Engage,
    g: *mut effects::control_abi::Grid,
    q: *const effects::control_abi::Config,
    seed: *mut u32,
    table: *const u8,
    kind: u32,
    step: i32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    effects::grid::apply(
        unsafe { &mut *s },
        unsafe { &mut *g },
        unsafe { &*q },
        &mut rng,
        unsafe { &*table.cast::<[[u8; 16]; 27]>() },
        kind,
        step,
    );
    unsafe {
        seed.write(rng.0);
    }
}

/// # Safety
/// Initialized aligned scalar packets and four `n`-sample arrays are disjoint.
/// The render owner exclusively holds state and all four writable arrays until
/// return; no callback or retained pointer crosses the worker ownership seam.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_bus(
    s: *mut effects::bus::State,
    c: *const effects::bus::Config,
    l: *mut f32,
    r: *mut f32,
    ml: *mut f32,
    mr: *mut f32,
    n: u32,
) {
    effects::bus::process(
        unsafe { &mut *s },
        unsafe { &*c },
        effects::bus::Buffers {
            left: unsafe { core::slice::from_raw_parts_mut(l, n as usize) },
            right: unsafe { core::slice::from_raw_parts_mut(r, n as usize) },
            mix_left: unsafe { core::slice::from_raw_parts_mut(ml, n as usize) },
            mix_right: unsafe { core::slice::from_raw_parts_mut(mr, n as usize) },
        },
    );
}

/// # Safety
/// Scalar packets are initialized, aligned and disjoint. Render exclusively
/// holds mutable state; no callback or engine pointer is retained. The caller
/// performs the returned single engagement action after copying state back.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_control(
    state: *mut effects::control_abi::State,
    config: *const effects::control_abi::Config,
    n: u32,
) -> u32 {
    effects::control::chunk(unsafe { &mut *state }, unsafe { &*config }, n)
}

/// # Safety
/// `state` is initialized, aligned and exclusively writable by the render owner.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_master_set(
    state: *mut master::State,
    ratio: i32,
    threshold: i32,
    makeup: i32,
    ceiling: i32,
) {
    master::set(unsafe { &mut *state }, ratio, threshold, makeup, ceiling);
}
unsafe fn buffers<'a>(l: *mut f32, r: *mut f32, g: *mut f32, s: *mut f32) -> master::Buffers<'a> {
    master::Buffers {
        left: unsafe { &mut *l.cast::<[f32; 64]>() },
        right: unsafe { &mut *r.cast::<[f32; 64]>() },
        gains: unsafe { &mut *g.cast::<[f32; 64]>() },
        sub: unsafe { &mut *s.cast::<[f32; 8]>() },
    }
}
/// # Safety
/// All arrays provide 64/64/64/8 aligned initialized f32 values. They and state
/// are pairwise disjoint and exclusively held; no callback or pointer retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_master_reset(
    state: *mut master::State,
    l: *mut f32,
    r: *mut f32,
    g: *mut f32,
    s: *mut f32,
) {
    master::reset(unsafe { &mut *state }, unsafe { buffers(l, r, g, s) });
}
/// # Safety
/// The reset array/state contract applies. `left` and `right` each additionally
/// provide `n` initialized aligned writable f32 values disjoint from all others.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_master_block(
    state: *mut master::State,
    l: *mut f32,
    r: *mut f32,
    g: *mut f32,
    s: *mut f32,
    left: *mut f32,
    right: *mut f32,
    n: u32,
) {
    master::process(
        unsafe { &mut *state },
        unsafe { buffers(l, r, g, s) },
        unsafe { core::slice::from_raw_parts_mut(left, n as usize) },
        unsafe { core::slice::from_raw_parts_mut(right, n as usize) },
    );
    MASTER_SAMPLES.fetch_add(n, Ordering::Relaxed);
}
/// # Safety
/// `output` supplies three aligned writable f32 values, exclusively held.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_mixer_gains(pan: i32, send: i32, output: *mut f32) {
    unsafe {
        output
            .cast::<[f32; 3]>()
            .write(effects::mixer_gains(pan, send))
    };
}
/// # Safety
/// `mix`/`before` supply `n` initialized aligned f32s, are disjoint, and cannot
/// be accessed by another owner during this scoped call. Output is writable.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_mixer_end(
    mix: *mut f32,
    before: *const f32,
    n: u32,
    gain: f32,
) {
    effects::mixer_end(
        unsafe { core::slice::from_raw_parts_mut(mix, n as usize) },
        unsafe { core::slice::from_raw_parts(before, n as usize) },
        gain,
    );
}
/// # Safety
/// Inputs are `n` initialized readable aligned f32s; out is `2*n` writable i32s
/// and optional non-null USB is `2*n` writable i16s. Output stays a raw span,
/// with no Rust reference live; an active DMA reader is allowed by the sealed
/// sink. DSP/USB regions are pairwise disjoint and exclusively held. In hardware
/// mode the CPU0 HAL registration/render contract holds. Gain/state is finite.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_output(
    left: *const f32,
    right: *const f32,
    n: u32,
    out: *mut i32,
    usb: *mut i16,
    gain: f32,
    usb_gain: f32,
    shed: u32,
) {
    let n = n as usize;
    let left = unsafe { core::slice::from_raw_parts(left, n) };
    let right = unsafe { core::slice::from_raw_parts(right, n) };
    let mut sink = unsafe { crate::raw_output::Sink::new(out, n * 2) };
    for j in 0..n {
        let words = effects::output_frame(left[j], right[j], gain, shed != 0);
        sink.write(2 * j, words[0]);
        sink.write(2 * j + 1, words[1]);
    }
    if !usb.is_null() {
        let usb = unsafe { core::slice::from_raw_parts_mut(usb, n * 2) };
        for j in 0..n {
            let words = effects::usb_frame(left[j], right[j], usb_gain);
            usb[2 * j] = words[0];
            usb[2 * j + 1] = words[1];
        }
    }
}
pub fn stat(index: u32) -> u32 {
    match index {
        28 => MASTER_SAMPLES.load(Ordering::Relaxed),
        29 => ECHO_SAMPLES.load(Ordering::Relaxed),
        30 => REVERB_SAMPLES.load(Ordering::Relaxed),
        31 => COLOUR_SAMPLES.load(Ordering::Relaxed),
        _ => 0,
    }
}

/// # Safety
/// The worker exclusively owns initialized state, the 2x512-f32 chorus delay,
/// and `n` initialized left/right samples. All are aligned and pairwise disjoint.
/// No borrow of the concurrently consumed pad output ring is created here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_chorus(
    state: *mut effects::chorus::State,
    buffers: *mut f32,
    left: *mut f32,
    right: *mut f32,
    n: u32,
    amount: i32,
    shed: u32,
) {
    effects::chorus::process(
        unsafe { &mut *state },
        unsafe { &mut *buffers.cast::<[[f32; 512]; 2]>() },
        unsafe { core::slice::from_raw_parts_mut(left, n as usize) },
        unsafe { core::slice::from_raw_parts_mut(right, n as usize) },
        effects::config::chorus(amount, shed),
    );
    COLOUR_SAMPLES.fetch_add(n, Ordering::Relaxed);
}

/// # Safety
/// Initialized/aligned scalar state/config and two distinct 2x512-f32 delays
/// are exclusively held by the render owner. Output provides two writable f32s
/// and is disjoint from all inputs. No engine pointer survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_tone(
    state: *mut effects::tone::State,
    config: *const effects::tone::Config,
    tape: *mut f32,
    flanger: *mut f32,
    left: f32,
    right: f32,
    output: *mut f32,
) {
    let value = effects::tone::sample(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &mut *tape.cast::<[[f32; 512]; 2]>() },
        unsafe { &mut *flanger.cast::<[[f32; 512]; 2]>() },
        left,
        right,
    );
    unsafe { output.cast::<[f32; 2]>().write(value) };
    COLOUR_SAMPLES.fetch_add(1, Ordering::Relaxed);
}

/// # Safety
/// The render owner uniquely holds initialized scalar state, the 42000-i16
/// performance ring and aligned RNG word. Config is initialized/readable;
/// output provides two writable f32s. All regions are disjoint, configured
/// offsets stay within one ring, and no UI/worker accesses this ring until
/// return. No caller pointer is retained and no foreign callback is invoked.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_sampler(
    state: *mut effects::sampler::State,
    config: *const effects::sampler::Config,
    ring: *mut i16,
    seed: *mut u32,
    left: f32,
    right: f32,
    output: *mut f32,
) {
    let mut rng = fm1_engine::sequencer::Rng(unsafe { seed.read() });
    let value = effects::sampler::sample(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &mut *ring.cast::<[i16; 42000]>() },
        &mut rng,
        left,
        right,
    );
    unsafe {
        seed.write(rng.0);
        output.cast::<[f32; 2]>().write(value)
    };
    COLOUR_SAMPLES.fetch_add(1, Ordering::Relaxed);
}

/// # Safety
/// The audio/worker owner uniquely holds initialized state, the 33383-i16 tank
/// and 2048-f32 shimmer storage. Predelay is excluded from this tank allocation;
/// its concurrent foreground writer never aliases these borrows. Config is
/// initialized/readable, output holds two writable f32s; all regions are aligned
/// and pairwise disjoint. All ring indices/configuration bounds remain valid.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_reverb(
    input: f32,
    state: *mut effects::reverb::State,
    config: *const effects::reverb::Config,
    tank: *mut i16,
    shimmer: *mut f32,
    output: *mut f32,
) {
    let value = effects::reverb::sample(
        input,
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &mut *tank.cast::<[i16; 33383]>() },
        unsafe { &mut *shimmer.cast::<[f32; 2048]>() },
    );
    unsafe { output.cast::<[f32; 2]>().write(value) };
    REVERB_SAMPLES.fetch_add(1, Ordering::Relaxed);
}

/// # Safety
/// State/config are aligned and initialized. Tape and plex provide 21800/7100
/// initialized i16 values, output provides four writable f32s. All regions are
/// disjoint and uniquely held for mutation; state has valid ring indices and
/// finite configured delay bounds. No pointer or borrow survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fx_echo(
    state: *mut effects::echo::State,
    config: *const effects::echo::Config,
    tape: *mut i16,
    plex: *mut i16,
    input: f32,
    output: *mut f32,
) {
    let value = effects::echo::sample(
        unsafe { &mut *state },
        unsafe { &*config },
        unsafe { &mut *tape.cast::<[i16; 21800]>() },
        unsafe { &mut *plex.cast::<[i16; 7100]>() },
        input,
    );
    unsafe { output.cast::<[f32; 4]>().write(value) };
    ECHO_SAMPLES.fetch_add(1, Ordering::Relaxed);
}
