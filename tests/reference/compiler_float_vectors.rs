// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
const NOISE: f32 = -6.28318530717959_f32 * 300.0_f32;
const CHAIN: f32 = (1.0_f32 + 1e-8_f32) - 1.0_f32;
const fn multiply_add(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}
const CALL: f32 = multiply_add(0.7, 0.7, 0.1);
const CONTROL64: f64 = (1.0_f64 + 1e-17_f64) - 1.0_f64;
#[no_mangle]
pub extern "C" fn folded32(case: u32) -> u32 {
    match case {
        0 => (-6.28318530717959_f32 * 300.0_f32).to_bits(),
        1 => (16777217.0_f32 - 16777216.0_f32).to_bits(),
        2 => ((1.0_f32 + 1e-8_f32) - 1.0_f32).to_bits(),
        3 => (0.7_f32 * 0.7_f32 + 0.1_f32).to_bits(),
        4 => (-1.0_f32 / (0.004_f32 * 44100.0_f32)).to_bits(),
        5 => (1.0_f32 - 0.9993_f32).to_bits(),
        6 => (1.00000011920928955078125_f32 * 1.00000011920928955078125_f32
            - 1.0000002384185791015625_f32)
            .to_bits(),
        7 => (-0.0_f32 * 3.0_f32).to_bits(),
        8 => (0.0_f32 * 3.0_f32).to_bits(),
        9 => ((1e-30_f32 * 1e-20_f32) * 1e20_f32).to_bits(),
        10 => ((1e30_f32 * 1e20_f32) / 1e20_f32).to_bits(),
        11 => ((1.0_f32 - 1.0_f32) * -1.0_f32).to_bits(),
        12 => 16777217.0_f32 as u32,
        13 => (1.25_f32 * 2.0_f32 + 0.5_f32).to_bits(),
        14 => NOISE.to_bits(),
        15 => CHAIN.to_bits(),
        16 => CALL.to_bits(),
        17 => (5.5_f32 % 2.0_f32).to_bits(),
        18 => (-5.5_f32 % 2.0_f32).to_bits(),
        19 => (5.5_f32 % -2.0_f32).to_bits(),
        20 => (-5.5_f32 % -2.0_f32).to_bits(),
        21 => (0.0_f32 % 2.0_f32).to_bits(),
        22 => (-0.0_f32 % 2.0_f32).to_bits(),
        _ => 0,
    }
}
#[no_mangle]
#[inline(never)]
pub extern "C" fn runtime32(case: u32, a: f32, b: f32, c: f32) -> u32 {
    match case {
        0 | 14 => (-a * b).to_bits(),
        1 | 5 => (a - b).to_bits(),
        2 | 15 => ((a + b) - c).to_bits(),
        3 | 13 | 16 => (a * b + c).to_bits(),
        4 => (-c / (a * b)).to_bits(),
        6 => (a * b - c).to_bits(),
        7 | 8 => (a * b).to_bits(),
        9 => ((a * b) * c).to_bits(),
        10 => ((a * b) / c).to_bits(),
        11 => ((a - b) * c).to_bits(),
        12 => a as u32,
        17..=22 => (a % b).to_bits(),
        _ => 0,
    }
}
#[no_mangle]
pub extern "C" fn folded64(case: u32) -> u64 {
    match case {
        0 => (-6.28318530717959_f64 * 300.0_f64).to_bits(),
        1 => (1.0000000000000002_f64 - 1.0_f64).to_bits(),
        2 => ((1.0_f64 + 1e-17_f64) - 1.0_f64).to_bits(),
        3 => (1.0000000000000002_f64 * 1.0000000000000002_f64 - 1.0000000000000004_f64).to_bits(),
        4 => (-0.0_f64 * 3.0_f64).to_bits(),
        5 => (1.25_f64 * 2.0_f64 + 0.5_f64).to_bits(),
        6 => (16777217.0_f32 as f64).to_bits(),
        7 => CONTROL64.to_bits(),
        8 => (5.5_f64 % 2.0_f64).to_bits(),
        9 => (-5.5_f64 % 2.0_f64).to_bits(),
        10 => (5.5_f64 % -2.0_f64).to_bits(),
        11 => (-5.5_f64 % -2.0_f64).to_bits(),
        12 => (0.0_f64 % 2.0_f64).to_bits(),
        13 => (-0.0_f64 % 2.0_f64).to_bits(),
        _ => 0,
    }
}
#[no_mangle]
#[inline(never)]
pub extern "C" fn runtime64(case: u32, a: f64, b: f64, c: f64) -> u64 {
    match case {
        0 => (-a * b).to_bits(),
        1 => (a - b).to_bits(),
        2 | 7 => ((a + b) - c).to_bits(),
        3 => (a * b - c).to_bits(),
        4 => (a * b).to_bits(),
        5 => (a * b + c).to_bits(),
        6 => a.to_bits(),
        8..=13 => (a % b).to_bits(),
        _ => 0,
    }
}
