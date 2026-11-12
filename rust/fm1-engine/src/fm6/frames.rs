// SPDX-License-Identifier: GPL-3.0-only
//! Existing 64-frame generator queue and exact caller-selected FIFO spans.
use super::{pool, voice, Lookup, Scratch};
use crate::effects::math::{abs, clamp, max};

pub struct Runtime<'a> {
    pub pool: pool::Pool<'a>,
    pub programs: voice::Programs<'a>,
    pub config: &'a voice::Config,
    pub cache: &'a mut voice::AmsCache,
    pub tables: Lookup<'a>,
    pub scratch: Scratch<'a>,
    pub block: &'a mut [f32; 64],
    pub position: &'a mut u32,
    pub bass_sample: &'a mut f32,
}
impl Runtime<'_> {
    /// The outputs are just this call's writable spans, never whole shared
    /// rings. Legacy uploads alias the bass ring, so it is absent outside Jam.
    pub fn render(&mut self, output: &mut [f32], mut bass: Option<&mut [f32]>) -> u32 {
        assert!(output.len() <= 256);
        assert_eq!(bass.is_some(), self.config.jam != 0);
        if let Some(b) = &bass {
            assert_eq!(b.len(), output.len());
        }
        let mut blocks = 0;
        for (i, out) in output.iter_mut().enumerate() {
            if *self.position >= 64 {
                voice::block(
                    pool::Pool {
                        slots: self.pool.slots,
                        keys_lfo: self.pool.keys_lfo,
                        bass_lfo: self.pool.bass_lfo,
                    },
                    voice::Programs {
                        keys: self.programs.keys,
                        bass: self.programs.bass,
                    },
                    self.config,
                    self.cache,
                    &self.tables,
                    &mut self.scratch,
                    self.block,
                );
                *self.position = 0;
                blocks += 1;
            }
            let p = *self.position as usize;
            *self.bass_sample = if self.config.jam != 0 {
                self.scratch.sum[p]
            } else {
                0.0
            };
            *out = clamp(self.block[p], -16.0, 16.0);
            *self.position += 1;
            if let Some(b) = &mut bass {
                b[i] = *self.bass_sample;
            }
        }
        blocks
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OutputConfig {
    pub gain: f32,
    pub start: f32,
    pub end: f32,
    pub inverse: f32,
    pub offset: u32,
}
pub fn output(input: &[f32], outputs: [&mut [f32]; 4], c: &OutputConfig) -> f32 {
    let [left, right, reverb, echo] = outputs;
    assert!(input.len() <= 256);
    assert!(left.len() == input.len() && right.len() == input.len());
    assert!(reverb.len() == input.len() && echo.len() == input.len());
    let mut peak = 0.0;
    for (i, x) in input.iter().enumerate() {
        let track = c.start + (c.end - c.start) * (i as u32 + c.offset) as f32 * c.inverse;
        let y = clamp(*x, -16.0, 16.0) * c.gain * track;
        left[i] = y;
        right[i] = y;
        reverb[i] = y * 0.35;
        echo[i] = y * 0.5;
        peak = max(peak, abs(y));
    }
    peak
}
pub fn bass_output(input: &[f32], output: &mut [f32], gain: f32) -> f32 {
    assert!(input.len() <= 256 && input.len() == output.len());
    let mut peak = 0.0;
    for (x, out) in input.iter().zip(output) {
        *out = *x * gain;
        peak = max(peak, abs(*out));
    }
    peak
}
const _: [(); 20] = [(); core::mem::size_of::<OutputConfig>()];
