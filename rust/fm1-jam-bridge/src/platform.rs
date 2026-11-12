// SPDX-License-Identifier: GPL-3.0-only
//! Stateless early platform bindings. These functions are usable before BSS/data
//! setup: no global counters, allocation, engine callbacks or initialized statics.
use fm1_hal::{
    boot, drivers,
    io::{Reg16, Reg32, Reg8, RegisterIo},
};
/// Pure decision over a copied CPU0-owned word; no atomics, waits or callbacks.
#[no_mangle]
pub extern "C" fn fm1_rust_hal_flash(operation: u32, state: u32, unknown: u32) -> u32 {
    use fm1_hal::flash;
    match operation {
        0 => flash::enter(state, unknown != 0),
        1 => flash::leave(state),
        2 => flash::render(state, true),
        3 => flash::render(state, false),
        4 => u32::from(flash::window(state, unknown != 0)),
        5 => u32::from(flash::serial(state)),
        _ => 0,
    }
}
pub(crate) struct Mmio;
#[cfg(feature = "hal-test")]
extern "C" {
    fn fm1_hal_test_mmio(kind: u32, address: u32, value: u32) -> u32;
}
impl RegisterIo for Mmio {
    fn read8(&mut self, r: Reg8) -> u8 {
        #[cfg(feature = "hal-test")]
        {
            unsafe { fm1_hal_test_mmio(8, r.address(), 0) as u8 }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::read_volatile(r.address() as *const u8) }
        }
    }
    fn read16(&mut self, r: Reg16) -> u16 {
        #[cfg(feature = "hal-test")]
        {
            unsafe { fm1_hal_test_mmio(16, r.address(), 0) as u16 }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::read_volatile(r.address() as *const u16) }
        }
    }
    fn read32(&mut self, r: Reg32) -> u32 {
        #[cfg(feature = "hal-test")]
        {
            unsafe { fm1_hal_test_mmio(32, r.address(), 0) }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::read_volatile(r.address() as *const u32) }
        }
    }
    fn write8(&mut self, r: Reg8, v: u8) {
        #[cfg(feature = "hal-test")]
        {
            unsafe {
                fm1_hal_test_mmio(136, r.address(), u32::from(v));
            }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::write_volatile(r.address() as *mut u8, v) }
        }
    }
    fn write16(&mut self, r: Reg16, v: u16) {
        #[cfg(feature = "hal-test")]
        {
            unsafe {
                fm1_hal_test_mmio(144, r.address(), u32::from(v));
            }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::write_volatile(r.address() as *mut u16, v) }
        }
    }
    fn write32(&mut self, r: Reg32, v: u32) {
        #[cfg(feature = "hal-test")]
        {
            unsafe {
                fm1_hal_test_mmio(160, r.address(), v);
            }
        }
        #[cfg(not(feature = "hal-test"))]
        {
            unsafe { core::ptr::write_volatile(r.address() as *mut u32, v) }
        }
    }
}
/// # Safety
/// Four aligned initialized words are exclusively available through this call.
/// They are a copied noinit record, not a reference to the noinit allocation.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_boot(words: *mut u32, operation: u32, argument: u32) -> u32 {
    let p = unsafe { words.cast::<[u32; 4]>().read() };
    let mut guard = boot::Guard {
        magic: p[0],
        failed: p[1],
        pending: p[2],
    };
    let action = match operation {
        0 => match boot::enter(&mut guard) {
            boot::Action::Continue => 0,
            boot::Action::SafeMode => 1,
            boot::Action::UpdateLoader => 2,
        },
        1 => {
            boot::healthy(&mut guard, argument, false);
            0
        }
        2 => {
            boot::healthy(&mut guard, argument, true);
            0
        }
        3 => {
            boot::intentional_reset(&mut guard, argument != 0);
            0
        }
        _ => return 0,
    };
    unsafe {
        words
            .cast::<[u32; 4]>()
            .write([guard.magic, guard.failed, guard.pending, action]);
    }
    1
}
#[no_mangle]
pub extern "C" fn fm1_rust_hal_readable(address: u32, length: u32) -> u32 {
    u32::from(boot::readable(address, length))
}
/// # Safety
/// Execute only on the FM-1 map with the source-specific IRQ/core exclusion.
/// MMIO is volatile at its declared width. This creates no Rust reference into
/// MMIO/DMA, grants no DMA lifetime/exclusivity and retains no caller address.
/// Audio/LCD buffer address arguments denote live caller-owned RAM for the
/// complete external DMA lifetime, including timeout/quarantine. Init runs with
/// interrupts off; foreground ADC/LCD operations are serialized by their caller.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_hal_io(operation: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
    let mut io = Mmio;
    match operation {
        0 => drivers::time_init(&mut io),
        1 => drivers::timer5_start(&mut io, a, b),
        2 => drivers::audio_init(&mut io, a, b, c, d),
        3 => drivers::adc_init(&mut io),
        4 => return drivers::adc_read(&mut io, a) as u32,
        5 => drivers::lcd_init(&mut io),
        6 => drivers::lcd_baud(&mut io, a),
        7 => drivers::lcd_command(&mut io, a as u8),
        8 => drivers::lcd_data(&mut io, a, b),
        9 => return u32::from(drivers::lcd_ready(&mut io, a != 0)),
        _ => return 0,
    }
    1
}
