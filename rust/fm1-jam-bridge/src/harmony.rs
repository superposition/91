// SPDX-License-Identifier: GPL-3.0-only
//! Engine/render/drain scalar snapshots and copied actions. No engine/voice pointer is
//! retained and no C callback executes while a Rust state/buffer borrow is live.
use fm1_engine::{
    harmony::{
        abi::{Bing, Config, Mood, State, Targets},
        basic, legacy,
    },
    sequencer::Rng,
};
use fm1_rt::atomic::{AtomicU32, Ordering};
static CALLS: AtomicU32 = AtomicU32::new(0);
static REGISTERS: AtomicU32 = AtomicU32::new(0);
static CHANGES: AtomicU32 = AtomicU32::new(0);
static REJECTIONS: AtomicU32 = AtomicU32::new(0);
const ERROR: u32 = 0x8000_0000;
pub fn stat(index: u32) -> u32 {
    match index {
        76 => CALLS.load(Ordering::Relaxed),
        77 => REGISTERS.load(Ordering::Relaxed),
        78 => CHANGES.load(Ordering::Relaxed),
        79 => REJECTIONS.load(Ordering::Relaxed),
        _ => 0,
    }
}
fn reject() -> u32 {
    REJECTIONS.fetch_add(1, Ordering::Relaxed);
    ERROR
}
fn bounded(x: i32) -> bool {
    (-1_000_000..=1_000_000).contains(&x)
}
fn valid(s: &State, c: &Config) -> bool {
    (0..19).contains(&c.genre)
        && (0..8).contains(&c.chord_kind)
        && (0..7).contains(&c.loop_mode)
        && (-8..=8).contains(&c.pad_oct)
        && (-8..=8).contains(&c.song_oct)
        && bounded(c.root)
        && (0..=12).contains(&s.nsteps)
        && (0..12).contains(&s.deg)
        && (0..=4).contains(&s.chord_nt)
        && s.pg_i < 8
        && s.pg_len <= 8
        && (s.pg_on == 0 || s.pg_len > 0)
        && s.drift_unit <= 3
        && s.dh_n <= 8
        && s.hist_n <= 8
        && [0, 2, 4, 8].contains(&s.dh_ln)
        && (-1..12).contains(&s.walk_next)
        && (-1..10).contains(&s.naiwan_pending)
        && s.naiwan_arc < 10
        && s.pg_deg.iter().all(|x| *x < 8)
        && s.pg_bars.iter().all(|x| *x < 4)
        && s.pg_kind.iter().all(|x| *x < 8)
}
/// # Safety
/// Output provides capacity for twelve aligned i32s. Only the returned initialized
/// prefix is written; old C callers may leave the unused tail uninitialized.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_scale(pool: u32, out: *mut i32) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let value = basic::scale(pool);
    for i in 0..value.count {
        unsafe {
            out.add(i).write(value.steps[i]);
        }
    }
    value.count as i32
}
/// # Safety
/// Output supplies four aligned writable i32s. Its unused tail is untouched.
/// Negative degree/root retain C remainder semantics within bounded integers.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_chord(
    pool: u32,
    root: i32,
    degree: i32,
    kind: i32,
    out: *mut i32,
) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let value = basic::chord(pool, root, degree, kind);
    for i in 0..value.count {
        unsafe {
            out.add(i).write(value.pcs[i]);
        }
    }
    value.count as i32
}
/// # Safety
/// From provides four initialized aligned i32s; pcs provides min(max(count,1),4)
/// initialized i32s; output has four writable aligned i32s. All spans stay live
/// and no other owner accesses them until return. They may overlap: only copied
/// values/raw bounded writes are used, with no aliased Rust references formed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_voice(
    from: *const i32,
    pcs: *const i32,
    count: i32,
    out: *mut i32,
    lo: i32,
    hi: i32,
) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let from_values = unsafe { from.cast::<[i32; 4]>().read() };
    let mut tones = [0; 4];
    for (i, t) in tones
        .iter_mut()
        .enumerate()
        .take(count.clamp(1, 4) as usize)
    {
        *t = unsafe { pcs.add(i).read() };
    }
    if !bounded(lo) || !bounded(hi) || !from_values.iter().chain(tones.iter()).all(|x| bounded(*x))
    {
        reject();
        return 0;
    }
    let mut aliases = [-1; 4];
    for (v, alias) in aliases.iter_mut().enumerate() {
        for j in 0..4 {
            if unsafe { out.add(v) }.cast_const() == unsafe { from.add(j) } {
                *alias = j as i8;
            }
        }
    }
    let to = basic::voice_lead(from_values, &tones, count, lo, hi, aliases);
    for (i, n) in to.iter().enumerate() {
        unsafe {
            out.add(i).write(*n);
        }
    }
    REGISTERS.fetch_add(1, Ordering::Relaxed);
    1
}
#[no_mangle]
pub extern "C" fn fm1_rust_harm_time(value: i32) -> f32 {
    legacy::drift_seconds(value)
}
#[no_mangle]
pub extern "C" fn fm1_rust_harm_bars(genre: u32, unit: u32, value: i32) -> i32 {
    if genre >= 19 || unit > 3 {
        reject();
        return 0;
    }
    legacy::genre_bars(genre as usize, unit as u8, value)
}
#[no_mangle]
pub extern "C" fn fm1_rust_harm_meter_bars(
    genre: u32,
    unit: u32,
    value: i32,
    playing: u32,
    log: u32,
) -> i32 {
    if genre >= 19 || unit > 3 || log > 3 {
        reject();
        return 0;
    }
    legacy::meter_bars(genre as usize, unit as u8, value, playing != 0, log as u8)
}
/// # Safety
/// State/config/seed are initialized, aligned, disjoint numeric snapshots owned
/// by one engine caller for the synchronous call. State has the existing bounded array/
/// index invariants. Returned actions are executed only AFTER this call returns.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_step(
    s: *mut State,
    c: *const Config,
    seed: *mut u32,
    kind: u32,
    a: u32,
    b: u32,
    d: u32,
) -> u32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let s = unsafe { &mut *s };
    let c = unsafe { &*c };
    if !valid(s, c) {
        return reject();
    }
    let mut rng = Rng(unsafe { seed.read() });
    let result = match kind {
        0 => {
            legacy::scale_set(s);
            0
        }
        1 => {
            legacy::dominant(s, c);
            0
        }
        2 => {
            legacy::clear_alterations(s);
            0
        }
        3 => legacy::walk(s, c, &mut rng) as u32,
        4 => {
            if a >= 12 {
                return reject();
            }
            u32::from(legacy::borrow_decision(s, c, a as usize, &mut rng))
        }
        5 => {
            if a >= 12 {
                return reject();
            }
            u32::from(legacy::alter(s, c, a as usize, b != 0, d as i32, &mut rng))
        }
        6 => {
            if a >= 12 {
                return reject();
            }
            CHANGES.fetch_add(1, Ordering::Relaxed);
            u32::from(legacy::chord_header(s, c, a as i32))
        }
        7 => {
            legacy::reset_drift(s, c, &mut rng);
            0
        }
        8 => {
            legacy::regrow(s, c);
            0
        }
        9 => {
            legacy::pool_set(s, c, a);
            0
        }
        10 => legacy::arc(s),
        11 => {
            if s.pg_on != 0 && s.nsteps == 0 {
                return reject();
            }
            legacy::begin_change(s, c, &mut rng)
        }
        12 => {
            if a >= 12 {
                return reject();
            }
            u32::from(legacy::finish_change(s, c, a as usize, b != 0, &mut rng))
        }
        13 => legacy::take_begin(s, c, a, b),
        14 => u32::from(legacy::take_finish(s, a)),
        15 => legacy::bars(s, c) as u32,
        16 => {
            legacy::edit_drift(s, c);
            0
        }
        17 => {
            if a >= 12 {
                return reject();
            }
            u32::from(legacy::diminished(s, a as usize))
        }
        18 => {
            s.naiwan_left = 8;
            0
        }
        _ => return reject(),
    };
    unsafe {
        seed.write(rng.0);
    }
    result
}
/// # Safety
/// State/config and four initialized current f32 targets are aligned/readable;
/// State and output are exclusively writable. All four regions are disjoint,
/// caller-owned snapshots. No voice state is borrowed or pointer retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_targets(
    s: *mut State,
    c: *const Config,
    from: *const f32,
    out: *mut Targets,
) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let s = unsafe { &mut *s };
    let c = unsafe { &*c };
    let from = unsafe { from.cast::<[f32; 4]>().read() };
    if !valid(s, c) || !from.iter().all(|x| x.is_finite() && x.abs() < 1_000_000.0) {
        reject();
        return 0;
    }
    let value = legacy::targets(s, c, from);
    unsafe {
        out.write(value);
    }
    REGISTERS.fetch_add(1, Ordering::Relaxed);
    1
}
/// # Safety
/// Aligned initialized state/config snapshots are readable; output has four
/// writable i32s disjoint from those snapshots. Only the active prefix is written.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_preview(
    s: *const State,
    c: *const Config,
    degree: i32,
    kind: i32,
    out: *mut i32,
) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let s = unsafe { &*s };
    let c = unsafe { &*c };
    if !valid(s, c) {
        reject();
        return -1;
    }
    let value = legacy::preview(s, c, degree, kind);
    for i in 0..value.count {
        unsafe {
            out.add(i).write(value.pcs[i]);
        }
    }
    value.count as i32
}
/// # Safety
/// State/config snapshots and six initialized writable progression bytes are
/// aligned/disjoint and caller-owned. Play/unit/extension bits are preserved.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_capture(
    s: *const State,
    c: *const Config,
    bytes: *mut u8,
) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let s = unsafe { &*s };
    let c = unsafe { &*c };
    if !valid(s, c) {
        reject();
        return 0;
    }
    let value = legacy::capture(s, c, unsafe { bytes.cast::<[u8; 6]>().read() });
    unsafe {
        bytes.cast::<[u8; 6]>().write(value);
    }
    1
}
/// # Safety
/// Mood and RNG word are initialized, aligned, disjoint and exclusively held
/// by the engine caller. No worker accesses the snapshots until return.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_mood(
    s: *mut Mood,
    n: u32,
    playing: u32,
    motion: f32,
    section: i32,
    seed: *mut u32,
) {
    CALLS.fetch_add(1, Ordering::Relaxed);
    let mut rng = Rng(unsafe { seed.read() });
    legacy::mood(
        unsafe { &mut *s },
        n,
        playing != 0,
        motion,
        section,
        &mut rng,
    );
    unsafe {
        seed.write(rng.0);
    }
}

