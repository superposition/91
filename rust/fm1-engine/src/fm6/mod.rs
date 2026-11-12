// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Kerem Kilic (Ellic Studio)
// Portions Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
//! Six fixed FM voices; 64-frame operator blocks over caller-owned storage.
pub mod envelopes;
pub mod frames;
pub mod lfo;
pub mod operators;
pub mod plan;
pub mod pool;
pub mod program;
pub mod scaling;
#[rustfmt::skip]
pub mod tables;
pub mod types;
pub mod voice;

pub const BLOCK: usize = 64;
pub const VOICES: usize = 6;
pub const PATCH: usize = 155;
pub const MODERN: u32 = 0;
pub const MARK1: u32 = 1;
pub const OPL: u32 = 2;
pub const SILENT: i32 = 1638400;
pub const R1: usize = 0;
pub const L1: usize = 4;
pub const BP: usize = 8;
pub const LD: usize = 9;
pub const RD: usize = 10;
pub const LC: usize = 11;
pub const RC: usize = 12;
pub const RS: usize = 13;
pub const AMS: usize = 14;
pub const KVS: usize = 15;
pub const OL: usize = 16;
pub const MODE: usize = 17;
pub const FC: usize = 18;
pub const FF: usize = 19;
pub const DET: usize = 20;
pub const OP: usize = 21;
pub const PR1: usize = 126;
pub const PL1: usize = 130;
pub const ALG: usize = 134;
pub const FB: usize = 135;
pub const OKS: usize = 136;
pub const LFS: usize = 137;
pub const LFD: usize = 138;
pub const LPMD: usize = 139;
pub const LAMD: usize = 140;
pub const LKS: usize = 141;
pub const LFW: usize = 142;
pub const LPMS: usize = 143;
pub const TRNSP: usize = 144;
pub const NAME: usize = 145;

/// RAM lookup regions are initialized once before the worker can read them.
/// They keep their existing allocation/placement; no large temporary is made.
pub fn initialize_tables(
    sin: &mut [f32; 1025],
    mki_log: &mut [u16; 2048],
    mki_exp: &mut [u16; 1024],
    opl_log: &mut [u16; 512],
    opl_exp: &mut [u16; 256],
) {
    let mut u: i32 = 1 << 30;
    let mut v: i32 = 0;
    for i in 0..512 {
        let x = (v + 32) >> 6;
        sin[i] = x as f32 * (1.0 / 16777216.0);
        sin[i + 512] = (-x) as f32 * (1.0 / 16777216.0);
        let x = ((u as i64 * 1073721611 - v as i64 * 6588356 + (1 << 29)) >> 30) as i32;
        v = ((u as i64 * 6588356 + v as i64 * 1073721611 + (1 << 29)) >> 30) as i32;
        u = x;
    }
    sin[1024] = 0.0;
    for i in 0..1024 {
        mki_log[i] = tables::FM6_MKI_LOG_H[i];
        mki_log[2047 - i] = tables::FM6_MKI_LOG_H[i];
    }
    for i in 0..256 {
        opl_log[i] = tables::FM6_OPL_LOG_H[i];
        opl_log[511 - i] = tables::FM6_OPL_LOG_H[i];
    }
    let mut e = 0u32;
    for i in 0..1024 {
        if i != 0 {
            e += 2 + ((tables::FM6_MKI_EXP_D[(i - 1) >> 1] as u32 >> (((i - 1) & 1) * 4)) & 15);
        }
        mki_exp[i ^ 1023] = (4096 + e) as u16;
    }
    e = 0;
    for i in 0..256 {
        if i != 0 {
            e += 2 + ((tables::FM6_OPL_EXP_D[(i - 1) >> 1] as u32 >> (((i - 1) & 1) * 4)) & 15);
        }
        opl_exp[i ^ 255] = ((0x400 + e) << 1) as u16;
    }
}

pub struct Lookup<'a> {
    pub sin: &'a [f32; 1025],
    pub mki_log: &'a [u16; 2048],
    pub mki_exp: &'a [u16; 1024],
    pub opl_log: &'a [u16; 512],
    pub opl_exp: &'a [u16; 256],
}
pub struct Scratch<'a> {
    pub bus: &'a mut [[f32; BLOCK]; 2],
    pub sum: &'a mut [f32; BLOCK],
    pub ibus: &'a mut [[i32; BLOCK]; 2],
    pub isum: &'a mut [i32; BLOCK],
}
