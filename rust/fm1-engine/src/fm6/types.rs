// SPDX-License-Identifier: GPL-3.0-only
//! Numeric C-compatible storage; flags stay bytes, not foreign Rust enums/bools.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Envelope {
    pub level: i32,
    pub target: i32,
    pub inc: i32,
    pub hold: i32,
    pub ix: u8,
    pub rising: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PitchEnvelope {
    pub pl: i32,
    pub pt: i32,
    pub pi: i32,
    pub pix: u8,
    pub prise: u8,
    pub pr: [u8; 4],
    pub pv: [u8; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Voice {
    pub eg: [Envelope; 6],
    pub ph: [u32; 6],
    pub fq: [i32; 6],
    pub freq_log: [i32; 6],
    pub freq_valid: u8,
    pub gout: [f32; 6],
    pub g: [f32; 6],
    pub dg: [f32; 6],
    pub base: [i32; 6],
    pub porta: [i32; 6],
    pub ol: [i16; 6],
    pub rs: [i8; 6],
    pub plan: [u8; 6],
    pub fb: [f32; 2],
    pub pe: PitchEnvelope,
    pub down: u8,
    pub sub: u8,
    pub note: u8,
    pub vel: u8,
    pub eng: u8,
    pub loop_: u8,
    pub fbs: u8,
    pub quiet: u8,
    pub played: u8,
    pub frozen: u8,
    pub still: u8,
    pub spb: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Slot {
    pub s: Voice,
    pub used: u8,
    pub gate: u8,
    pub part: u8,
    /// Held-key identity; occupies the existing padding byte before age.
    pub owner: u8,
    pub age: u32,
    pub gate_left: i32,
    pub kill_left: i32,
    pub fade: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Lfo {
    pub ph: u32,
    pub dly: u32,
    pub val: i32,
    pub depth: i32,
    pub rnd: u8,
}
const _: [(); 20] = [(); core::mem::size_of::<Envelope>()];
const _: [(); 24] = [(); core::mem::size_of::<PitchEnvelope>()];
const _: [(); 388] = [(); core::mem::size_of::<Voice>()];
const _: [(); 408] = [(); core::mem::size_of::<Slot>()];
const _: [(); 20] = [(); core::mem::size_of::<Lfo>()];
const _: [(); 120] = [(); core::mem::offset_of!(Voice, ph)];
const _: [(); 196] = [(); core::mem::offset_of!(Voice, gout)];
const _: [(); 268] = [(); core::mem::offset_of!(Voice, base)];
const _: [(); 340] = [(); core::mem::offset_of!(Voice, fb)];
const _: [(); 348] = [(); core::mem::offset_of!(Voice, pe)];
const _: [(); 384] = [(); core::mem::offset_of!(Voice, spb)];
const _: [(); 392] = [(); core::mem::offset_of!(Slot, age)];
const _: [(); 391] = [(); core::mem::offset_of!(Slot, owner)];