/// # Safety
/// Output is an aligned, exclusively writable 32-byte event packet. The caller
/// dispatches its four queued notes only after this function returns.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_harm_bing(root: i32, up: u32, out: *mut Bing) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    if !bounded(root) || up > 1 {
        reject();
        return 0;
    }
    unsafe {
        out.write(basic::bing(root, up != 0));
    }
    1
}
#[no_mangle]
pub extern "C" fn fm1_rust_harm_bass(pedal: u32, root: i32, first: i32) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    basic::bass_pc(pedal, root, first)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialized_invalid_snapshots_do_not_advance_rng_or_mutate_state() {
        let mut state = State {
            nsteps: 13,
            pool: 0x5ad,
            steps: [17; 12],
            ..State::default()
        };
        let config = Config::default();
        let mut seed = 91;
        let result = unsafe { fm1_rust_harm_step(&mut state, &config, &mut seed, 3, 0, 0, 0) };
        assert_eq!(result, ERROR);
        assert_eq!(seed, 91);
        assert_eq!(state.nsteps, 13);
        assert_eq!(state.pool, 0x5ad);
        assert_eq!(state.steps, [17; 12]);
    }
    #[test]
    fn exported_helpers_write_only_active_prefixes_and_accept_a_single_tone() {
        let mut output = [0x12345678; 16];
        let n = unsafe { fm1_rust_harm_scale(0, output.as_mut_ptr().add(2)) };
        assert_eq!(n, 1);
        assert_eq!(output[2], 0);
        assert!(output
            .iter()
            .enumerate()
            .all(|(i, n)| i == 2 || *n == 0x12345678));
        let chord = unsafe { fm1_rust_harm_chord(1, 5, -1, 0, output.as_mut_ptr().add(3)) };
        assert_eq!(chord, 1);
        assert_eq!(output[3], 5);
        assert_eq!(output[4], 0x12345678);
        let from = [48, 55, 60, 64];
        let pcs = [0];
        let mut to = [0; 4];
        assert_eq!(
            unsafe { fm1_rust_harm_voice(from.as_ptr(), pcs.as_ptr(), 0, to.as_mut_ptr(), 48, 77) },
            1
        );
        assert!(to.iter().all(|n| *n % 12 == 0));
    }
}
