// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
#![allow(invalid_nan_comparisons)]
macro_rules! compare {
    ($z:expr,$n:expr,$q:expr,$one:expr) => {{
        let (z, n, q, one) = ($z, $n, $q, $one);
        ((z == n) as u32)
            | (((z != n) as u32) << 1)
            | (((z < n) as u32) << 2)
            | (((z > n) as u32) << 3)
            | (((z <= n) as u32) << 4)
            | (((z >= n) as u32) << 5)
            | (((q == q) as u32) << 6)
            | (((q != q) as u32) << 7)
            | (((q < one) as u32) << 8)
            | (((q > one) as u32) << 9)
            | (((q <= one) as u32) << 10)
            | (((q >= one) as u32) << 11)
            | (((q == one) as u32) << 12)
            | (((q != one) as u32) << 13)
    }};
}
const fn c32(z: f32, n: f32, q: f32) -> u32 {
    compare!(z, n, q, 1.0_f32)
}
const fn c64(z: f64, n: f64, q: f64) -> u32 {
    compare!(z, n, q, 1.0_f64)
}
const CONST32: u32 = c32(0.0, -0.0, f32::NAN);
const CONST64: u32 = c64(0.0, -0.0, f64::NAN);
#[no_mangle]
pub extern "C" fn expression_cmp32() -> u32 {
    compare!(0.0_f32, -0.0_f32, f32::NAN, 1.0_f32)
}
#[no_mangle]
pub extern "C" fn expression_cmp64() -> u32 {
    compare!(0.0_f64, -0.0_f64, f64::NAN, 1.0_f64)
}
#[no_mangle]
pub extern "C" fn constant_cmp32() -> u32 {
    CONST32
}
#[no_mangle]
pub extern "C" fn constant_cmp64() -> u32 {
    CONST64
}
#[no_mangle]
#[inline(never)]
pub extern "C" fn runtime_cmp32(z: f32, n: f32, q: f32) -> u32 {
    c32(z, n, q)
}
#[no_mangle]
#[inline(never)]
pub extern "C" fn runtime_cmp64(z: f64, n: f64, q: f64) -> u32 {
    c64(z, n, q)
}
