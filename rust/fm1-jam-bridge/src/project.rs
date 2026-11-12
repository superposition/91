// SPDX-License-Identifier: GPL-3.0-only
//! Borrow each distinct C-owned runtime field only for the duration of a call.
use fm1_engine::persistence_ui as ui;
use fm1_engine::project::{Globals, Objects, Patterns, Project, Runtime};
#[repr(C)]
pub struct Fields {
    runtime: *mut Runtime,
    image: *mut [u8; 3840],
    saved: *mut u16,
    pending: *mut u8,
    pending_globals: *mut u8,
    globals: *mut Globals,
    from_old: *mut i32,
    extras: *mut [[u8; 16]; 10],
    live_extras: *mut [[u8; 8]; 10],
    pending_extras: *mut u8,
    bass_dirty: *mut u16,
    load: unsafe extern "C" fn(u32, *mut u8, u32) -> i32,
    save: unsafe extern "C" fn(u32, *const u8, u32) -> i32,
}
struct Platform<'a>(&'a Fields);
impl Objects for Platform<'_> {
    fn load(&mut self, object: u32, output: &mut [u8]) -> i32 {
        // SAFETY: callback is synchronous and cannot access project fields.
        unsafe { (self.0.load)(object, output.as_mut_ptr(), output.len() as u32) }
    }
    fn save(&mut self, object: u32, input: &[u8]) -> i32 {
        crate::persistence::object_save(input.len());
        // SAFETY: callback is synchronous and cannot access project fields.
        unsafe { (self.0.save)(object, input.as_ptr(), input.len() as u32) }
    }
}
const _: [(); 578] = [(); core::mem::size_of::<fm1_engine::project::Slot>()];
const _: [(); 6194] = [(); core::mem::size_of::<Runtime>()];
const _: [(); 32] = [(); core::mem::size_of::<Globals>()];
const _: [(); 168] = [(); core::mem::size_of::<Patterns>()];
/// # Safety
/// Fields and all pointed ranges are initialized, aligned, mutually disjoint and
/// exclusively owned for this call. No platform callback may access these state
/// ranges or re-enter project/storage. patterns is a distinct initialized scratch
/// snapshot: engine volatile arrays must be copied in/out after this call. Every
/// callback finishes synchronously and retains no pointer. Nothing escapes.
/// recipe_effect is a distinct writable aligned i32; C replays a nonnegative
/// effect only after this function has released every project field borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_project(
    operation: u32,
    fields: *const Fields,
    arg: u32,
    patterns: *mut Patterns,
    recipe_effect: *mut i32,
) -> i32 {
    crate::persistence::project_call();
    if fields.is_null() || patterns.is_null() || recipe_effect.is_null() {
        return -1;
    }
    // SAFETY: lifetime/disjoint/exclusive field contract stated above.
    let f = unsafe { &*fields };
    if f.runtime.is_null()
        || f.image.is_null()
        || f.saved.is_null()
        || f.pending.is_null()
        || f.pending_globals.is_null()
        || f.globals.is_null()
        || f.from_old.is_null()
        || f.extras.is_null()
        || f.live_extras.is_null()
        || f.pending_extras.is_null()
        || f.bass_dirty.is_null()
    {
        return -1;
    }
    let mut p = unsafe {
        Project {
            recipe_effect: None,
            runtime: &mut *f.runtime,
            image: &mut *f.image,
            saved: &mut *f.saved,
            pending: &mut *f.pending,
            pending_globals: &mut *f.pending_globals,
            globals: &mut *f.globals,
            from_old: &mut *f.from_old,
            extras: &mut *f.extras,
            live_extras: &mut *f.live_extras,
            pending_extras: &mut *f.pending_extras,
            bass_dirty: &mut *f.bass_dirty,
        }
    };
    // SAFETY: distinct initialized output scratch, not a volatile engine array.
    let pat = unsafe { &mut *patterns };
    let mut io = Platform(f);
    if operation != 1 && (p.runtime.genre >= 19 || p.runtime.piece >= 10) {
        return -1;
    }
    let result = match operation {
        0 => {
            p.image_load(&mut io);
            0
        }
        1 => i32::from(p.boot(&mut io, arg, pat)),
        2 => {
            p.keep(pat);
            0
        }
        3 => {
            if arg >= 10 {
                return -1;
            }
            let previous = p.runtime.piece;
            p.runtime.piece = arg as u8;
            p.take(pat);
            p.runtime.piece = previous;
            0
        }
        4 => {
            p.save_piece();
            0
        }
        5 => {
            p.factory_piece(pat);
            0
        }
        6 => p.flush(&mut io),
        7 => {
            p.globals_load(&mut io);
            0
        }
        8 => {
            p.globals_sync();
            0
        }
        9 => p.flush_globals(&mut io),
        10 => {
            p.extras_load(&mut io);
            0
        }
        11 => p.extras_save(&mut io),
        12 => {
            p.bass_load(&mut io);
            0
        }
        13 => p.bass_save(&mut io),
        14 => {
            p.place_set(arg);
            0
        }
        15 => {
            p.random_set(arg != 0);
            0
        }
        16 => {
            p.runtime.piece = fm1_engine::project::normalize(arg as i32, 10);
            p.take(pat);
            0
        }
        17 => {
            p.switch_genre(&mut io, arg as i32, pat);
            0
        }
        18 => p.apply_lines() as i32,
        _ => -1,
    };
    // SAFETY: caller provides one distinct writable aligned effect word.
    unsafe { recipe_effect.write(p.recipe_effect.map_or(-1, i32::from)) };
    result
}
/// # Safety
/// list identifies16 initialized readable bytes. No pointer survives the call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_project_list(
    list: *const [u8; 16],
    mode: u32,
    position: i32,
    direction: i32,
    genre: u32,
    piece: u32,
) -> i32 {
    if list.is_null() {
        return -1;
    }
    // SAFETY: caller guarantees the fixed initialized read-only list range.
    let list = unsafe { &*list };
    if mode == 0 {
        fm1_engine::project::list_find(list, position as u8, genre as u8, piece as u8) as i32
    } else {
        fm1_engine::project::list_step(list, position, direction)
    }
}

