// SPDX-License-Identifier: GPL-3.0-only
//! Pure scale/chord construction and four-voice register search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale {
    pub steps: [i32; 12],
    pub count: usize,
}
pub fn scale(pool: u32) -> Scale {
    let mut result = Scale {
        steps: [0; 12],
        count: 0,
    };
    let pool = pool | 1;
    for i in 0..12 {
        if pool >> i & 1 != 0 {
            result.steps[result.count] = i;
            result.count += 1;
        }
    }
    result
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub pcs: [i32; 4],
    pub count: usize,
}
pub const OFF: [[u8; 7]; 5] = [
    [0, 2, 4, 6, 1, 3, 5],
    [0, 2, 6, 1, 4, 3, 5],
    [0, 3, 4, 6, 1, 2, 5],
    [0, 6, 1, 3, 2, 4, 5],
    [0, 1, 4, 6, 2, 3, 5],
];
pub const SHAPE: [[u8; 4]; 2] = [[0, 3, 10, 14], [0, 5, 10, 15]];
pub fn chord(pool: u32, root: i32, degree: i32, kind: i32) -> Chord {
    let st = scale(pool);
    let m = st.count as i32;
    let degree = ((degree % m + m) % m) as usize;
    let mut result = Chord {
        pcs: [0; 4],
        count: 0,
    };
    if kind == 5 || kind == 6 {
        for (i, offset) in SHAPE[usize::from(kind == 6)].iter().enumerate() {
            result.pcs[i] = root
                .wrapping_add(st.steps[degree])
                .wrapping_add(*offset as i32)
                % 12;
        }
        result.count = 4;
        return result;
    }
    let want = if kind == 1 {
        3
    } else if kind == 0 {
        if m >= 6 {
            4
        } else {
            3
        }
    } else {
        4
    };
    let off = match kind {
        3 => &OFF[1],
        4 => {
            if m == 7 && (st.steps[(degree + 3) % st.count] - st.steps[degree] + 12) % 12 == 6 {
                &OFF[4]
            } else {
                &OFF[2]
            }
        }
        7 => &OFF[3],
        _ => &OFF[0],
    };
    for offset in off {
        if result.count >= want {
            break;
        }
        let pc = root.wrapping_add(st.steps[(degree + *offset as usize) % st.count]) % 12;
        if !result.pcs[..result.count].contains(&pc) {
            result.pcs[result.count] = pc;
            result.count += 1;
        }
    }
    result
}
pub const PERM: [[u8; 4]; 24] = [
    [0, 1, 2, 3],
    [0, 1, 3, 2],
    [0, 2, 1, 3],
    [0, 2, 3, 1],
    [0, 3, 1, 2],
    [0, 3, 2, 1],
    [1, 0, 2, 3],
    [1, 0, 3, 2],
    [1, 2, 0, 3],
    [1, 2, 3, 0],
    [1, 3, 0, 2],
    [1, 3, 2, 0],
    [2, 0, 1, 3],
    [2, 0, 3, 1],
    [2, 1, 0, 3],
    [2, 1, 3, 0],
    [2, 3, 0, 1],
    [2, 3, 1, 0],
    [3, 0, 1, 2],
    [3, 0, 2, 1],
    [3, 1, 0, 2],
    [3, 1, 2, 0],
    [3, 2, 0, 1],
    [3, 2, 1, 0],
];
/// Replaces the original repeated ±12 corrections with exact integer ceilings.
/// Inputs are bounded musical/register integers, far from i32 overflow.
pub fn nearest(from: i32, pc: i32, lo: i32, hi: i32) -> i32 {
    let mut n = from + ((pc - from) % 12 + 12) % 12;
    if n - from > 6 {
        n -= 12;
    }
    if n < lo {
        n += ((lo - n + 11) / 12) * 12;
    }
    if n > hi {
        n -= ((n - hi + 11) / 12) * 12;
    }
    n
}
/// `aliases[v]` identifies a from slot overwritten by output slot v, or -1.
/// This models the C helper's successive best-candidate writes on a copied
/// arena, without aliased Rust references. Normal consumers use [-1;4].
pub fn voice_lead(
    mut from: [i32; 4],
    pcs: &[i32; 4],
    count: i32,
    lo: i32,
    hi: i32,
    aliases: [i8; 4],
) -> [i32; 4] {
    let mut tones = [0; 4];
    for v in 0..4 {
        tones[v] = pcs[if (v as i32) < count { v } else { 0 }];
    }
    let mut result = [0; 4];
    let mut best = 1 << 30;
    for perm in PERM {
        let mut n = [0; 4];
        let mut cost = 0;
        for v in 0..4 {
            n[v] = nearest(from[v], tones[perm[v] as usize], lo, hi);
            cost += (n[v] - from[v]).abs();
        }
        for a in 0..4 {
            for b in a + 1..4 {
                if n[a] == n[b] {
                    cost += 40;
                }
            }
        }
        if cost < best {
            best = cost;
            result = n;
            for v in 0..4 {
                if (0..4).contains(&aliases[v]) {
                    from[aliases[v] as usize] = n[v];
                }
            }
        }
    }
    result
}

/// The existing four queued bell notes, captured before application dispatch.
pub fn bing(root: i32, up: bool) -> super::abi::Bing {
    const OFF: [i8; 4] = [-6, 0, 3, 9];
    const VELOCITIES: [[u8; 4]; 2] = [[66, 78, 72, 86], [70, 84, 74, 92]];
    let mut result = super::abi::Bing::default();
    for i in 0..4 {
        result.keys[i] = 48 + root + i32::from(up) + i32::from(OFF[i]);
        result.velocities[i] = i32::from(VELOCITIES[usize::from(up)][i]);
    }
    result
}
/// The pedal follows the key root; its switch does not change the shown chord.
pub fn bass_pc(pedal: u32, root: i32, first: i32) -> i32 {
    if pedal & 1 != 0 {
        root % 12
    } else {
        first
    }
}
