// SPDX-License-Identifier: GPL-3.0-only
//! Bounded drum decisions. Voice synthesis and MIDI dispatch are caller actions;
//! their order and the shared xorshift stream remain part of the musical ABI.

const GROOVE: i32 = 0;
const BREAK: i32 = 1;
const BUILD: i32 = 2;
const KICK: u32 = 0;
const SNARE: u32 = 1;
const CLAP: u32 = 2;
const HAT: u32 = 3;
const OHAT: u32 = 4;
const PERC: u32 = 5;
const ROLL: u32 = 1;
const HROLL: u32 = 4;
const PBUILD: u32 = 16;
const PFILL: u32 = 32;
const KDROP: u32 = 64;
const IMP: [u8; 16] = [0, 8, 4, 12, 2, 10, 6, 14, 1, 3, 5, 7, 9, 11, 13, 15];
const SLICE: [[u8; 16]; 8] = [
    [8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 0, 1, 2, 3, 8, 9, 10, 11, 8, 9, 10, 11],
    [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 6, 7, 8, 9, 10, 11, 12, 13],
    [0, 1, 2, 3, 8, 9, 10, 11, 4, 5, 6, 7, 12, 13, 14, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 7, 6, 5, 4, 3, 2, 1, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7],
    [4, 5, 6, 7, 0, 1, 2, 3, 12, 13, 14, 15, 8, 9, 10, 11],
];
const GHOST_SN: [u16; 19] = [
    0, 0x8480, 0x0200, 0, 0, 0x8200, 0xc288, 0x0408, 0, 0, 0, 0x8a48, 0, 0, 0, 0x8280, 0x8a20, 0,
    0x0420,
];
const GHOST_KK: [u16; 19] = [
    0, 0x4400, 0, 0, 0, 0, 0x2200, 0, 0, 0, 0, 0x2200, 0, 0, 0, 0x4400, 0x0880, 0, 0,
];
const FILLS: [[u8; 16]; 5] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 5, 0, 6, 7, 8, 9],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 4, 6, 9],
    [0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 5, 6, 7, 8, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 6, 0, 7, 0, 9],
    [0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 5, 6, 8, 9],
];

/// The common stream is distinct from the Songbook planner's LCG stream.
pub struct Rng(pub u32);
impl Rng {
    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    pub fn fraction(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
    pub fn index(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next_u32() >> 8) % n
        }
    }
}

pub fn density(mask: u16, amount: i32) -> u16 {
    let mut bits = mask as u32;
    let pop = bits.count_ones() as i32;
    if amount < 50 {
        let mut count = (pop * (50 - amount) + 25) / 50;
        for step in IMP.iter().rev() {
            if count > 0 && bits >> step & 1 != 0 {
                bits &= !(1u32 << step);
                count -= 1;
            }
        }
    } else if amount > 50 {
        let mut count = ((16 - pop) * (amount - 50) + 50) / 100;
        for step in IMP {
            if count > 0 && bits >> step & 1 == 0 {
                bits |= 1u32 << step;
                count -= 1;
            }
        }
    }
    bits as u16
}

pub fn condition(condition: u32, bars: u32, fill: bool) -> bool {
    let n = bars.wrapping_sub(1);
    match condition {
        1 => n & 1 == 0,
        2 => n & 1 != 0,
        3 => n.is_multiple_of(4),
        4 => n % 4 == 3,
        5 => n.is_multiple_of(8),
        6 => fill,
        7 => !fill,
        _ => true,
    }
}

pub fn pattern_bit(
    mask: u16,
    mut step: u32,
    poly: bool,
    time: u32,
    sixteenth: u32,
    slice: u32,
) -> bool {
    if poly {
        return mask as u32 >> (step & 15) & 1 != 0;
    }
    if time == 1 {
        if sixteenth & 1 != 0 {
            return false;
        }
        step = sixteenth >> 1 & 15;
    } else if time == 2 {
        let s = (step & 7) * 2;
        return ((mask as u32 >> s) | (mask as u32 >> (s + 1))) & 1 != 0;
    }
    if (1..=8).contains(&slice) {
        step = SLICE[(slice - 1) as usize][(step & 15) as usize] as u32;
    }
    mask as u32 >> (step & 15) & 1 != 0
}

