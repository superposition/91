// SPDX-License-Identifier: GPL-3.0-only
//! Sidechain/gate/noise merge and differential drum crush, in render order.
#[repr(C)]
pub struct Duck {
    pub envelope: f32,
    pub gate: f32,
}
#[repr(C)]
pub struct DuckConfig {
    pub depth: f32,
    pub decay: f32,
    pub noise_pump: f32,
    pub kick_gain: f32,
    pub step_length: f32,
    pub step: i32,
    pub gate: i32,
    pub odd: u32,
    pub bus: u32,
}
const _: [(); 8] = [(); core::mem::size_of::<Duck>()];
const _: [(); 36] = [(); core::mem::size_of::<DuckConfig>()];
pub struct DuckBuffers<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
    pub reverb: &'a mut [f32],
    pub echo: &'a mut [f32],
    pub bus_left: &'a mut [f32],
    pub bus_right: &'a mut [f32],
    pub factors: &'a mut [f32],
    pub gates: &'a mut [f32],
    pub noise_left: &'a [f32],
    pub noise_right: &'a [f32],
}
pub fn duck(s: &mut Duck, c: &DuckConfig, b: DuckBuffers<'_>) {
    let n = b.left.len();
    assert_eq!(b.right.len(), n);
    assert_eq!(b.reverb.len(), n);
    assert_eq!(b.echo.len(), n);
    assert_eq!(b.bus_left.len(), n);
    assert_eq!(b.bus_right.len(), n);
    assert_eq!(b.factors.len(), n);
    assert_eq!(b.gates.len(), n);
    assert_eq!(b.noise_left.len(), n);
    assert_eq!(b.noise_right.len(), n);
    for j in 0..n {
        let gt = if c.gate == 1 {
            if ((c.step + j as i32) as f32) < 0.5 * c.step_length {
                1.0
            } else {
                0.0
            }
        } else if c.gate == 2 {
            if c.odd != 0 {
                0.0
            } else {
                1.0
            }
        } else {
            1.0
        };
        s.gate += (gt - s.gate) * 0.04;
        b.gates[j] = s.gate;
        b.factors[j] = 1.0 - c.depth * s.envelope;
        s.envelope *= c.decay;
        b.left[j] *= b.factors[j] * s.gate;
        b.right[j] *= b.factors[j] * s.gate;
        b.reverb[j] *= s.gate;
        b.echo[j] *= s.gate;
        let pk = 1.0 - c.noise_pump * 0.95 * s.envelope * c.kick_gain;
        b.left[j] += b.noise_left[j] * pk;
        b.right[j] += b.noise_right[j] * pk;
        b.reverb[j] += (b.noise_left[j] + b.noise_right[j]) * pk * 0.15;
    }
    if c.bus != 0 {
        for j in 0..n {
            b.bus_left[j] *= b.factors[j] * b.gates[j];
            b.bus_right[j] *= b.factors[j] * b.gates[j];
        }
    }
    // The caller retains the unflushed value until the later bass/acid merge;
    // legacy code publishes it to duck_env only after that merge.
}
#[repr(C)]
pub struct Crush {
    pub left: f32,
    pub right: f32,
    pub count: u32,
}
const _: [(); 12] = [(); core::mem::size_of::<Crush>()];
pub struct CrushBuffers<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
    pub before_left: &'a [f32],
    pub before_right: &'a [f32],
}
pub fn crush(s: &mut Crush, b: CrushBuffers<'_>, amount: f32) {
    let n = b.left.len();
    assert_eq!(b.right.len(), n);
    assert_eq!(b.before_left.len(), n);
    assert_eq!(b.before_right.len(), n);
    let hold = 2 + (amount * 4.0) as u32;
    for j in 0..n {
        let dl = b.left[j] - b.before_left[j];
        let dr = b.right[j] - b.before_right[j];
        s.count = s.count.wrapping_add(1);
        if s.count >= hold {
            s.count = 0;
            s.left = (dl * 24.0 + if dl < 0.0 { -0.5 } else { 0.5 }) as i32 as f32 * (1.0 / 24.0);
            s.right = (dr * 24.0 + if dr < 0.0 { -0.5 } else { 0.5 }) as i32 as f32 * (1.0 / 24.0);
        }
        b.left[j] = b.before_left[j] + dl + (s.left - dl) * amount;
        b.right[j] = b.before_right[j] + dr + (s.right - dr) * amount;
    }
}