/// # Safety
/// input length readable bytes; defaults count initialized i16s, when op0.
/// output capacity writable bytes; output is disjoint from both input/defaults.
/// All buffers remain live for this call; no retained pointer or callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_persist_ui(
    op: u32,
    input: *const u8,
    length: u32,
    defaults: *const i16,
    count: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    crate::persistence::project_call();
    if input.is_null() || length > 3840 {
        return -1;
    }
    // SAFETY: checked bounded initialized caller input, read-only this call.
    let raw = unsafe { core::slice::from_raw_parts(input, length as usize) };
    match op {
        0 => {
            if defaults.is_null() || output.is_null() || count > 256 || capacity > 3840 {
                return -1;
            }
            let mut values = [0; 256];
            for (i, v) in values.iter_mut().enumerate().take(count as usize) {
                // SAFETY: count initialized values supplied, no alignment assumption.
                *v = unsafe { defaults.add(i).read_unaligned() } as u8;
            }
            // SAFETY: distinct exclusive initialized output, bounded capacity.
            let dst = unsafe { core::slice::from_raw_parts_mut(output, capacity as usize) };
            if ui::migrate_settings(raw, &values[..count as usize], dst) {
                capacity as i32
            } else {
                -1
            }
        }
        1 => ui::voice_count(raw).map_or(-1, |n| n as i32),
        2 => {
            if output.is_null() || capacity < 20 {
                return -1;
            }
            // Same session-codec witness as the former direct C validation call.
            crate::SESSIONS.fetch_add(1, fm1_rt::atomic::Ordering::Relaxed);
            let bytes = ui::boot_session(raw);
            // SAFETY: initialized local source, distinct checked destination.
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), output, 20) };
            20
        }
        3 => {
            if output.is_null() || capacity < 40 || length != 20 {
                return -1;
            }
            let bytes: &[u8; 20] = raw.try_into().unwrap();
            let values = ui::session_values(bytes);
            for (i, v) in values.iter().enumerate() {
                // SAFETY: checked40 writable bytes; no alignment assumption.
                unsafe { output.add(i * 4).cast::<i32>().write_unaligned(*v) };
            }
            40
        }
        4 => {
            if length == 20 {
                ui::builtin_source(raw.try_into().unwrap()).map_or(-1, i32::from)
            } else {
                -1
            }
        }
        _ => -1,
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_session_save_allowed(flags: u32) -> i32 {
    i32::from(ui::session_save_allowed(
        flags & 1 != 0,
        flags & 2 != 0,
        flags & 4 != 0,
    ))
}
#[no_mangle]
pub extern "C" fn fm1_rust_session_slot(request: i32) -> i32 {
    ui::session_slot(request).map_or(-1, i32::from)
}
/// # Safety
/// bytes covers20 initialized exclusive writable bytes. No pointer retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_session_resolve(bytes: *mut [u8; 20], loaded: u32) -> i32 {
    if bytes.is_null() {
        return -1;
    }
    // SAFETY: caller's fixed initialized exclusive session span.
    ui::resolve_session(unsafe { &mut *bytes }, loaded != 0).map_or(-1, i32::from)
}
/// # Safety
/// input/output are initialized/aligned and disjoint Performance/PerformanceBytes
/// (mode0), or PerformanceBytes/Performance (mode1); the destination is writable.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_performance(mode: u32, input: *const u8, output: *mut u8) -> i32 {
    if input.is_null() || output.is_null() {
        return -1;
    }
    match mode {
        0 => {
            // SAFETY: caller supplies an initialized/aligned12B snapshot and4B output.
            let result = unsafe { input.cast::<ui::Performance>().read() }.encode();
            unsafe { output.cast::<ui::PerformanceBytes>().write(result) };
            0
        }
        1 => {
            // SAFETY: caller supplies an initialized/aligned4B input and12B output.
            let result =
                ui::Performance::decode(unsafe { input.cast::<ui::PerformanceBytes>().read() });
            unsafe { output.cast::<ui::Performance>().write(result) };
            0
        }
        _ => -1,
    }
}
/// # Safety
/// input is a readable initialized/aligned Performance snapshot.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_performance_knob(
    input: *const ui::Performance,
    index: u32,
) -> i32 {
    if input.is_null() {
        return -1;
    }
    // SAFETY: fixed initialized snapshot, no retention.
    unsafe { *input }.knob(index).map_or(-1, i32::from)
}
/// # Safety
/// input is initialized/aligned; clock is two initialized aligned exclusive
/// u32s disjoint from input. No concurrent mutation or pointer retention.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_autosave(
    input: *const ui::Autosave,
    clock: *mut [u32; 2],
) -> u32 {
    if input.is_null() || clock.is_null() {
        return 0;
    }
    // SAFETY: caller's live aligned disjoint state is borrowed only this call.
    ui::autosave(unsafe { *input }, unsafe { &mut *clock })
}
/// # Safety
/// Effects/stored cover count initialized bytes, positions12 initialized bytes.
/// Effects is writable and disjoint from stored/positions. No retained pointer.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_preserve_genre(
    effects: *mut u8,
    stored: *const u8,
    positions: *const [u8; 12],
    count: u32,
    genre: u32,
    have: u32,
) -> i32 {
    if effects.is_null() || stored.is_null() || positions.is_null() || count > 256 {
        return -1;
    }
    // SAFETY: exclusive effect span and distinct read-only spans, bounded count.
    let ok = ui::preserve_genre(
        unsafe { core::slice::from_raw_parts_mut(effects, count as usize) },
        unsafe { core::slice::from_raw_parts(stored, count as usize) },
        unsafe { &*positions },
        genre as u8,
        have != 0,
    );
    i32::from(ok)
}
/// # Safety
/// saved/live/defaults identify16/8/7 initialized bytes; output7 writable bytes,
/// distinct from inputs. No retained pointer or callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_piece_effects(
    saved: *const [u8; 16],
    live: *const [u8; 8],
    defaults: *const [u8; 7],
    output: *mut [u8; 7],
) {
    if saved.is_null() || live.is_null() || defaults.is_null() || output.is_null() {
        return;
    }
    // SAFETY: fixed disjoint initialized buffers guaranteed by the caller.
    let result = ui::piece_effects(unsafe { &*saved }, unsafe { &*live }, unsafe { &*defaults });
    unsafe { output.write(result) };
}