pub fn swing_delay(sixteenth: u32, swing: f32, length: f32) -> i32 {
    if sixteenth & 1 != 0 {
        (swing.min(0.5) * length) as i32
    } else {
        0
    }
}
pub fn loose(mut wait: i32, amount: u32, milliseconds: f32, rng: &mut Rng) -> i32 {
    if amount != 0 {
        let r = rng.fraction();
        wait += (r * r * amount as f32 * 0.01 * milliseconds * 0.001 * 44_100.0) as i32;
    }
    wait
}

/// Scalar request, captured by the render owner. Flags are explicit u32 values
/// rather than foreign bool/enum layouts. Validated firmware parameters bound
/// lane/genre/step/slice/fill indices before this request reaches the ABI.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Lane {
    pub mask: u32,
    pub jam_mask: u32,
    pub lane: u32,
    pub sixteenth: u32,
    pub bars: u32,
    pub length: u32,
    pub condition: u32,
    pub time: u32,
    pub slice: u32,
    pub section: i32,
    pub section_bars: i32,
    pub section_length: i32,
    pub section_request: i32,
    pub groove_bars: u32,
    pub section_flags: u32,
    pub keep: u32,
    pub genre: u32,
    pub fill_hold: u32,
    pub fill_bar: u32,
    pub drift_fill: u32,
    pub fill_kind: u32,
    pub drop_impact: u32,
    pub jam: u32,
    pub modification: u32,
    pub ghost: u32,
    pub probability: u32,
    pub accent: u32,
    pub human: u32,
    pub fx: u32,
    pub energy: u32,
    pub step_length: f32,
    pub swing: i32,
    pub hat_level: f32,
    pub chop: f32,
}

/// At most three main-hit actions. Pending older hits are flushed by the caller
/// first, in their original order. Action kinds: 1 fire, 2 wait1, 3 wait2,
/// 4 continue ratchet, 16+n configure n remaining ratchet hits.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Action {
    pub kind: u32,
    pub wait: i32,
    pub velocity: f32,
    pub multiplier: f32,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Hit {
    pub step: u32,
    pub hit: u32,
    pub kick: u32,
    pub crash: u32,
    pub hat: u32,
    pub perc: u32,
    pub count: u32,
    pub reserved: u32,
    pub actions: [Action; 3],
}
const _: [(); 136] = [(); core::mem::size_of::<Lane>()];
const _: [(); 80] = [(); core::mem::size_of::<Hit>()];

fn action(kind: u32, wait: i32, velocity: f32, multiplier: f32) -> Action {
    Action {
        kind,
        wait,
        velocity,
        multiplier,
    }
}

