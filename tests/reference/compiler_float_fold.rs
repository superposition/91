// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
#[no_mangle]
pub extern "C" fn folded_bits() -> u32 {
    (-6.28318530717959_f32 * 300.0_f32).to_bits()
}
#[no_mangle]
pub extern "C" fn runtime_bits(x: f32) -> u32 {
    (-x * 300.0_f32).to_bits()
}
