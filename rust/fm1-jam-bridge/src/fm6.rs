// SPDX-License-Identifier: GPL-3.0-only
//! FM block-owner adapters. Buffers/state remain caller-owned; no pointer or
//! callback is retained across the reviewed render/worker ownership boundary.
use fm1_engine::fm6::frames;
use fm1_engine::fm6::{self, pool, program, types::*, voice};
use fm1_rt::atomic::{AtomicU32, Ordering};

/// # Safety
/// Selection/output are initialized, aligned, exclusive and disjoint. Output
/// has 156 bytes; only the selected user row is read, and only when selected.
/// Its 128 bytes are stable after the preceding worker joins. Patch/name/table
/// borrows are transient or immutable statics; no user pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_program(
    s: *mut program::Selection,
    users: *const u8,
    output: *mut u8,
) -> u32 {
    let s = unsafe { &mut *s };
    if !program::select(s) {
        return 0;
    }
    let user = if s.current >= 30 {
        Some(unsafe {
            &*users
                .add((s.current as usize - 30) * 128)
                .cast::<[u8; 128]>()
        })
    } else {
        None
    };
    program::load(s.current as usize, s.dy_current != 0, user, unsafe {
        &mut *output.cast::<[u8; 155]>()
    });
    unsafe {
        *output.add(155) = 0;
    }
    COUNTERS[4].fetch_add(1, Ordering::Relaxed);
    1
}
/// # Safety
/// Packed 128 bytes/config are readable and output 156 bytes is writable,
/// initialized/disjoint, held exclusively at the existing profile boundary.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_profile(
    packed: *const u8,
    output: *mut u8,
    config: *const program::Profile,
) {
    program::songbook(
        unsafe { &*packed.cast::<[u8; 128]>() },
        unsafe { &mut *output.cast::<[u8; 155]>() },
        unsafe { &*config },
    );
    unsafe {
        *output.add(155) = 0;
    }
    COUNTERS[4].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// Index is an existing 0..45 public FM program. This returns a scalar only.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_gain(index: u32, dy: u32) -> f32 {
    program::gain(index as usize, dy != 0)
}
/// # Safety
/// Index is an existing 0..29 factory program. The returned 16-byte NUL-ended
/// immutable static row remains valid forever and must never be written.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_name(index: u32, dy: u32) -> *const u8 {
    program::name(index as usize, dy != 0).as_ptr()
}
/// # Safety
/// Scalar state is exclusive. For accepted stage/commit, source/destination
/// provide 128 initialized readable/writable disjoint bytes; commit additionally
/// has a disjoint initialized 16-byte name buffer. Rejected operations and wipe
/// do not dereference any payload pointer. Caller serializes the original
/// staging protocol and refuses access while the staging union is a bass FIFO.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_user(
    mode: u32,
    slot: u32,
    s: *mut program::UserState,
    source: *const u8,
    destination: *mut u8,
    name: *mut u8,
) -> u32 {
    let s = unsafe { &mut *s };
    let accepted = match mode {
        0 => program::stage(s, slot),
        1 => program::commit(s, slot),
        2 => {
            program::wipe(s);
            return 1;
        }
        _ => false,
    };
    if !accepted {
        return 0;
    }
    let destination = unsafe { &mut *destination.cast::<[u8; 128]>() };
    program::copy_packed(unsafe { &*source.cast::<[u8; 128]>() }, destination);
    if mode == 1 {
        program::user_name(destination, unsafe { &mut *name.cast::<[u8; 16]>() });
        COUNTERS[5].fetch_add(1, Ordering::Relaxed);
    }
    1
}
static COUNTERS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.wrapping_sub(48) as usize)
        .map(|v| v.load(Ordering::Relaxed))
        .unwrap_or(0)
}
#[repr(C)]
pub struct Tables {
    sin: *mut f32,
    mki_log: *mut u16,
    mki_exp: *mut u16,
    opl_log: *mut u16,
    opl_exp: *mut u16,
}
#[repr(C)]
pub struct Scratch {
    bus: *mut f32,
    sum: *mut f32,
    ibus: *mut i32,
    isum: *mut i32,
}
#[repr(C)]
pub struct Render {
    slots: *mut Slot,
    keys: *mut Lfo,
    bass: *mut Lfo,
    key_patch: *const u8,
    bass_patch: *const u8,
    context: *const voice::Config,
    cache: *mut voice::AmsCache,
    tables: *const Tables,
    scratch: *const Scratch,
    block: *mut f32,
    position: *mut u32,
    bass_sample: *mut f32,
}
const _: [(); 12 * core::mem::size_of::<*const ()>()] = [(); core::mem::size_of::<Render>()];
const _: [(); 5 * core::mem::size_of::<*const ()>()] = [(); core::mem::size_of::<Tables>()];
const _: [(); 4 * core::mem::size_of::<*const ()>()] = [(); core::mem::size_of::<Scratch>()];
unsafe fn lookup<'a>(t: &Tables) -> fm6::Lookup<'a> {
    fm6::Lookup {
        sin: unsafe { &*t.sin.cast::<[f32; 1025]>() },
        mki_log: unsafe { &*t.mki_log.cast::<[u16; 2048]>() },
        mki_exp: unsafe { &*t.mki_exp.cast::<[u16; 1024]>() },
        opl_log: unsafe { &*t.opl_log.cast::<[u16; 512]>() },
        opl_exp: unsafe { &*t.opl_exp.cast::<[u16; 256]>() },
    }
}
unsafe fn scratch<'a>(p: &Scratch) -> fm6::Scratch<'a> {
    fm6::Scratch {
        bus: unsafe { &mut *p.bus.cast::<[[f32; 64]; 2]>() },
        sum: unsafe { &mut *p.sum.cast::<[f32; 64]>() },
        ibus: unsafe { &mut *p.ibus.cast::<[[i32; 64]; 2]>() },
        isum: unsafe { &mut *p.isum.cast::<[i32; 64]>() },
    }
}
unsafe fn pool<'a>(slots: *mut Slot, keys: *mut Lfo, bass: *mut Lfo) -> pool::Pool<'a> {
    pool::Pool {
        slots: unsafe { &mut *slots.cast::<[Slot; 6]>() },
        keys_lfo: unsafe { &mut *keys },
        bass_lfo: unsafe { &mut *bass },
    }
}
/// # Safety
/// Descriptor and five distinct, initialized/aligned writable table regions
/// provide 1025 f32 /2048,1024,512,256 u16 values. Initialization is exclusive,
/// before any worker/foreground reader; no table borrow escapes this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_init(raw: *const Tables) {
    let t = unsafe { &*raw };
    fm6::initialize_tables(
        unsafe { &mut *t.sin.cast::<[f32; 1025]>() },
        unsafe { &mut *t.mki_log.cast::<[u16; 2048]>() },
        unsafe { &mut *t.mki_exp.cast::<[u16; 1024]>() },
        unsafe { &mut *t.opl_log.cast::<[u16; 512]>() },
        unsafe { &mut *t.opl_exp.cast::<[u16; 256]>() },
    );
}
/// # Safety
/// Six initialized/aligned slots and one LFO are disjoint and exclusively held
/// after the preceding worker has joined. The C caller resets its scalar queue
/// cursor/current-program/age separately, preserving legacy reset sequencing.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_reset(slots: *mut Slot, lfo: *mut Lfo) {
    pool::reset(unsafe { &mut *slots.cast::<[Slot; 6]>() }, unsafe {
        &mut *lfo
    });
    COUNTERS[7].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// Six initialized/aligned slots, two LFOs, context, age and 155 canonical patch
