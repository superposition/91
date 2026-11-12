// SPDX-License-Identifier: GPL-3.0-only
//! The only unsafe layer: serialized C application calls and leaf callbacks.
//! Callbacks borrow only thread-local platform/sinks; no borrow crosses a call
//! into C. Production app/engine state stays C-owned until its tickets land.
use fm1_sim::platform::{Platform, STORE_MAX};
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Write},
};
thread_local! {
    pub static PLATFORM:RefCell<Platform>=RefCell::new(Platform::default());
    pub static MIDI_LOG:RefCell<Option<BufWriter<File>>>=const{RefCell::new(None)};
}
pub fn platform<T>(f: impl FnOnce(&mut Platform) -> T) -> T {
    PLATFORM.with(|p| f(&mut p.borrow_mut()))
}
pub fn midi_file(file: Option<File>) {
    MIDI_LOG.with(|f| *f.borrow_mut() = file.map(BufWriter::new));
}
macro_rules! getter {
    ($name:ident,$field:ident) => {
        #[no_mangle]
        pub extern "C" fn $name() -> u32 {
            platform(|p| p.$field)
        }
    };
}
getter!(plat_ms, now);
getter!(plat_buttons, buttons);
getter!(plat_keys, keys);
getter!(plat_master, master);
getter!(plat_seed, seed);
getter!(plat_battery, battery);
#[no_mangle]
pub extern "C" fn plat_enc(role: i32) -> i32 {
    platform(|p| core::mem::take(&mut p.enc[role as usize]))
}
#[no_mangle]
pub extern "C" fn plat_usb_power() -> i32 {
    platform(|p| p.usb)
}
#[no_mangle]
pub extern "C" fn plat_keys_dim(keys: u32) {
    platform(|p| p.keys_dim = keys);
}
#[no_mangle]
pub extern "C" fn plat_leds(buttons: u32, keys: u32) {
    platform(|p| p.leds(buttons, keys));
}
#[no_mangle]
pub extern "C" fn plat_glow(on: i32) {
    platform(|p| p.glow = on);
}
#[no_mangle]
pub extern "C" fn plat_play_red(_red: i32) {}
#[no_mangle]
pub unsafe extern "C" fn plat_midi_in(packet: *mut u32) -> i32 {
    if let Some(p) = platform(|p| p.dequeue()) {
        unsafe { packet.write(p) };
        1
    } else {
        0
    }
}
#[no_mangle]
pub extern "C" fn plat_midi_out(packet: u32) {
    let (log, now) = platform(|p| (p.midi_out(packet), p.now));
    if log {
        MIDI_LOG.with(|f| {
            if let Some(f) = f.borrow_mut().as_mut() {
                let _ = writeln!(
                    f,
                    "{} ms: {:02X} {:02X} {:02X}",
                    now,
                    (packet >> 8) & 255,
                    (packet >> 16) & 255,
                    packet >> 24
                );
            }
        });
    }
}
#[no_mangle]
pub extern "C" fn en_midi_out(status: u32, a: u32, b: u32) {
    plat_midi_out(status >> 4 | status << 8 | a << 16 | b << 24);
}
#[no_mangle]
pub unsafe extern "C" fn plat_store_load(object: u32, dst: *mut u8, max: u32) -> i32 {
    if object >= 31 {
        return -1;
    }
    let len = platform(|p| p.store.lengths[object as usize]);
    if len < 0 {
        return -1;
    }
    let n = (len as u32).min(max);
    if n == 0 {
        return 0;
    }
    // C promises writable actual-copy bytes, disjoint from the host store.
    platform(|p| {
        p.store.load(object, unsafe {
            core::slice::from_raw_parts_mut(dst, n as usize)
        })
    })
}
unsafe fn store(object: u32, src: *const u8, n: u32, restore: bool) -> i32 {
    if object >= 31 || n as usize > STORE_MAX {
        return -1;
    }
    if !restore && platform(|p| p.store.locked) {
        return -8;
    }
    if n == 0 {
        return platform(|p| p.store.save(object, &[], restore));
    }
    // Synchronous read/copy only: never re-enter app/project state here.
    platform(|p| {
        p.store.save(
            object,
            unsafe { core::slice::from_raw_parts(src, n as usize) },
            restore,
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn plat_store_save(object: u32, src: *const u8, n: u32) -> i32 {
    unsafe { store(object, src, n, false) }
}
#[no_mangle]
pub unsafe extern "C" fn plat_store_restore(object: u32, src: *const u8, n: u32) -> i32 {
    unsafe { store(object, src, n, true) }
}
#[no_mangle]
pub extern "C" fn plat_store_locked() -> i32 {
    platform(|p| p.store.locked as i32)
}
#[no_mangle]
pub extern "C" fn plat_store_seamless() -> i32 {
    platform(|p| p.seamless)
}
#[no_mangle]
pub extern "C" fn plat_usb_host(on: i32) {
    platform(|p| {
        if on != p.host_want {
            p.host_message = if on != 0 { 1 } else { 2 };
        }
        p.host_want = on;
    });
}
#[no_mangle]
pub unsafe extern "C" fn plat_usb_host_diag(b: *mut u8) {
    unsafe { b.write(0) };
}
#[no_mangle]
pub extern "C" fn plat_usb_host_state() -> i32 {
    platform(|p| p.host_want)
}
#[no_mangle]
pub extern "C" fn plat_usb_host_msg() -> i32 {
    platform(|p| core::mem::take(&mut p.host_message))
}
macro_rules! zero {($($name:ident),*)=>{$(#[no_mangle]pub extern "C" fn $name()->u32{0})*};}
// Wall-clock utilization is deliberately excluded from the virtual platform.
zero!(
    plat_cpu_pct,
    plat_xruns,
    plat_late,
    plat_peak_pct,
    plat_cpu1_pct,
    plat_core2
);
#[no_mangle]
pub extern "C" fn lcd_sync() {}
#[no_mangle]
pub extern "C" fn lcd_fill(x: u32, y: u32, w: u32, h: u32, color: u16) {
    platform(|p| p.fill(x, y, w, h, color));
}
#[no_mangle]
pub unsafe extern "C" fn lcd_blit(x: u32, y: u32, w: u32, h: u32, pixels: *const u16) {
    platform(|p| {
        p.blit(x, y, w, h, unsafe {
            core::slice::from_raw_parts(pixels, w as usize * h as usize)
        })
    });
}
#[no_mangle]
pub extern "C" fn fm1_sim_host_value(id: u32, index: u32) -> u32 {
    platform(|p| p.value(id, index))
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Row {
    pub beat: f32,
    pub wall: u32,
    pub gate: u32,
    pub done: u32,
}
#[repr(C)]
#[derive(Default)]
pub struct Runner {
    pub on: u32,
    pub over: u32,
    pub lane: u32,
    pub beat: f32,
    pub rows: [Row; 16],
}
extern "C" {
    pub fn fm1_sim_boot(seed: u32, legacy: u32);
    pub fn fm1_sim_reset_project();
    pub fn fm1_sim_service();
    pub fn fm1_sim_splash(ms: u32);
    pub fn fm1_sim_accel(off: u32);
    pub fn fm1_sim_restore_detach();
    pub fn fm1_sim_observe(name: *const u8, value: *mut i32) -> i32;
    pub fn fm1_sim_layout(out: *mut u32);
    pub fn fm1_sim_recipe(piece: i32, params: *mut i16, pool: *mut u16, pats: *mut u16);
    pub fn fm1_sim_runner(out: *mut Runner);
    pub fn ui_frame();
    pub fn ui_input_only();
    pub fn ui_param_set(param: i32, value: i32);
    pub fn ui_restore_valid(object: u32, src: *const u8, n: u32) -> i32;
    pub fn ui_restore_start(object: u32, src: *const u8, n: u32);
    pub fn en_render(out: *mut i32, n: u32, master: u32);
    pub fn en_jam_set(param: i32, value: i32);
    pub fn en_play(on: i32);
    pub fn en_jam_upload(src: *const u8, n: u32) -> i32;
}
