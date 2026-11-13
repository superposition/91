// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
#[cfg(feature = "fragment")]
macro_rules! apply {
    ($e:expr) => { $e };
}
#[cfg(not(feature = "fragment"))]
macro_rules! apply {
    ($($e:tt)+) => { $($e)+ };
}
#[no_mangle]
pub extern "C" fn fm1_macro_shift(mut mask: u8, shift: u32) -> u8 {
    apply!(mask ^= 1u8 << shift);
    mask
}