/// bytes are pairwise disjoint. Render exclusively holds mutable regions after
/// worker join. Patch fields and current-program are in the retained ranges.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_note(
    slots: *mut Slot,
    keys: *mut Lfo,
    bass: *mut Lfo,
    patch: *const u8,
    context: *const pool::NoteContext,
    age: *mut u32,
    note: i32,
    velocity: f32,
    gate: i32,
) -> u32 {
    match pool::note_on(
        unsafe { pool(slots, keys, bass) },
        unsafe { &*patch.cast::<[u8; 155]>() },
        unsafe { &*context },
        unsafe { &mut *age },
        note,
        velocity,
        gate,
    ) {
        Some(result) => {
            COUNTERS[1].fetch_add(1, Ordering::Relaxed);
            if result.stolen {
                COUNTERS[2].fetch_add(1, Ordering::Relaxed);
            }
            result.slot as u32
                | if result.stolen { 0x100 } else { 0 }
                | result.previous.map_or(0, |n| {
                    (u32::from(n.owner) << 16) | (u32::from(n.pitch) << 24)
                })
        }
        None => u32::MAX,
    }
}
/// # Safety
/// Six initialized/aligned slots are exclusively held by render after worker
/// join. Frames/tail are the bounded current control interval and release time.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_gates(slots: *mut Slot, frames: u32, tail: i32) {
    let released = pool::gates(unsafe { &mut *slots.cast::<[Slot; 6]>() }, frames, tail);
    COUNTERS[3].fetch_add(released, Ordering::Relaxed);
}
/// # Safety
/// Descriptor/contexts/programs and all fixed table regions are initialized,
/// aligned/readable. Six slots, two LFOs, AMS cache, four scratch regions and 64
/// output f32s are pairwise disjoint and exclusively held by the FM job. Patch
/// buffers supply155 canonical bytes, table initialization is complete, and
/// the foreground does not access pool/scratch/programs until job join. There
/// is no shared FIFO borrow, foreign callback or retained pointer here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_block(
    slots: *mut Slot,
    keys: *mut Lfo,
    bass: *mut Lfo,
    key_patch: *const u8,
    bass_patch: *const u8,
    context: *const voice::Config,
    cache: *mut voice::AmsCache,
    t: *const Tables,
    b: *const Scratch,
    output: *mut f32,
) {
    voice::block(
        unsafe { pool(slots, keys, bass) },
        voice::Programs {
            keys: unsafe { &*key_patch.cast::<[u8; 155]>() },
            bass: unsafe { &*bass_patch.cast::<[u8; 155]>() },
        },
        unsafe { &*context },
        unsafe { &mut *cache },
        &unsafe { lookup(&*t) },
        &mut unsafe { scratch(&*b) },
        unsafe { &mut *output.cast::<[f32; 64]>() },
    );
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
}