/// Decisions and random draws for one lane, before voice dispatch. Voice firing
/// does not consume this RNG stream, so returning its final word preserves the
/// original interleaving between lanes and other tick clients.
pub fn lane(r: &Lane, rng: &mut Rng) -> Hit {
    let l = r.lane;
    let st = r.sixteenth & 15;
    let length = if (1..16).contains(&r.length) {
        r.length
    } else {
        16
    };
    let ls = if length < 16 {
        r.sixteenth % length
    } else {
        st
    };
    let fill = r.fill_hold != 0 || r.fill_bar != 0 || (r.drift_fill != 0 && st >= 8);
    let groove_fill = r.fill_hold != 0
        || r.fill_bar != 0
        || (r.drift_fill != 0 && st >= 8 && r.section == GROOVE);
    let mut hit = pattern_bit(r.mask as u16, ls, length < 16, r.time, r.sixteenth, r.slice)
        && condition(r.condition, r.bars, fill);
    if r.jam != 0 {
        hit = r.jam_mask >> st & 1 != 0;
    }
    let mut vel = 0.9f32;
    let mut dbl = 0;
    let mut push = 0i32;
    let mut rat = 0usize;
    if l == KICK
        && r.section == GROOVE
        && st >= 12
        && r.section_flags & KDROP != 0
        && r.groove_bars.is_multiple_of(8)
    {
        hit = false;
    }
    if r.section == BREAK {
        hit = hit
            && ((l == KICK && r.keep & 1 != 0)
                || (l == HAT && r.keep & 2 != 0)
                || (l == SNARE && r.keep & 4 != 0));
    } else if r.section == BUILD {
        if l == PERC && r.section_flags & PBUILD != 0 {
            hit = if r.section_bars > 1 {
                st & 1 == 0
            } else {
                st < 14
            };
            vel = 0.3
                + 0.6 * ((r.section_length - r.section_bars) * 16 + st as i32) as f32
                    / (r.section_length * 16) as f32;
        } else if l == KICK || l == CLAP || l == PERC {
            hit = l == KICK && r.section_bars == 1 && r.keep & 8 != 0 && (st == 0 || st == 8);
        } else if l == SNARE && r.section_flags & PBUILD != 0 {
            hit = false;
        } else if l == SNARE {
            hit = if r.section_bars > 2 {
                st & 3 == 0
            } else if r.section_bars == 2 {
                st & 1 == 0
            } else {
                st < 14
            };
            vel = 0.3
                + 0.7 * ((r.section_length - r.section_bars) * 16 + st as i32) as f32
                    / (r.section_length * 16) as f32;
            if r.section_bars == 1 && r.section_flags & ROLL != 0 && (8..14).contains(&st) {
                dbl = if st >= 12 { 2 } else { 1 };
            }
        } else if l == OHAT && r.section_bars == 1 {
            hit = st & 1 == 0;
            vel = 0.55;
        }
    } else if r.section == GROOVE
        && r.section_request == 1
        && (l == PERC || l == OHAT || (l == HAT && st & 1 != 0))
    {
        hit = false;
    }
    if groove_fill
        && l == PERC
        && st >= 8
        && ((r.fill_kind == 6 && r.section_flags & PFILL != 0) || r.fill_kind == 7)
    {
        hit = if r.fill_kind == 7 {
            st != 9 && st != 11
        } else {
            st >= 12 || st & 1 == 0
        };
        vel = 0.5 + 0.05 * (st - 8) as f32;
        dbl = if r.fill_kind == 6 && st >= 12 { 2 } else { 0 };
    }
    if groove_fill && (l == SNARE || l == HAT) {
        if r.fill_kind == 0 {
            hit = true;
            vel = if l == SNARE {
                0.35 + 0.04 * st as f32
            } else {
                0.5
            };
        } else if l == SNARE
            && st >= 8
            && (r.fill_kind == 7 || (r.fill_kind == 6 && r.section_flags & PFILL != 0))
        {
            hit = false;
        } else if l == SNARE && st >= 8 && r.fill_kind == 6 {
            hit = st >= 12 || st & 1 == 0;
            vel = 0.45 + 0.06 * (st - 8) as f32;
            dbl = if st >= 12 { 2 } else { 0 };
        } else if l == SNARE && st >= 8 {
            let fv = FILLS[(r.fill_kind - 1) as usize][st as usize];
            hit = fv != 0;
            vel = 0.1 * fv as f32 + 0.1;
        } else if l == HAT && st >= 8 && r.section_flags & HROLL != 0 {
            hit = true;
            vel = if st & 1 != 0 { 0.45 } else { 0.6 };
            dbl = if st >= 12 { 2 } else { 1 };
        }
    }
    if r.chop > 0.0 && r.section == GROOVE && r.fill_hold == 0 {
        if !hit && l == SNARE && rng.fraction() < 0.16 * r.chop {
            hit = true;
            vel = 0.25 + 0.25 * rng.fraction();
        } else if !hit && l == KICK && st & 1 != 0 && rng.fraction() < 0.1 * r.chop {
            hit = true;
            vel = 0.7;
        } else if l == HAT && rng.fraction() < 0.25 * r.chop {
            hit = !hit;
            vel = 0.5;
        }
    }
    if !hit
        && r.ghost != 0
        && r.section == GROOVE
        && r.fill_hold == 0
        && r.fill_bar == 0
        && !(r.drift_fill != 0 && st >= 8)
        && (l == SNARE || l == KICK)
        && ((if l == SNARE {
            GHOST_SN[r.genre as usize]
        } else {
            GHOST_KK[r.genre as usize]
        }) as u32
            >> ls
            & 1)
            != 0
        && rng.fraction() * 100.0 < 0.6 * r.ghost as f32
    {
        hit = true;
        vel = 0.22 + 0.2 * rng.fraction();
    }
    if hit && r.probability < 100 && rng.fraction() * 100.0 >= r.probability as f32 {
        hit = false;
    }
    let mut result = Hit {
        step: ls,
        ..Hit::default()
    };
    if r.drop_impact != 0 && st == 0 && (l == KICK || l == CLAP || l == OHAT) {
        hit = true;
        vel = if l == KICK { 1.0 } else { 0.95 };
        result.crash = u32::from(l == OHAT);
    }
    result.kick = u32::from(l == KICK && hit);
    // fx bits are captured from stable firmware IDs by the C adapter, rather
    // than reusing the much wider synthesis FX mask as another wire format.
    if l == HAT && r.fx & 3 != 0 {
        hit = true;
        vel = if st & 1 != 0 { 0.5 } else { 0.65 };
        dbl = if r.fx & 2 != 0 { 2 } else { 1 };
    }
    if l == SNARE && r.fx & 4 != 0 {
        hit = true;
        vel = 0.3 + 0.045 * st as f32;
        dbl = if st & 1 != 0 { 1 } else { 0 };
    }
    if hit && (l == SNARE || l == CLAP) && r.fx & 8 != 0 {
        dbl = 3;
    }
    if hit && l == KICK && r.fx & 16 != 0 {
        dbl = 1;
    }
    if r.jam == 0
        && hit
        && r.section == GROOVE
        && r.fill_hold == 0
        && r.fill_bar == 0
        && !(r.drift_fill != 0 && st >= 8)
        && r.drop_impact == 0
    {
        let level = [1.0, 0.4, 0.7, 1.12][(r.modification >> 6 & 3) as usize];
        vel = (vel * (1.0 + (level - 1.0) * r.accent as f32 * 0.02)).clamp(0.05, 1.0);
        push = ((r.modification >> 3 & 7) as f32 * r.step_length * (1.0 / 12.0)) as i32;
        rat = (r.modification & 7) as usize;
    }
    if hit && r.human != 0 {
        let hm = r.human as f32 * 0.01;
        vel *= 1.0 - 0.2 * hm * rng.fraction();
        push += (rng.fraction() * hm * 0.12 * r.step_length) as i32;
    }
    result.hit = u32::from(hit);
    result.hat = u32::from(hit && (l == HAT || l == OHAT));
    result.perc = u32::from(hit && l == PERC);
    if !hit {
        return result;
    }
    vel *= if l == HAT || l == OHAT {
        r.hat_level
    } else {
        1.0
    };
    if r.jam != 0 {
        vel *= 0.65 + 0.005 * r.energy as f32;
    }
    let a = &mut result.actions;
    if dbl == 3 {
        a[0] = action(1, 0, vel * 0.4, 0.0);
        a[1] = action(2, (0.02 * 44_100.0) as i32, vel, 0.0);
        result.count = 2;
    } else if dbl != 0 {
        a[0] = action(1, 0, vel, 0.0);
        a[1] = action(
            2,
            (r.step_length * if dbl == 2 { 0.3333 } else { 0.5 }) as i32,
            vel * 0.7,
            0.0,
        );
        result.count = 2;
        if dbl == 2 {
            a[2] = action(3, (r.step_length * 0.6667) as i32, vel * 0.6, 0.0);
            result.count = 3;
        }
    } else if rat != 0 {
        const N: [u32; 8] = [0, 1, 2, 3, 5, 1, 2, 3];
        const DIV: [u32; 8] = [0, 2, 3, 4, 6, 0, 3, 4];
        const FIRST: [f32; 8] = [1.0, 1.0, 1.0, 1.0, 1.0, 0.4, 0.5, 0.4];
        const MUL: [f32; 8] = [1.0, 0.8, 0.8, 0.8, 0.88, 2.5, 1.4, 1.35];
        let v = vel * FIRST[rat];
        let gap = if DIV[rat] != 0 {
            (r.step_length / DIV[rat] as f32) as i32
        } else {
            (0.02 * 44_100.0) as i32
        };
        a[0] = action(16 + N[rat], gap, 0.0, MUL[rat]);
        if r.swing + push > 0 {
            a[1] = action(2, r.swing + push, v, 0.0);
            result.count = 2;
        } else {
            a[1] = action(1, 0, v, 0.0);
            a[2] = action(4, 0, v, 0.0);
            result.count = 3;
        }
    } else if r.chop > 0.0
        && r.swing == 0
        && push == 0
        && (l == SNARE || l == KICK || l == HAT)
        && rng.fraction() < 0.2 * r.chop
    {
        a[0] = action(1, 0, vel, 0.0);
        a[1] = action(2, (0.5 * r.step_length) as i32, vel * 0.7, 0.0);
        result.count = 2;
    } else {
        a[0] = action(
            if r.swing + push > 0 { 2 } else { 1 },
            r.swing + push,
            vel,
            0.0,
        );
        result.count = 1;
    }
    result
}

