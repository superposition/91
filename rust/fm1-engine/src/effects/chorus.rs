// SPDX-License-Identifier: GPL-3.0-only
pub const LENGTH: usize = 512;
#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub index: u32,
    pub phases: [f32; 3],
}
pub fn process(
    s: &mut State,
    buffers: &mut [[f32; LENGTH]; 2],
    left: &mut [f32],
    right: &mut [f32],
    amount: f32,
) {
    assert_eq!(left.len(), right.len());
    const RATES: [f32; 3] = [0.47 / 44100.0, 0.61 / 44100.0, 0.79 / 44100.0];
    for j in 0..left.len() {
        let dl = left[j];
        let dr = right[j];
        let mut wl = 0.0;
        let mut wr = 0.0;
        buffers[0][s.index as usize] = dl;
        buffers[1][s.index as usize] = dr;
        if amount > 0.0 {
            for k in 0..3 {
                let tri = if s.phases[k] < 0.5 {
                    4.0 * s.phases[k] - 1.0
                } else {
                    3.0 - 4.0 * s.phases[k]
                };
                let d1 = (0.007 + 0.003 * tri) * 44100.0;
                let d2 = (0.007 - 0.003 * tri) * 44100.0;
                wl += read(&buffers[0], s.index, d1);
                wr += read(&buffers[1], s.index, d2);
            }
        }
        for (phase, rate) in s.phases.iter_mut().zip(RATES) {
            *phase += rate;
            if *phase >= 1.0 {
                *phase -= 1.0;
            }
        }
        s.index = if s.index + 1 >= 512 { 0 } else { s.index + 1 };
        left[j] = dl + wl * amount * 0.45;
        right[j] = dr + wr * amount * 0.45;
    }
}
fn read(buffer: &[f32; LENGTH], index: u32, delay: f32) -> f32 {
    let mut rp = index as f32 - delay;
    if rp < 0.0 {
        rp += 512.0;
    }
    let mut i = rp as usize;
    if i >= 512 {
        i -= 512;
        rp -= 512.0;
    }
    let f = rp - i as f32;
    let next = if i + 1 >= 512 { 0 } else { i + 1 };
    buffer[i] + (buffer[next] - buffer[i]) * f
}
