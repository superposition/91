// SPDX-License-Identifier: GPL-3.0-only
//! Small phase transitions: no voice callback can run inside a state borrow.
//! The caller publishes state, executes its returned action, then captures the
//! callback's resulting RNG/state before advancing the next phase.
use super::{
    abi::{Config, State},
    basic,
    tables::*,
};
use crate::{effects::math, sequencer::Rng};

pub fn drift_seconds(value: i32) -> f32 {
    20.0 * math::exp2(value as f32 * 0.039069)
}
pub fn genre_bars(genre: usize, unit: u8, value: i32) -> i32 {
    let mut b = GRAPHS[genre].bars as i32;
    if unit == 2 || (b == 0 && unit != 1) {
        return 0;
    }
    if b == 0 {
        b = 4;
    }
    b = if value < 20 {
        b >> 2
    } else if value < 45 {
        b >> 1
    } else if value > 95 {
        b << 2
    } else if value > 80 {
        b << 1
    } else {
        b
    };
    b.max(1)
}
pub fn bars(s: &State, c: &Config) -> i32 {
    meter_bars(
        c.genre as usize,
        s.drift_unit,
        c.drift,
        s.pg_on != 0,
        s.pg_bars[s.pg_i as usize],
    )
}
pub fn meter_bars(genre: usize, unit: u8, value: i32, playing: bool, log: u8) -> i32 {
    if playing && unit != 2 {
        1 << log
    } else {
        genre_bars(genre, unit, value)
    }
}
pub fn scale_set(s: &mut State) {
    let scale = basic::scale(s.pool);
    s.nsteps = scale.count as i32;
    s.steps[..scale.count].copy_from_slice(&scale.steps[..scale.count]);
    s.steps_nat = s.steps;
    s.dom_on = 0;
    s.en_chord_alt = 0;
    s.walk_next = -1;
    s.en_walk_next = -1;
}
pub fn natural(s: &State, k: usize) -> i32 {
    s.steps[k % 7] - if k % 7 == 6 { s.dom_on as i32 } else { 0 }
}
pub fn diminished(s: &State, i: usize) -> bool {
    s.nsteps == 7 && (natural(s, i + 4) - natural(s, i) + 12) % 12 == 6
}
pub fn dominant(s: &mut State, c: &Config) {
    if s.nsteps != 7 {
        return;
    }
    s.steps[6] -= s.dom_on as i32;
    s.dom_on = u8::from(
        GRAPHS[c.genre as usize].dominant != 0
            && s.deg == 4
            && s.steps[6] - s.steps[4] == 3
            && s.steps[6] < 11,
    );
    s.steps[6] += s.dom_on as i32;
}
pub fn preview(s: &State, c: &Config, degree: i32, kind: i32) -> basic::Chord {
    let mut chord = basic::chord(
        s.pool,
        c.root,
        degree,
        if kind != 0 { kind } else { c.chord_kind },
    );
    if s.nsteps == 7
        && GRAPHS[c.genre as usize].dominant != 0
        && degree % 7 == 4
        && natural(s, 6) - natural(s, 4) == 3
        && natural(s, 6) < 11
    {
        let lt = (c.root + natural(s, 6)) % 12;
        for pc in &mut chord.pcs[..chord.count] {
            if *pc == lt {
                *pc = (lt + 1) % 12;
            }
        }
    }
    chord
}
pub fn reset_drift(s: &mut State, c: &Config, rng: &mut Rng) {
    s.hr_left = bars(s, c);
    s.drift_total = if s.hr_left != 0 {
        s.hr_left as f32 * 16.0 * c.step_length
    } else {
        drift_seconds(c.drift) * (0.7 + 0.6 * rng.fraction()) * 44100.0
    };
    s.drift_left = s.drift_total;
    s.fifth_done = 0;
}
pub fn clear_alterations(s: &mut State) {
    let n = s.nsteps as usize;
    s.steps[..n].copy_from_slice(&s.steps_nat[..n]);
    s.dom_on = 0;
    s.en_chord_alt = 0;
}
fn fixed(c: &Config) -> bool {
    c.chord_kind == 5 || c.chord_kind == 6
}
fn percent(v: u8) -> i32 {
    (v as i32 * 3 / 2).min(60)
}
fn borrow_key(s: &State, c: &Config) -> bool {
    s.nsteps == 7
        && c.genre != 0
        && percent(BORROW[c.genre as usize]) != 0
        && s.steps_nat[2] == 4
        && !fixed(c)
}
fn borrow_vii(s: &State, c: &Config, i: usize) -> bool {
    i == 6 && borrow_key(s, c)
}
pub fn borrow_decision(s: &State, c: &Config, i: usize, rng: &mut Rng) -> bool {
    borrow_key(s, c)
        && (i == 6
            || ((i == 3 || i == 5)
                && rng.fraction() * 100.0 < (percent(BORROW[c.genre as usize]) as f32)))
}
pub fn alter(
    s: &mut State,
    c: &Config,
    i: usize,
    borrowed: bool,
    next: i32,
    rng: &mut Rng,
) -> bool {
    if s.nsteps != 7 || c.genre == 0 || fixed(c) {
        return false;
    }
    let mut changed = false;
    if borrowed {
        for (a, b) in s.steps.iter_mut().zip(AEOLIAN) {
            *a = b as i32;
        }
        s.dom_on = 0;
        s.en_chord_alt = 2;
        changed = true;
    } else if (1..7).contains(&next)
        && next as usize != i
        && percent(SECONDARY[c.genre as usize]) != 0
        && (s.steps_nat[i] - s.steps_nat[next as usize] + 12) % 12 == 7
        && (s.steps_nat[(i + 4) % 7] - s.steps_nat[i] + 12) % 12 == 7
        && rng.fraction() * 100.0 < (percent(SECONDARY[c.genre as usize]) as f32)
    {
        let t = (i + 2) % 7;
        let seventh = (i + 6) % 7;
        if (s.steps[t] - s.steps_nat[i] + 12) % 12 == 3 {
            s.steps[t] += 1;
            changed = true;
        }
        if (s.steps[seventh] - s.steps_nat[i] + 12) % 12 == 11 {
            s.steps[seventh] -= 1;
            changed = true;
        }
        if changed {
            s.en_chord_alt = 1;
        }
    }
    changed
}
pub fn walk_ok(s: &State, c: &Config, i: usize) -> bool {
    !(i != 0 && diminished(s, i)) || borrow_vii(s, c, i)
}
pub fn walk(s: &mut State, c: &Config, rng: &mut Rng) -> usize {
    const W7: [f32; 7] = [4.0, 2.0, 1.0, 3.0, 3.0, 3.0, 0.4];
    let mut weights = [0.0; 12];
    let mut sum = 0.0;
    let graph = &GRAPHS[c.genre as usize];
    let rich = s.nsteps == 7 && c.genre != 0;
    let mut lead = 0;
    if rich {
        lead = graph.lead as usize;
        if diminished(s, lead) {
            lead = if diminished(s, 4) { 3 } else { 4 };
        }
    }
    for (i, w) in weights.iter_mut().enumerate().take(s.nsteps as usize) {
        if i == s.deg as usize {
            *w = 0.0;
        } else if rich {
            let edges = if c.genre == 13 && natural(s, 2) == 4 {
                &SYNTH_MAJOR
            } else {
                &graph.edges
            };
            *w = 1.0 + if i == 0 { graph.tonic as f32 } else { 0.0 };
            for e in edges {
                if e[2] != 0
                    && e[0] as i32 == s.deg
                    && e[1] as usize == i
                    && (e[2] & 128 == 0 || natural(s, 1) == 1)
                {
                    *w += (e[2] & 127) as f32;
                }
            }
            if graph.back == 0
                && i == s.deg_prev as usize
                && s.deg_prev as i32 != s.deg
                && s.cad_n != 0
            {
                *w = 0.0;
            }
            if percent(SECONDARY[c.genre as usize]) != 0
                && i != 0
                && (s.steps_nat[s.deg as usize] - s.steps_nat[i] + 12) % 12 == 7
            {
                *w += percent(SECONDARY[c.genre as usize]) as f32 * 0.1;
            }
            if s.cad_n as i32 + 1 >= graph.cadence as i32 && i == lead && s.deg != 0 {
                *w += 10.0;
            }
            if s.cad_n >= graph.cadence && i == 0 {
                *w += 30.0;
            }
        } else {
            *w = if s.nsteps == 7 {
                W7[i]
            } else if i == 0 {
                3.0
            } else {
                1.5
            };
        }
        if i != 0 && diminished(s, i) && !borrow_vii(s, c, i) {
            *w = 0.0;
        }
        sum += *w;
    }
    let mut r = rng.fraction() * sum;
    let mut selected = s.nsteps as usize;
    for (i, w) in weights.iter().enumerate().take(s.nsteps as usize) {
        r -= *w;
        if r <= 0.0 && *w > 0.0 {
            selected = i;
            break;
        }
    }
    if selected >= s.nsteps as usize {
        selected = if s.deg == 0 {
            usize::from(s.nsteps > 1)
        } else {
            0
        };
    }
    let lp = c.loop_mode;
    let len = if lp != 0 { 2 << ((lp - 1) % 3) } else { 0 };
    let varied = lp > 3;
    if len != s.dh_ln as i32 {
        s.dh_ln = len as u8;
        s.dh_n = 0;
        s.dh_i = 0;
    }
    if len != 0 && s.dh_n == 0 {
        s.dh[s.dh_n as usize] = s.deg as u8;
        s.dh_n += 1;
    }
    if len != 0 && s.dh_n as i32 >= len {
        let k = s.dh_i as usize % len as usize;
        s.dh_i = (k + 1) as u8;
        if varied && rng.fraction() * 100.0 < 25.0 {
            s.dh[k] = selected as u8;
        } else if (s.dh[k] as i32) < s.nsteps && walk_ok(s, c, s.dh[k] as usize) {
            selected = s.dh[k] as usize;
        }
    } else if len != 0 {
        s.dh[s.dh_n as usize] = selected as u8;
        s.dh_n += 1;
        s.dh_i = 0;
    }
    selected
}
pub fn chord_header(s: &mut State, c: &Config, degree: i32) -> bool {
    s.deg_prev = s.deg as u8;
    s.cad_n = if degree == 0
        && s.nsteps == 7
        && c.genre != 0
        && s.cad_n >= GRAPHS[c.genre as usize].cadence
    {
        0
    } else {
        s.cad_n.wrapping_add(1)
    };
    s.deg = degree;
    if s.hist_n >= 8 {
        for k in 0..7 {
            s.hist[k] = s.hist[k + 1];
        }
    }
    let at = if s.hist_n < 8 {
        let at = s.hist_n;
        s.hist_n += 1;
        at
    } else {
        7
    };
    s.hist[at as usize] = degree as u8;
    dominant(s, c);
    c.trigger == 0
}
/// Returns 1+index*256 for a pending arc (caller also dispatches both bings),
/// 2+index*256 for rotation, or zero. Queue/voice callbacks run after return.
pub fn arc(s: &mut State) -> u32 {
    if s.naiwan_pending >= 0 {
        let index = s.naiwan_pending;
        s.naiwan_pending = -1;
        s.naiwan_rotate = 1;
        s.naiwan_arc = index as u8;
        1 | ((index as u32) << 8)
    } else if s.naiwan_rotate != 0 && s.naiwan_left != 0 {
        s.naiwan_left -= 1;
        if s.naiwan_left == 0 {
            s.naiwan_arc = (s.naiwan_arc + 1) % 10;
            return 2 | ((s.naiwan_arc as u32) << 8);
        }
        0
    } else {
        0
    }
}
/// Action 1=direct authored chord, 2=empty-slot reset, 3=walk chord. Degree is
/// bits8..15, borrowed bit16 and host diminished diagnostic bit17.
pub fn begin_change(s: &mut State, c: &Config, rng: &mut Rng) -> u32 {
    clear_alterations(s);
    if s.pg_on != 0 {
        s.pg_i = if s.drop_tonic != 0 || s.pg_start != 0 {
            0
        } else {
            (s.pg_i + 1) % s.pg_len
        };
        s.drop_tonic = 0;
        s.pg_start = 0;
        s.en_prog_pos = s.pg_i as i8;
        s.walk_next = -1;
        s.en_walk_next = -1;
        let degree = s.pg_deg[s.pg_i as usize];
        if degree != 0 {
            let i = (degree as i32 - 1) % s.nsteps;
            return 1 | ((i as u32) << 8) | (u32::from(i != 0 && diminished(s, i as usize)) << 17);
        }
        return 2;
    }
    let (mut i, mut borrowed) = if s.walk_next >= 0
        && (s.walk_next as i32) < s.nsteps
        && walk_ok(s, c, s.walk_next as usize)
    {
        (s.walk_next as usize, s.walk_bor != 0)
    } else {
        let i = walk(s, c, rng);
        (i, borrow_decision(s, c, i, rng))
    };
    if s.drop_tonic != 0 {
        s.drop_tonic = 0;
        i = 0;
        borrowed = false;
    }
    3 | ((i as u32) << 8) | (u32::from(borrowed) << 16)
}
pub fn finish_change(
    s: &mut State,
    c: &Config,
    degree: usize,
    borrowed: bool,
    rng: &mut Rng,
) -> bool {
    s.walk_next = walk(s, c, rng) as i8;
    s.walk_bor = u8::from(borrow_decision(s, c, s.walk_next as usize, rng));
    s.en_walk_next_root = ((c.root
        + if s.walk_bor != 0 {
            AEOLIAN[s.walk_next as usize % 7] as i32
        } else {
            s.steps_nat[s.walk_next as usize]
        })
        % 12) as u8;
    s.en_walk_next = s.walk_next;
    alter(s, c, degree, borrowed, s.walk_next as i32, rng)
}

