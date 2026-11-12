// SPDX-License-Identifier: GPL-3.0-only
//! Quarter-rate tape heads and the full-rate Echoplex. Payload stays caller-owned.
pub const TAPE: usize = 21800;
pub const PLEX: usize = 7100;
/// Historical boot/reset behavior is intentionally partial: filter/history
/// scalars and head pan/lean/timer survive while payload and these cursors reset.
pub fn reset(s: &mut State, h: &mut Heads, tape: &mut [i16; TAPE], plex: &mut [i16; PLEX]) {
    tape.fill(0);
    plex.fill(0);
    s.tape_write = 0;
    s.plex_write = 0;
    s.phase = 0;
    s.head2 = 0.75;
    h.target = 0.75;
    h.speed = 0.0;
    s.length = 0.0;
}
#[repr(C)]
pub struct Heads {
    pub pan: [f32; 4],
    pub lean: f32,
    pub target: f32,
    pub speed: f32,
    pub left: u32,
}
#[repr(C)]
pub struct HeadConfig {
    pub motion: f32,
    pub modulation: [f32; 3],
    pub length: f32,
    pub head: f32,
    pub playing: u32,
    pub frozen: u32,
}
const _: [(); 32] = [(); core::mem::size_of::<Heads>()];
const _: [(); 32] = [(); core::mem::size_of::<HeadConfig>()];
pub fn heads(s: &mut Heads, c: &HeadConfig, n: u32, rng: &mut crate::sequencer::Rng) {
    use super::math::*;
    for i in 0..2 {
        let mut p = clamp(
            (if i != 0 { 0.7 } else { -0.7 }) + 0.9 * c.motion * c.modulation[i],
            -1.0,
            1.0,
        );
        p = (p + 1.0) * (core::f32::consts::PI / 4.0);
        s.pan[2 * i] = cos(p);
        s.pan[2 * i + 1] = sin(p);
    }
    s.lean = 0.25 * c.motion * (1.0 + c.modulation[2]);
    if s.left <= n {
        s.left = ((20.0 + 40.0 * rng.fraction()) * 44100.0) as u32;
        if c.playing != 0 && c.frozen == 0 && rng.fraction() < c.motion {
            s.target = [0.75, 0.6667, 0.625, 0.5, 0.375][(rng.fraction() * 4.999) as usize];
        }
    } else {
        s.left -= n;
    }
    let vmax = 0.008 / max(c.length, 100.0);
    let want = clamp((s.target - c.head) * (1.0 / (2.0 * 11025.0)), -vmax, vmax);
    s.speed += (want - s.speed) * min(n as f32 * (1.0 / 44100.0), 1.0);
}
pub fn length(mut index: usize, tempo: i32, current: f32, beats: &[f32; 6]) -> Option<[f32; 2]> {
    if tempo <= 0 {
        return None;
    }
    let beat = 60.0 / tempo as f32;
    let mut t = beats[index] * beat;
    while t * 11025.0 > (TAPE - 8) as f32 && index > 0 {
        index -= 1;
        t = beats[index] * beat;
    }
    if t * 11025.0 > (TAPE - 8) as f32 {
        t = (TAPE - 8) as f32 / 11025.0;
    }
    let target = t * 11025.0;
    Some([if current <= 0.0 { target } else { current }, target])
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub tape_write: i32,
    pub phase: i32,
    pub accumulator: f32,
    pub low: f32,
    pub high: f32,
    pub previous1: f32,
    pub previous2: f32,
    pub current1: f32,
    pub current2: f32,
    pub plex_write: i32,
    pub plex_low: f32,
    pub plex_low2: f32,
    pub length: f32,
    pub head2: f32,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub target_length: f32,
    pub head_speed: f32,
    pub motion: f32,
    pub modulation: f32,
    pub lfo: f32,
    pub lean: f32,
    pub feedback: f32,
    pub pan: [f32; 4],
    pub shed: u32,
}
const _: [(); 56] = [(); core::mem::size_of::<State>()];
const _: [(); 48] = [(); core::mem::size_of::<Config>()];
fn read(tape: &[i16; TAPE], write: i32, delay: f32) -> f32 {
    let k = delay as i32;
    let fraction = delay - k as f32;
    let mut i0 = write - k;
    let mut i1 = i0 - 1;
    if i0 < 0 {
        i0 += TAPE as i32;
    }
    if i1 < 0 {
        i1 += TAPE as i32;
    }
    let a = tape[i0 as usize] as f32;
    let c = tape[i1 as usize] as f32;
    (a + (c - a) * fraction) * (1.0 / 12000.0)
}
pub fn quantize(mut x: f32) -> i16 {
    x *= 12000.0;
    super::math::clamp(x, -32767.0, 32767.0) as i16
}
pub fn sample(
    s: &mut State,
    c: &Config,
    tape: &mut [i16; TAPE],
    plex: &mut [i16; PLEX],
    input: f32,
) -> [f32; 4] {
    let (xl, xr);
    if c.shed != 0 {
        xl = 0.0;
        xr = 0.0;
        s.accumulator += input;
    } else {
        let mut t = s.plex_write - 2700;
        if t < 0 {
            t += PLEX as i32;
        }
        let y = plex[s.plex_write as usize] as f32 * (1.0 / 12000.0);
        s.plex_low += 0.35 * (y - s.plex_low);
        s.plex_low2 += 0.35 * (plex[t as usize] as f32 * (1.0 / 12000.0) - s.plex_low2);
        plex[s.plex_write as usize] = quantize(input + 0.55 * s.plex_low);
        s.plex_write += 1;
        if s.plex_write >= PLEX as i32 {
            s.plex_write = 0;
        }
        xl = s.plex_low;
        xr = s.plex_low2;
        s.accumulator += input + 0.3 * s.plex_low;
    }
    s.phase += 1;
    if s.phase >= 4 {
        s.phase = 0;
        s.length += (c.target_length - s.length) * 0.001;
        s.head2 += c.head_speed;
        let mut d = s.length * (1.0 + 0.003 * c.motion * c.modulation + c.lfo);
        if d > (TAPE - 4) as f32 {
            d = (TAPE - 4) as f32;
        }
        let h1 = read(tape, s.tape_write, d);
        let h2 = read(tape, s.tape_write, d * s.head2);
        s.low += 0.55 * (h1 + c.lean * (h2 - h1) - s.low);
        s.high += 0.03 * (s.low - s.high);
        let feedback = s.low - s.high;
        s.tape_write += 1;
        if s.tape_write >= TAPE as i32 {
            s.tape_write = 0;
        }
        tape[s.tape_write as usize] = quantize(s.accumulator * 0.25 + c.feedback * feedback);
        s.accumulator = 0.0;
        s.previous1 = s.current1;
        s.previous2 = s.current2;
        s.current1 = h1;
        s.current2 = h2;
    }
    let f = s.phase as f32 * 0.25;
    let h1 = s.previous1 + (s.current1 - s.previous1) * f;
    let h2 = s.previous2 + (s.current2 - s.previous2) * f;
    [
        c.pan[0] * h1 + c.pan[2] * h2,
        c.pan[1] * h1 + c.pan[3] * h2,
        xl,
        xr,
    ]
}
