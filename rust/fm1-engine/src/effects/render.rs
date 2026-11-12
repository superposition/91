// SPDX-License-Identifier: GPL-3.0-only
//! Block traversal keeps sampler then tone interleaved per sample. No callback,
//! payload copy or retained buffer reference crosses this boundary.
use super::{sampler, tone};
use crate::sequencer::Rng;

pub struct State<'a> {
    pub sampler: &'a mut sampler::State,
    pub tone: &'a mut tone::State,
    pub rng: &'a mut Rng,
}
pub struct Buffers<'a> {
    pub ring: &'a mut [i16; sampler::WORDS],
    pub tape: &'a mut [[f32; 512]; 2],
    pub flanger: &'a mut [[f32; 512]; 2],
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
}
pub fn process(s: State<'_>, sampler: &sampler::Config, tone: &tone::Config, b: Buffers<'_>) {
    assert_eq!(b.left.len(), b.right.len());
    for j in 0..b.left.len() {
        let x = sampler::sample(s.sampler, sampler, b.ring, s.rng, b.left[j], b.right[j]);
        let x = tone::sample(s.tone, tone, b.tape, b.flanger, x[0], x[1]);
        b.left[j] = x[0];
        b.right[j] = x[1];
    }
}
