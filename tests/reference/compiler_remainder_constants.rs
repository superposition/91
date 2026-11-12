// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
const fn r32(a: f32, b: f32) -> f32 {
    a % b
}
const fn r64(a: f64, b: f64) -> f64 {
    a % b
}
const A32: f32 = r32(5.5, 2.0);
const B32: f32 = r32(-5.5, 2.0);
const C32: f32 = r32(5.5, -2.0);
const D32: f32 = r32(-5.5, -2.0);
const E32: f32 = r32(0.0, 2.0);
const F32: f32 = r32(-0.0, 2.0);
const A64: f64 = r64(5.5, 2.0);
const B64: f64 = r64(-5.5, 2.0);
const C64: f64 = r64(5.5, -2.0);
const D64: f64 = r64(-5.5, -2.0);
const E64: f64 = r64(0.0, 2.0);
const F64: f64 = r64(-0.0, 2.0);
#[no_mangle]
pub extern "C" fn const_rem32(i: u32) -> u32 {
    match i {
        0 => A32,
        1 => B32,
        2 => C32,
        3 => D32,
        4 => E32,
        _ => F32,
    }
    .to_bits()
}
#[no_mangle]
pub extern "C" fn const_rem64(i: u32) -> u64 {
    match i {
        0 => A64,
        1 => B64,
        2 => C64,
        3 => D64,
        4 => E64,
        _ => F64,
    }
    .to_bits()
}