/// Earliest of the fixed sixteen delayed-hit slots (four melodic, two per
/// drum). No dynamic scheduled-event collection or allocation is introduced.
pub fn earliest(waits: &[i32; 16]) -> i32 {
    waits.iter().copied().filter(|n| *n > 0).min().unwrap_or(0)
}
pub fn advance_wait(wait: &mut i32, frames: i32) -> bool {
    if *wait <= 0 {
        return false;
    }
    *wait -= frames;
    if *wait <= 0 {
        *wait = 0;
        true
    } else {
        false
    }
}
pub fn ratchet_chain(
    remaining: u32,
    gap: i32,
    multiplier: f32,
    velocity: f32,
) -> Option<(u32, i32, f32)> {
    (remaining != 0).then(|| (remaining - 1, gap, velocity * multiplier))
}
pub fn song_pool(patterns: &[[u16; 6]; 6]) -> u32 {
    let mut count = 1;
    for p in 1..4 {
        if patterns[..p].contains(&patterns[p]) {
            break;
        }
        count += 1;
    }
    count
}
pub fn drift_length(song: bool, genre: usize, every: u32) -> u32 {
    const SONG: [u32; 19] = [
        16, 8, 8, 16, 16, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 16, 8, 16, 16,
    ];
    const LENGTH: [u32; 6] = [0, 4, 8, 16, 32, 64];
    if song {
        SONG[genre]
    } else {
        LENGTH.get(every as usize).copied().unwrap_or(0)
    }
}

