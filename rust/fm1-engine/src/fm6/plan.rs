// SPDX-License-Identifier: GPL-3.0-only
// Routing and gain ramps from the attributed MSFA/Dexed-based C implementation.
use super::{tables::FM6_ALG, types::Voice, MARK1, MODERN};
use crate::effects::math::exp2;
pub fn prepare(s: &mut Voice, levels: &[i32; 6], algorithm: u32, engine: u32, feedback_shift: u32) {
    assert!(algorithm < 32);
    let mut has = 1u32;
    let feedback = feedback_shift < 16;
    if s.eng as u32 != engine {
        s.gout.fill(0.0);
        s.fb = [0.0; 2];
    }
    s.eng = engine as u8;
    s.loop_ = 0;
    let mut k = 0;
    while k < 6 {
        let mut flags = FM6_ALG[algorithm as usize][k] as u32;
        let mut output = flags & 3;
        let mut input = (flags >> 4) & 3;
        let mut add = flags & 4;
        if engine == MARK1 && k == 0 && feedback && (algorithm == 3 || algorithm == 5) {
            flags = 0xc4;
            output = 0;
            input = 0;
            add = 4;
        }
        let (g1, g2, run) = if engine == MODERN {
            let g1 = s.gout[k];
            let g2 = if levels[k] == 0 {
                1.0 / 16384.0
            } else {
                exp2(levels[k] as f32 * (1.0 / 16777216.0) - 14.0)
            };
            (
                g1,
                g2,
                g1 >= 1120.0 / 16777216.0 || g2 >= 1120.0 / 16777216.0,
            )
        } else if engine == MARK1 {
            let g1 = if s.gout[k] != 0.0 { s.gout[k] } else { 16383.0 };
            let g2 = (16384 - (levels[k] >> 14)) as f32;
            (g1, g2, g1 <= 16284.0 || g2 <= 16284.0)
        } else {
            let g1 = if s.gout[k] != 0.0 { s.gout[k] } else { 511.0 };
            let g2 = (512 - (levels[k] >> 19)) as f32;
            (g1, g2, g1 <= 507.0 || g2 <= 507.0)
        };
        s.gout[k] = g2;
        s.plan[k] = 0;
        if !run {
            if add == 0 {
                has &= !(1 << output);
            }
            s.ph[k] = s.ph[k].wrapping_add((s.fq[k] as u32) << 6);
            k += 1;
            continue;
        }
        if has >> output & 1 == 0 {
            add = 0;
        }
        if input != 0 && has >> input & 1 == 0 {
            input = 0;
        }
        s.g[k] = g1;
        s.dg[k] = if engine == MODERN {
            (g2 - g1) * (1.0 / 64.0)
        } else {
            (((g2 - g1) as i32 + 32) >> 6) as f32
        };
        s.plan[k] = (0x80 | output | add | (input << 4)) as u8;
        if input == 0 && flags & 0xc0 == 0xc0 && feedback {
            s.plan[k] |= 0x40;
            s.fbs = feedback_shift as u8;
            if engine == MARK1 && (algorithm == 3 || algorithm == 5 || algorithm == 31) {
                s.fbs = (feedback_shift + 2).min(16) as u8;
            }
            if engine == MARK1 && (algorithm == 3 || algorithm == 5) {
                s.loop_ = if algorithm == 3 { 3 } else { 2 };
                for (j, level) in levels.iter().enumerate().take(s.loop_ as usize).skip(1) {
                    s.gout[j] = (16384 - (level >> 14)) as f32;
                    s.g[j] = if s.gout[j] != 0.0 { s.gout[j] } else { 16383.0 };
                    s.dg[j] = 0.0;
                    s.plan[j] = 0;
                }
                k += s.loop_ as usize - 1;
            }
        }
        has |= 1 << output;
        k += 1;
    }
}
