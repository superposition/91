// SPDX-License-Identifier: GPL-3.0-only
//! Physical panic boundary, also supplying mrustc's explicit core ABI entries.
pub fn panic_stop() -> ! {
    extern "C" {
        fn fm1_mcu_panic() -> !;
    }
    // SAFETY: the linked HAL silences/holds the appropriate core and never
    // returns into the interrupted Rust operation. It retains no Rust pointer.
    unsafe { fm1_mcu_panic() }
}
#[no_mangle]
// Match C's void prototypes. The bodies never return; mrustc represents a
// Rust `!` return as char in generated C, which conflicts with libc's abort.
pub extern "C" fn abort() {
    panic_stop()
}
#[no_mangle]
pub extern "C" fn panic_impl(_info: *const core::ffi::c_void) -> ! {
    panic_stop()
}
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() {
    panic_stop()
}