pub fn mutate(pattern: &mut [u16; 6], morph: bool, rng: &mut Rng) {
    const LANES: [usize; 4] = [1, 3, 4, 5];
    for _ in 0..2 {
        let l = LANES[rng.index(4) as usize];
        let b = pattern[l] as u32;
        let keep = if l == 1 { 0x1111 } else { 0 };
        let mut on = [0u32; 16];
        let mut count = 0;
        for step in 0..16 {
            if (b & !keep) >> step & 1 != 0 {
                on[count] = step;
                count += 1;
            }
        }
        if count != 0 {
            let k = on[rng.index(count as u32) as usize];
            let to = (k + if rng.fraction() < 0.5 { 15 } else { 1 }) & 15;
            if b >> to & 1 == 0 {
                pattern[l] = ((b & !(1 << k)) | 1 << to) as u16;
            }
        }
    }
    if morph {
        let l = LANES[rng.index(4) as usize];
        let mut b = pattern[l] as u32;
        let h = b.count_ones();
        let free = b & !if l == 1 { 0x1111 } else { 0 };
        if ((rng.fraction() < 0.5 && h > 1) || h >= 10) && free != 0 {
            let mut step = rng.index(16);
            while free >> step & 1 == 0 {
                step = (step + 1) & 15;
            }
            b &= !(1 << step);
        } else if h < 10 {
            let mut step = rng.index(16);
            while b >> step & 1 != 0 {
                step = (step + 1) & 15;
            }
            b |= 1 << step;
        }
        pattern[l] = b as u16;
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Drift {
    pub countdown: i32,
    pub bars: u32,
    pub fill: u32,
    pub left: i32,
    pub next: i32,
    pub changed: u32,
    pub mutation: u32,
}
const _: [(); 28] = [(); core::mem::size_of::<Drift>()];
const _: [(); 4] = [(); core::mem::align_of::<Drift>()];
pub fn drift(
    state: &mut Drift,
    length: u32,
    change: u32,
    mutation: u32,
    pool: u32,
    current: i32,
    rng: &mut Rng,
) {
    state.changed = 0;
    state.mutation = 0;
    if change == 0 && mutation == 0 {
        state.countdown = -1;
        state.left = -1;
        state.fill = 0;
        state.bars = 0;
        return;
    }
    if state.countdown > 0 {
        state.countdown -= 1;
    }
    if state.countdown == 0 {
        let mut next = current;
        if change == 1 {
            next = if current + 1 >= pool as i32 {
                0
            } else {
                current + 1
            };
        } else if change == 2 {
            loop {
                next = rng.index(pool) as i32;
                if next != current {
                    break;
                }
            }
        }
        if change != 0 {
            state.next = next;
            state.changed = 1;
        }
        state.mutation = u32::from(mutation != 0);
        state.countdown = -1;
        state.bars = 0;
    } else if length != 0 && state.countdown < 0 {
        state.bars = state.bars.wrapping_add(1);
        if state.bars >= length - 1 {
            state.countdown = 1;
        }
    }
    state.fill = u32::from(state.countdown == 1 && change != 0);
    state.left = if state.countdown >= 0 {
        state.countdown
    } else if length != 0 {
        length as i32 - state.bars as i32
    } else {
        -1
    };
}

/// Songbook section action. Copy/activation and synthesis remain caller actions,
/// but Hold, queued edits and upload priority are a single portable decision.
pub fn phrase_boundary(position: u32, bars: u32, upload: bool, hold: bool, pending: bool) -> u32 {
    if position < bars * 16 {
        0
    } else if upload {
        1
    } else if !hold || pending {
        2
    } else {
        3
    }
}

pub fn time_mode(player: u32, genre_time: u32, section: i32, groove: u32, build_left: i32) -> u32 {
    if player != 0 {
        player
    } else if genre_time != 0
        && ((section == GROOVE && groove > genre_time && groove <= 2 * genre_time)
            || (section == BUILD && build_left <= 2))
    {
        2
    } else {
        0
    }
}

pub fn fill_kind(allowed: u32, bars: u32, genre: u32) -> u32 {
    let mut k = bars.wrapping_add(genre * 2) % 8;
    for _ in 0..8 {
        if allowed >> k & 1 != 0 {
            break;
        }
        k = (k + 1) % 8;
    }
    k
}
pub fn chain(chain: u32, index: u32) -> (u32, u32) {
    const CHAIN: [[u32; 7]; 9] = [
        [2, 1, 2, 0, 0, 0, 0],
        [4, 1, 1, 1, 2, 0, 0],
        [4, 1, 1, 2, 3, 0, 0],
        [4, 1, 1, 3, 4, 0, 0],
        [4, 1, 2, 1, 3, 0, 0],
        [4, 1, 2, 3, 4, 0, 0],
        [4, 1, 2, 3, 2, 0, 0],
        [4, 1, 1, 1, 5, 0, 0],
        [6, 1, 2, 3, 4, 5, 6],
    ];
    let row = CHAIN[(chain - 1) as usize];
    (row[1 + (index % row[0]) as usize] - 1, (index + 1) % row[0])
}
pub fn slice(request: u32, bars: u32, rng: &mut Rng) -> u32 {
    if request == 9 {
        if bars % 4 == 3 {
            1 + rng.index(8)
        } else {
            0
        }
    } else {
        request
    }
}

pub fn section(
    command: u32,
    current: i32,
    degree: i32,
    nsteps: i32,
    evolve: f32,
    rng: &mut Rng,
) -> u32 {
    if command == 0 {
        return BREAK as u32;
    }
    if command == 1 {
        return if current == BREAK {
            BUILD as u32 | 0x100
        } else {
            current as u32
        };
    }
    if current == GROOVE {
        return current as u32;
    }
    let mut result = GROOVE as u32 | 0x200;
    if degree != 0 && nsteps > 1 && rng.fraction() < 0.7 {
        result |= 0x400 | 0x800;
    } else if rng.fraction() < 0.4 + 0.5 * evolve {
        result |= 0x800;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(mut h: u32, value: u32) -> u32 {
        for b in value.to_le_bytes() {
            h = (h ^ b as u32).wrapping_mul(16_777_619);
        }
        h
    }
    #[test]
    fn all_masks_and_densities_match_the_frozen_c_oracle() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../tests/sequencer-density.json")).unwrap();
        for (amount, expected) in corpus.as_array().unwrap().iter().enumerate() {
            let mut h = 2_166_136_261;
            for mask in 0..=u16::MAX {
                h = word(h, density(mask, amount as i32) as u32);
            }
            assert_eq!(h, expected.as_u64().unwrap() as u32, "density {amount}");
        }
    }
    #[test]
    fn shift_morph_and_every_rng_word_match_frozen_c() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../tests/sequencer-mutation.json")).unwrap();
        for mode in 0..2 {
            let mut h = 2_166_136_261;
            for index in 0u32..4096 {
                let mut rng = Rng((index + 1).wrapping_mul(2_654_435_761));
                let mut pattern = core::array::from_fn(|lane| {
                    (index
                        .wrapping_mul(1_103_515_245)
                        .wrapping_add((lane as u32).wrapping_mul(1_013_904_223))
                        >> ((lane as u32 + index) % 16)) as u16
                });
                let before = pattern;
                mutate(&mut pattern, mode == 1, &mut rng);
                assert_eq!(pattern[0], before[0]);
                assert_eq!(pattern[2], before[2]);
                assert_eq!(pattern[1] & before[1] & 0x1111, before[1] & 0x1111);
                for mask in pattern {
                    h = word(h, mask as u32);
                }
                h = word(h, rng.0);
            }
            assert_eq!(h, corpus[mode].as_u64().unwrap() as u32);
        }
    }
    #[test]
    fn full_modification_space_fits_three_actions_and_fixed_wait_slots() {
        for lane_id in 0..6 {
            for modification in 0..256 {
                for step in 0..16 {
                    let request = Lane {
                        lane: lane_id,
                        mask: 65535,
                        sixteenth: step,
                        bars: 1,
                        length: 16,
                        section_length: 4,
                        modification,
                        probability: 100,
                        accent: 100,
                        human: 100,
                        step_length: 5512.5,
                        hat_level: 1.0,
                        ..Lane::default()
                    };
                    let mut rng = Rng(91);
                    let hit = lane(&request, &mut rng);
                    assert!(hit.count <= 3 && hit.hit != 0);
                    for event in &hit.actions[..hit.count as usize] {
                        assert!(event.velocity.is_finite());
                        if event.kind == 2 || event.kind == 3 {
                            assert!(event.wait > 0);
                        }
                    }
                }
            }
        }
        let mut waits = [0; 16];
        waits[0] = -4;
        waits[3] = 5;
        waits[15] = 1;
        assert_eq!(earliest(&waits), 1);
        assert!(advance_wait(&mut waits[15], 128));
        assert_eq!(waits[15], 0);
        assert_eq!(earliest(&waits), 5);
        assert!(advance_wait(&mut waits[3], 5));
        assert_eq!(earliest(&waits), 0);
    }
    #[test]
    fn phrase_boundary_prioritizes_upload_and_preserves_hold() {
        for hold in [false, true] {
            for pending in [false, true] {
                for upload in [false, true] {
                    assert_eq!(phrase_boundary(127, 8, upload, hold, pending), 0);
                    assert_eq!(
                        phrase_boundary(128, 8, upload, hold, pending),
                        if upload {
                            1
                        } else if !hold || pending {
                            2
                        } else {
                            3
                        }
                    );
                }
            }
        }
    }
}
