// SPDX-License-Identifier: GPL-3.0-only
//! Serialized browser host. No state borrow crosses an application callback.
use core::cell::UnsafeCell;
use fm1_app::host_platform::{
    self as model, Controls, Schedule, StoreMeta, OBJECTS, PIXELS, STORE_MAX,
};

struct Cell<T>(UnsafeCell<T>);
// SAFETY: these private cells have one AudioWorklet owner. Every foreign entry
// requires serialized calls; borrowed views are used only between those calls.
unsafe impl<T> Sync for Cell<T> {}
struct Host {
    controls: Controls,
    store: StoreMeta,
    schedule: Schedule,
    cpu: u32,
}
static HOST: Cell<Host> = Cell(UnsafeCell::new(Host {
    controls: Controls::new(),
    store: StoreMeta::new(),
    schedule: Schedule::new(),
    cpu: 0,
}));
// Persistent foreign pointers derive from separate allocation roots, never a
// temporary reference. Metadata borrows cannot retag their payload allocations.
static FRAME: Cell<[u16; PIXELS]> = Cell(UnsafeCell::new([0; PIXELS]));
static STORE: Cell<[[u8; STORE_MAX]; OBJECTS]> = Cell(UnsafeCell::new([[0; STORE_MAX]; OBJECTS]));
static BLOCK: Cell<[i32; 512]> = Cell(UnsafeCell::new([0; 512]));
#[cfg(test)]
pub(crate) static TEST_OWNER: std::sync::Mutex<()> = std::sync::Mutex::new(());
unsafe fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    // SAFETY: caller serializes host access and f never calls foreign code.
    f(unsafe { &mut *HOST.0.get() })
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Callbacks {
    boot: unsafe extern "C" fn(u32, u32),
    service: unsafe extern "C" fn(),
    render: unsafe extern "C" fn(*mut i32, u32, u32),
    frame: unsafe extern "C" fn(),
    input: unsafe extern "C" fn(),
    nanoseconds: unsafe extern "C" fn() -> f64,
}
/// # Safety
/// Calls are serialized on the browser owner; no reentry into a render/boot.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_get(id: u32) -> u32 {
    unsafe {
        with_host(|h| match id {
            0 => h.controls.now,
            1 => h.controls.buttons,
            2 => h.controls.keys,
            3 => h.controls.master,
            4 => h.controls.battery,
            5 => h.controls.usb as u32,
            6 => h.controls.lit_buttons,
            7 => h.controls.lit_keys,
            8 => h.controls.blits,
            9 => h.controls.seed,
            10 => h.cpu,
            11 => h.controls.seamless as u32,
            12 => u32::from(h.store.locked),
            13 => h.controls.host_want as u32,
            14 => core::mem::take(&mut h.controls.host_message) as u32,
            _ => 0,
        })
    }
}
/// # Safety
/// Same serialized owner contract as get. No foreign callback occurs here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_set(id: u32, first: u32, second: u32) {
    unsafe {
        with_host(|h| match id {
            0 => h.controls.seed = first,
            1 => h.controls.buttons = first,
            2 => h.controls.keys = first,
            3 => h.controls.master = first.min(4096),
            4 => {
                let _ = h.controls.add_encoder(first as i32, second as i32);
            }
            5 => h.controls.keys_dim = first,
            6 => h.controls.leds(first, second),
            7 => h.controls.glow = first as i32,
            8 => {
                let on = first as i32;
                if on != h.controls.host_want {
                    h.controls.host_message = if on != 0 { 1 } else { 2 };
                }
                h.controls.host_want = on;
            }
            _ => {}
        })
    }
}
/// # Safety
/// Serialized owner; invalid roles leave all accumulators unchanged.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_encoder(role: i32) -> i32 {
    unsafe { with_host(|h| h.controls.take_encoder(role)) }
}
/// # Safety
/// Foreign framebuffer views are accessed only between serialized host calls.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_frame() -> *mut u16 {
    FRAME.0.get().cast()
}
/// # Safety
/// Serialized owner; no retained foreign view is concurrently accessed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_fill(x: u32, y: u32, w: u32, h: u32, color: u16) {
    model::fill(unsafe { &mut *FRAME.0.get() }, x, y, w, h, color);
}
/// # Safety
/// Source identifies w*h initialized u16 values disjoint from the framebuffer.
/// Calls serialize and the source stays immutable until this call returns.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_blit(x: u32, y: u32, w: u32, h: u32, source: *const u16) {
    unsafe { with_host(|h| h.controls.blits = h.controls.blits.wrapping_add(1)) };
    let Some(n) = (w as usize).checked_mul(h as usize) else {
        return;
    };
    if n > PIXELS || (n != 0 && source.is_null()) {
        return;
    }
    let pixels = if n == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(source, n) }
    };
    let _ = model::blit(unsafe { &mut *FRAME.0.get() }, x, y, w, h, pixels);
}
/// # Safety
/// Serialized owner; callback capture only, with no foreign calls inside it.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_host_value(id: u32, index: u32) -> u32 {
    unsafe {
        with_host(|h| match id {
            0 => h.store.writes,
            1 => h.controls.clocks,
            2 => h.controls.midi_count,
            3 => h.controls.host_want as u32,
            4 => h.controls.restore_valid as u32,
            5 => u32::from(h.store.locked),
            6 => h.controls.cc[index as usize & 127],
            7 => h.controls.last_velocity as u32,
            8 => h.controls.last_out,
            9 => h.controls.lit_buttons,
            10 => h.controls.dim_keys,
            11 => h.controls.lit_keys,
            _ => 0,
        })
    }
}
/// # Safety
/// Serialized owner; packet supplies one writable disjoint u32 if a MIDI event exists.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_midi_in(packet: *mut u32) -> i32 {
    let value = unsafe { with_host(|h| h.controls.dequeue()) };
    if let Some(value) = value {
        unsafe { packet.write(value) };
        1
    } else {
        0
    }
}
/// # Safety
/// Serialized owner; no foreign callback or borrowed pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_midi_out(packet: u32) {
    let _ = unsafe { with_host(|h| h.controls.midi_out(packet)) };
}
/// # Safety
/// The returned raw view is used only between serialized calls. No reference is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_ptr(object: u32) -> *mut u8 {
    if object as usize >= OBJECTS {
        core::ptr::null_mut()
    } else {
        unsafe { STORE.0.get().cast::<u8>().add(object as usize * STORE_MAX) }
    }
}
/// # Safety
/// Serialized owner.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_len(object: u32) -> i32 {
    unsafe { with_host(|h| h.store.lengths.get(object as usize).copied().unwrap_or(-1)) }
}
/// # Safety
/// Serialized owner; foreign writes to object payload finish before publishing its length.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_set_len(object: u32, n: i32) {
    let _ = unsafe { with_host(|h| h.store.set_length(object, n)) };
}
/// # Safety
/// dst supplies the returned number of writable bytes disjoint from the store.
/// Output may be uninitialized. No payload or metadata access is concurrent.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_load(object: u32, dst: *mut u8, max: u32) -> i32 {
    let n = match unsafe { with_host(|h| h.store.load_length(object, max as usize)) } {
        Ok(n) => n,
        Err(error) => return error,
    };
    if n != 0 {
        let source = unsafe { STORE.0.get().cast::<u8>().add(object as usize * STORE_MAX) };
        unsafe { core::ptr::copy_nonoverlapping(source, dst, n) };
    }
    n as i32
}
/// # Safety
/// src supplies n initialized bytes disjoint from the store; calls serialize.
/// access selects legacy diagnostic accounting, not a separate admission policy.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_save(
    object: u32,
    src: *const u8,
    n: u32,
    restore: u32,
    access: u32,
) -> i32 {
    let admission = unsafe {
        with_host(|h| {
            let result = h.store.writable(object, n as usize, restore != 0);
            if result != Err(-1) && access != 0 {
                let _ = crate::persistence::fm1_rust_store_access(
                    1,
                    u32::from(h.store.locked),
                    u32::from(restore != 0),
                );
            }
            result
        })
    };
    if let Err(error) = admission {
        return error;
    }
    if n != 0 {
        let destination = unsafe { STORE.0.get().cast::<u8>().add(object as usize * STORE_MAX) };
        unsafe { core::ptr::copy_nonoverlapping(src, destination, n as usize) };
    }
    unsafe { with_host(|h| h.store.committed(object, n as usize, restore != 0)) };
    0
}
/// # Safety
/// Serialized owner.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_store_writes() -> u32 {
    unsafe { with_host(|h| h.store.writes) }
}
/// # Safety
/// cb is a live immutable callback table. App callbacks may reenter leaf host
/// operations, but never boot/advance; no host borrow survives a callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_boot(cb: *const Callbacks) {
    let cb = unsafe { cb.read() };
    let seed = unsafe { with_host(|h| h.controls.seed) };
    unsafe { (cb.boot)(seed, 0) };
}
/// # Safety
/// Same callback/serialized owner contract as boot. Render writes exactly512
/// initialized output words, does not retain BLOCK and does not reenter advance.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_web_advance(ms: u32, cb: *const Callbacks) {
    let cb = unsafe { cb.read() };
    for _ in 0..ms {
        let now = unsafe {
            with_host(|h| {
                h.controls.now = h.controls.now.wrapping_add(1);
                h.controls.now
            })
        };
        unsafe { (cb.service)() };
        while unsafe { with_host(|h| h.schedule.audio_ready(now)) } {
            let master = unsafe { with_host(|h| h.controls.master) };
            let start = unsafe { (cb.nanoseconds)() };
            let block = BLOCK.0.get().cast::<i32>();
            unsafe { (cb.render)(block, 256, master) };
            let _ = unsafe { crate::web_audio::fm1_rust_web_push(block, 256) };
            let elapsed = unsafe { (cb.nanoseconds)() } - start;
            unsafe {
                with_host(|h| {
                    h.cpu = (100.0 * elapsed / (256.0 * 1e9 / 44100.0)) as u32;
                    h.schedule.rendered();
                })
            };
        }
        if unsafe { with_host(|h| h.schedule.frame_ready(now)) } {
            unsafe { (cb.frame)() };
        } else {
            unsafe { (cb.input)() };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        mem::MaybeUninit,
        sync::atomic::{AtomicU32, Ordering},
    };
    static RENDERS: AtomicU32 = AtomicU32::new(0);
    static FRAMES: AtomicU32 = AtomicU32::new(0);
    static INPUTS: AtomicU32 = AtomicU32::new(0);
    static NANOS: AtomicU32 = AtomicU32::new(0);
    unsafe extern "C" fn boot(seed: u32, legacy: u32) {
        assert_eq!((seed, legacy), (91, 0));
        unsafe { fm1_rust_web_set(1, 7, 0) };
    }
    unsafe extern "C" fn service() {
        assert!(unsafe { fm1_rust_web_get(0) } > 0);
    }
    unsafe extern "C" fn render(out: *mut i32, n: u32, master: u32) {
        assert_eq!((n, master), (256, 2048));
        RENDERS.fetch_add(1, Ordering::Relaxed);
        // Reenter leaf metadata while the caller lends only a raw output root.
        assert_eq!(unsafe { fm1_rust_web_get(1) }, 7);
        for i in 0..n * 2 {
            unsafe { out.add(i as usize).write(i as i32 * 123) };
        }
    }
    unsafe extern "C" fn frame() {
        FRAMES.fetch_add(1, Ordering::Relaxed);
        unsafe {
            fm1_rust_web_fill(0, 0, 1, 1, 0x1234);
            fm1_rust_web_set(6, 3, 5);
        }
    }
    unsafe extern "C" fn input() {
        INPUTS.fetch_add(1, Ordering::Relaxed);
        assert_eq!(unsafe { fm1_rust_web_get(3) }, 2048);
    }
    unsafe extern "C" fn nanoseconds() -> f64 {
        NANOS.fetch_add(60000, Ordering::Relaxed) as f64
    }
    #[test]
    fn retained_host_roots_and_uninitialized_outputs_survive_reentrant_app_callbacks() {
        let _owner = TEST_OWNER.lock().unwrap();
        // SAFETY: the test is the exclusive serialized owner. No prior host
        // view is retained here; all mock callbacks use disjoint leaf state.
        unsafe {
            *HOST.0.get() = Host {
                controls: Controls::new(),
                store: StoreMeta::new(),
                schedule: Schedule::new(),
                cpu: 0,
            };
            let pixels = fm1_rust_web_frame();
            let object = fm1_rust_web_store_ptr(4);
            let first = [1u8, 2, 3, 4];
            core::ptr::copy_nonoverlapping(first.as_ptr(), object, 4);
            fm1_rust_web_store_set_len(4, 4);
            let mut destination = MaybeUninit::<[u8; 4]>::uninit();
            assert_eq!(
                fm1_rust_web_store_load(4, destination.as_mut_ptr().cast(), 4),
                4
            );
            assert_eq!(destination.assume_init(), first);
            let second = [5u8, 6, 7, 8];
            assert_eq!(fm1_rust_web_store_save(4, second.as_ptr(), 4, 0, 0), 0);
            assert_eq!(core::slice::from_raw_parts(object, 4), second);
            assert_eq!(fm1_rust_web_store_load(0, core::ptr::null_mut(), 4), -1);
            assert_eq!(fm1_rust_web_store_load(30, core::ptr::null_mut(), 4), 0);
            assert!(fm1_rust_web_store_ptr(31).is_null());
            fm1_rust_web_set(4, 0, i32::MAX as u32);
            fm1_rust_web_set(4, 0, 1);
            assert_eq!(fm1_rust_web_encoder(0), i32::MAX);
            fm1_rust_web_set(0, 91, 0);
            let callbacks = Callbacks {
                boot,
                service,
                render,
                frame,
                input,
                nanoseconds,
            };
            fm1_rust_web_boot(&callbacks);
            fm1_rust_web_advance(20, &callbacks);
            assert_eq!(
                (
                    RENDERS.load(Ordering::Relaxed),
                    FRAMES.load(Ordering::Relaxed),
                    INPUTS.load(Ordering::Relaxed)
                ),
                (4, 2, 18)
            );
            assert_eq!(fm1_rust_web_get(0), 20);
            assert_eq!(fm1_rust_web_get(10), 1);
            assert_eq!(pixels.read(), 0x3412);
            assert_eq!(core::slice::from_raw_parts(object, 4), second);
            assert_eq!(fm1_rust_web_store_writes(), 1);
        }
    }
}