pub fn pool_set(s: &mut State, c: &Config, pool: u32) {
    s.pool = (pool | 1) & 0xfff;
    scale_set(s);
    if s.deg >= s.nsteps {
        s.deg = 0;
    }
    dominant(s, c);
}
pub fn regrow(s: &mut State, c: &Config) {
    s.pg_i = 0;
    s.pg_start = 0;
    s.deg = if s.pg_on != 0 && s.pg_deg[0] != 0 {
        (s.pg_deg[0] as i32 - 1) % if s.nsteps != 0 { s.nsteps } else { 7 }
    } else {
        0
    };
    s.en_prog_pos = if s.pg_on != 0 { 0 } else { -1 };
    s.dh_n = 0;
    s.dh_i = 0;
    s.hist_n = 0;
    dominant(s, c);
}
pub fn edit_drift(s: &mut State, c: &Config) {
    let b = bars(s, c);
    if b != 0 {
        if s.hr_left >= b {
            s.hr_left = b - 1;
        }
    } else if s.drift_total > 0.0 {
        let fraction = s.drift_left / s.drift_total;
        s.drift_total = drift_seconds(c.drift) * 44100.0;
        s.drift_left = fraction * s.drift_total;
    }
}
/// Returns original Play bit0, requested unit bits8..9 and immediate-change
/// bit16. The caller must finish AFTER executing that immediate chord callback.
pub fn take_begin(s: &mut State, c: &Config, lo: u32, hi: u32) -> u32 {
    let was = s.pg_on;
    let unit = (hi >> 17 & 3) as u8;
    s.pg_len = 0;
    for slot in 0..8 {
        s.pg_deg[slot] = (lo >> (3 * slot) & 7) as u8;
        s.pg_bars[slot] = (hi >> (2 * slot) & 3) as u8;
        if s.pg_deg[slot] != 0 {
            s.pg_len = (slot + 1) as u8;
        }
    }
    s.pg_on = u8::from(hi >> 16 & 1 != 0 && s.pg_len != 0);
    if s.pg_i >= s.pg_len {
        s.pg_i = 0;
    }
    let mut immediate = false;
    if s.pg_on != 0 && was == 0 {
        s.pg_start = 1;
        if c.playing != 0 {
            s.change_pending = 1;
        } else {
            immediate = true;
        }
    } else if s.pg_on != 0 {
        let b = bars(s, c);
        if s.hr_left >= b {
            s.hr_left = b - 1;
        }
    }
    was as u32 | ((unit as u32) << 8) | (u32::from(immediate) << 16)
}
pub fn take_finish(s: &mut State, token: u32) -> bool {
    s.en_prog_pos = if s.pg_on != 0 { s.pg_i as i8 } else { -1 };
    let was = token & 1 != 0;
    let unit = (token >> 8 & 3) as u8;
    if (s.pg_on == 0 && was) || unit != s.drift_unit {
        s.drift_unit = unit;
        true
    } else {
        false
    }
}
/// Preserve Play/unit/extension byte bits that the C capture never overwrites.
pub fn capture(s: &State, c: &Config, bytes: [u8; 6]) -> [u8; 6] {
    let mut progression = fm1_types::Progression::from_bytes(bytes);
    let mut degrees = [0; 8];
    let count;
    if s.dh_ln != 0 && s.dh_n >= s.dh_ln {
        count = s.dh_ln as usize;
        degrees[..count].copy_from_slice(&s.dh[..count]);
    } else {
        let n = s.hist_n.min(4) as usize;
        if n == 0 {
            degrees[0] = s.deg as u8;
            count = 1;
        } else {
            degrees[..n].copy_from_slice(&s.hist[s.hist_n as usize - n..s.hist_n as usize]);
            count = n;
        }
    }
    let bars = genre_bars(c.genre as usize, s.drift_unit, c.drift);
    let bars = if bars != 0 { bars } else { 4 };
    let mut log = 0;
    while log < 3 && (1 << log) < bars {
        log += 1;
    }
    for (slot, degree) in degrees.iter().enumerate() {
        let degree = if slot < count {
            degree.wrapping_add(1) & 7
        } else {
            0
        };
        progression.set_degree(slot, degree).unwrap();
        progression.set_bars_log2(slot, log as u8).unwrap();
    }
    progression.bytes()
}
fn register(c: &Config) -> (i32, i32) {
    let shift = 12 * (c.pad_oct + c.song_oct);
    let mut lo = 48 + shift;
    let mut hi = 77 + shift;
    if lo < 24 {
        let shift = (24 - lo + 11) / 12 * 12;
        lo += shift;
        hi += shift;
    }
    if hi > 108 {
        let shift = (hi - 108 + 11) / 12 * 12;
        lo -= shift;
        hi -= shift;
    }
    (lo, hi)
}
pub fn targets(s: &mut State, c: &Config, from: [f32; 4]) -> super::abi::Targets {
    let kind = if s.pg_on != 0 && s.pg_kind[s.pg_i as usize] != 0 {
        s.pg_kind[s.pg_i as usize] as i32
    } else {
        c.chord_kind
    };
    let chord = basic::chord(s.pool, c.root, s.deg, kind);
    s.chord_nt = chord.count as i32;
    s.chord_pcs[..chord.count].copy_from_slice(&chord.pcs[..chord.count]);
    for pc in &mut s.chord_pcs[..chord.count] {
        for j in 0..s.nsteps as usize {
            if *pc == (c.root + s.steps_nat[j]) % 12 {
                *pc = (c.root + s.steps[j]) % 12;
                break;
            }
        }
    }
    let from = from.map(|note| (note + 0.5) as i32);
    let (lo, hi) = register(c);
    let mut to = basic::voice_lead(from, &s.chord_pcs, s.chord_nt, lo, hi, [-1; 4]);
    if c.voicing != 0 {
        let mut highest = 0;
        for i in 1..4 {
            if to[i] > to[highest] {
                highest = i;
            }
        }
        let mut second = None;
        for i in 0..4 {
            if i != highest
                && match second {
                    None => true,
                    Some(b) => to[i] > to[b],
                }
            {
                second = Some(i);
            }
        }
        let second = second.unwrap();
        if to[second] - 12 >= lo - 7 {
            to[second] -= 12;
        }
    }
    let display = to;
    if c.pedal & 2 != 0 {
        let chord = basic::chord(s.pool, c.root, 0, kind);
        to = basic::voice_lead(from, &chord.pcs, chord.count as i32, lo, hi, [-1; 4]);
    }
    super::abi::Targets {
        target: to.map(|note| note as f32),
        display,
    }
}
pub fn mood(
    s: &mut super::abi::Mood,
    n: u32,
    playing: bool,
    motion: f32,
    section: i32,
    rng: &mut Rng,
) {
    if s.arc_left <= n {
        let r = rng.fraction() * 2.0 - 1.0;
        s.arc_tgt = r * (0.6 + 0.4 * rng.fraction()) + if r > 0.0 { 0.25 } else { -0.25 };
        s.arc_left = ((50.0 + 100.0 * rng.fraction()) * 44100.0) as u32;
    } else {
        s.arc_left -= n;
    }
    if playing {
        s.arc += (math::clamp(s.arc_tgt, -1.0, 1.0) - s.arc) * n as f32 * (1.0 / (35.0 * 44100.0));
    }
    s.arcm = s.arc * motion;
    let target = if section == 1 {
        1.0
    } else if section == 2 {
        -0.6
    } else {
        0.0
    };
    s.brk += (target - s.brk)
        * math::min(
            n as f32 / ((if section == 2 { 4.0 } else { 2.0 }) * 44100.0),
            1.0,
        );
}
