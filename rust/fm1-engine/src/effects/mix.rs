// SPDX-License-Identifier: GPL-3.0-only
//! Sidechain/gate/noise merge and differential drum crush, in render order.
use super::{echo, math};

/// Six disjoint, foreground-owned working spans. Shared delay rings are not
/// contained here; callers pass only the exact readable pad span separately.
pub struct Buffers<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
    pub reverb: &'a mut [f32],
    pub echo: &'a mut [f32],
    pub bus_left: &'a mut [f32],
    pub bus_right: &'a mut [f32],
}
impl Buffers<'_> {
    fn len(&self) -> usize {
        let n = self.left.len();
        assert_eq!(self.right.len(), n);
        assert_eq!(self.reverb.len(), n);
        assert_eq!(self.echo.len(), n);
        assert_eq!(self.bus_left.len(), n);
        assert_eq!(self.bus_right.len(), n);
        n
    }
}

#[repr(C)]
pub struct MergeConfig {
    pub first: [f32; 3],
    pub second: [f32; 3],
    pub first_bus: u32,
    pub second_bus: u32,
}
const _: [(); 32] = [(); core::mem::size_of::<MergeConfig>()];
pub struct PadFm<'a> {
    pub pad_left: &'a [f32],
    pub pad_right: &'a [f32],
    pub pad_dry: &'a [f32],
    pub fm_left: &'a [f32],
    pub fm_right: &'a [f32],
    pub fm_reverb: &'a [f32],
    pub fm_echo: &'a [f32],
}
/// Preserve per-sample pad-left/right, then FM-left/right, then send addition
/// order, including the case where both sources target the same accumulator.
pub fn pad_fm(b: Buffers<'_>, c: &MergeConfig, s: PadFm<'_>) {
    let n = b.len();
    for len in [
        s.pad_left.len(),
        s.pad_right.len(),
        s.pad_dry.len(),
        s.fm_left.len(),
        s.fm_right.len(),
        s.fm_reverb.len(),
        s.fm_echo.len(),
    ] {
        assert_eq!(len, n);
    }
    for j in 0..n {
        let ds = s.pad_dry[j] * c.first[2];
        if c.first_bus != 0 {
            b.bus_left[j] += s.pad_left[j] * c.first[0];
            b.bus_right[j] += s.pad_right[j] * c.first[1];
        } else {
            b.left[j] += s.pad_left[j] * c.first[0];
            b.right[j] += s.pad_right[j] * c.first[1];
        }
        if c.second_bus != 0 {
            b.bus_left[j] += s.fm_left[j] * c.second[0];
            b.bus_right[j] += s.fm_right[j] * c.second[1];
        } else {
            b.left[j] += s.fm_left[j] * c.second[0];
            b.right[j] += s.fm_right[j] * c.second[1];
        }
        b.reverb[j] += ds * 0.6 + s.fm_reverb[j] * c.second[2];
        b.echo[j] += ds * 0.2 + s.fm_echo[j] * c.second[2];
    }
}
pub struct BassAcid<'a> {
    pub bass: &'a [f32],
    pub acid: &'a [f32],
    pub duck: &'a [f32],
    pub gate: &'a [f32],
}
pub fn bass_acid(b: Buffers<'_>, c: &MergeConfig, s: BassAcid<'_>) {
    let n = b.len();
    for len in [s.bass.len(), s.acid.len(), s.duck.len(), s.gate.len()] {
        assert_eq!(len, n);
    }
    for j in 0..n {
        let bass = s.bass[j];
        let acid = s.acid[j];
        let dg = s.duck[j] * s.gate[j];
        if c.first_bus != 0 {
            b.bus_left[j] += bass * c.first[0] * dg;
            b.bus_right[j] += bass * c.first[1] * dg;
        } else {
            b.left[j] += bass * c.first[0] * dg;
            b.right[j] += bass * c.first[1] * dg;
        }
        if c.second_bus != 0 {
            b.bus_left[j] += acid * c.second[0] * dg;
            b.bus_right[j] += acid * c.second[1] * dg;
        } else {
            b.left[j] += acid * c.second[0] * dg;
            b.right[j] += acid * c.second[1] * dg;
        }
        b.reverb[j] += (bass * 0.06 * c.first[2] + acid * 0.15 * c.second[2]) * s.gate[j];
        b.echo[j] += (bass * 0.04 * c.first[2] + acid * 0.35 * c.second[2]) * s.gate[j];
    }
}
pub fn route_plain(b: Buffers<'_>) {
    for j in 0..b.len() {
        b.bus_left[j] = b.left[j];
        b.bus_right[j] = b.right[j];
        b.left[j] = 0.0;
        b.right[j] = 0.0;
    }
}
// Capture each drum contribution before replacing the working sample. This
// interleaving intentionally keeps the legacy per-sample operation order.
#[allow(clippy::manual_memcpy)]
pub fn route_drums(b: Buffers<'_>, before_left: &[f32], before_right: &[f32]) {
    let n = b.len();
    assert_eq!(before_left.len(), n);
    assert_eq!(before_right.len(), n);
    for j in 0..n {
        b.bus_left[j] += b.left[j] - before_left[j];
        b.bus_right[j] += b.right[j] - before_right[j];
        b.left[j] = before_left[j];
        b.right[j] = before_right[j];
    }
}
pub fn reverb_gain(send: f32, input_gain: f32, arc: f32, breakdown: f32) -> f32 {
    send * input_gain * (1.0 - 0.25 * arc) * (1.0 + 0.7 * math::max(breakdown, 0.0))
}
pub fn duck_depth(kit: f32, pulse: f32, kick_gain: f32) -> f32 {
    math::min(0.55 * kit, 0.85) * math::min(pulse * 1.5, 1.0) * kick_gain
}
#[repr(C)]
pub struct EchoSends {
    pub reverb_gain: f32,
    pub echo_gain: f32,
    pub reverb_send: f32,
    pub throw: f32,
    pub track_mix: f32,
    pub track_send: i32,
    pub wash: f32,
}
const _: [(); 28] = [(); core::mem::size_of::<EchoSends>()];
/// Reuses each now-consumed reverb-send slot for a quantized predelay word.
/// Every i16 is exactly representable as f32. The caller only casts/stores these
/// words in order into disjoint shared predelay slots; it performs no DSP math.
/// No predelay reference, callback or extra audio payload is needed.
pub fn echoes(
    s: &mut echo::State,
    c: &echo::Config,
    tape: &mut [i16; echo::TAPE],
    plex: &mut [i16; echo::PLEX],
    sends: &EchoSends,
    b: Buffers<'_>,
    duck: &mut [f32],
) {
    let n = b.len();
    assert_eq!(duck.len(), n);
    let throw = sends.throw * 0.5;
    let tdg = sends.track_mix * sends.track_send as f32 * 0.006;
    for (j, duck) in duck.iter_mut().enumerate() {
        let input = b.echo[j] * sends.echo_gain
            + throw * (b.left[j] + b.right[j])
            + tdg * (b.bus_left[j] + b.bus_right[j]);
        let [el, er, xl, xr] = echo::sample(s, c, tape, plex, input);
        b.reverb[j] = (b.reverb[j] * sends.reverb_gain
            + (el + er + xl + xr) * 0.35 * sends.reverb_send
            + sends.wash * 0.2 * (b.left[j] + b.right[j]))
            * 0.8;
        b.reverb[j] = super::reverb::quantize(b.reverb[j]) as f32;
        b.left[j] = (b.left[j] + (0.75 * el + 0.15 * xl) * *duck) * 0.36;
        b.right[j] = (b.right[j] + (0.75 * er + 0.15 * xr) * *duck) * 0.36;
        *duck *= 1.3 * 0.36;
    }
}
pub fn tank_return(b: Buffers<'_>, left: &[f32], right: &[f32], duck: &[f32]) {
    let n = b.len();
    assert_eq!(left.len(), n);
    assert_eq!(right.len(), n);
    assert_eq!(duck.len(), n);
    for j in 0..n {
        b.left[j] += duck[j] * left[j];
        b.right[j] += duck[j] * right[j];
    }
}
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