/// # Safety
/// The descriptor meets the block ABI's disjoint initialized regions contract.
/// Output supplies exactly n<=256 writable FIFO values, disjoint from every
/// other borrowed region and the foreground's simultaneous read span. Bass
/// supplies a separate n-value span iff Jam; otherwise it is null and the
/// staging/bass union is not borrowed. The C owner proves worker retirement
/// before state/program mutation. No reference escapes this synchronous call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_render(
    raw: *const Render,
    output: *mut f32,
    bass: *mut f32,
    n: u32,
) {
    assert!(n <= 256);
    let r = unsafe { &*raw };
    let config = unsafe { &*r.context };
    let mut runtime = frames::Runtime {
        pool: unsafe { pool(r.slots, r.keys, r.bass) },
        programs: voice::Programs {
            keys: unsafe { &*r.key_patch.cast::<[u8; 155]>() },
            bass: unsafe { &*r.bass_patch.cast::<[u8; 155]>() },
        },
        config,
        cache: unsafe { &mut *r.cache },
        tables: unsafe { lookup(&*r.tables) },
        scratch: unsafe { scratch(&*r.scratch) },
        block: unsafe { &mut *r.block.cast::<[f32; 64]>() },
        position: unsafe { &mut *r.position },
        bass_sample: unsafe { &mut *r.bass_sample },
    };
    let blocks = runtime.render(
        unsafe { core::slice::from_raw_parts_mut(output, n as usize) },
        if config.jam != 0 {
            Some(unsafe { core::slice::from_raw_parts_mut(bass, n as usize) })
        } else {
            None
        },
    );
    COUNTERS[0].fetch_add(blocks, Ordering::Relaxed);
    COUNTERS[6].fetch_add(n, Ordering::Relaxed);
}
/// # Safety
/// Six live slots are exclusively held by render after joining the FM job.
/// Mode 0 retires all;1 retires a part;2 reads part liveness;3 fades older keys.
/// 4 marks held owner (slot | owner<<8);5 releases owner (0 or0x100|pitch);
/// 6 counts held owners of a sounding pitch, for coalesced MIDI note-off.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_pool(slots: *mut Slot, mode: u32, value: u32) -> u32 {
    let slots = unsafe { &mut *slots.cast::<[Slot; 6]>() };
    match mode {
        0 => pool::retire_all(slots),
        1 => pool::retire_part(slots, value as u8),
        2 => return u32::from(pool::part_busy(slots, value as u8)),
        3 => pool::fade_previous_keys(slots, value),
        4 => {
            return u32::from(fm6::held::mark(
                slots,
                (value & 255) as usize,
                (value >> 8) as u8,
            ))
        }
        5 => {
            return fm6::held::release(slots, value as u8)
                .map_or(0, |pitch| 0x100 | u32::from(pitch))
        }
        6 => return fm6::held::count(slots, value as u8),
        _ => panic!("invalid FM pool operation"),
    }
    0
}
/// # Safety
/// Input is only the n<=256 readable, initialized FIFO span selected by C;
/// all four n-value output spans are distinct writable private chunk buffers.
/// They are disjoint from input/context and concurrent worker write spans.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_output(
    input: *const f32,
    outputs: *const *mut f32,
    context: *const frames::OutputConfig,
    n: u32,
) -> f32 {
    assert!(n <= 256);
    let outputs = unsafe { core::slice::from_raw_parts(outputs, 4) };
    frames::output(
        unsafe { core::slice::from_raw_parts(input, n as usize) },
        [
            unsafe { core::slice::from_raw_parts_mut(outputs[0], n as usize) },
            unsafe { core::slice::from_raw_parts_mut(outputs[1], n as usize) },
            unsafe { core::slice::from_raw_parts_mut(outputs[2], n as usize) },
            unsafe { core::slice::from_raw_parts_mut(outputs[3], n as usize) },
        ],
        unsafe { &*context },
    )
}
/// # Safety
/// Input/output supply n<=256 initialized readable/writable values, disjoint
/// from each other and the FM worker's write span in the same shared ring.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_fm6_bass_output(
    input: *const f32,
    output: *mut f32,
    gain: f32,
    n: u32,
) -> f32 {
    assert!(n <= 256);
    frames::bass_output(
        unsafe { core::slice::from_raw_parts(input, n as usize) },
        unsafe { core::slice::from_raw_parts_mut(output, n as usize) },
        gain,
    )
}
