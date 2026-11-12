// SPDX-License-Identifier: GPL-3.0-only
//! Kick Runner's fixed state and decisions. Platform effects happen after return.
use fm1_engine::effects::math::{floor, min};

pub const ROWS: usize = 16;
pub const LANES: u8 = 8;
const PARTS: [&[u8]; 5] = [b"HATS", b"SNARE", b"ACID", b"DRONE", b"PAD"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    State,
    Index,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Row {
    pub beat: f32,
    pub wall: u8,
    pub gate: u8,
    pub done: u8,
    pub taken: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub on: u8,
    pub over: u8,
    pub lane: u8,
    pub layer: u8,
    pub ships: u8,
    pub reserved: [u8; 3],
    pub start_beat: f32,
    pub view: f32,
    pub beat: f32,
    pub s16: u32,
    pub s16_ms: u32,
    pub step_ms: u32,
    pub kick_prev: f32,
    pub kick: f32,
    pub lp_until: f32,
    pub flash: f32,
    pub crash: f32,
    pub score: u32,
    pub best: u32,
    pub gates: u32,
    pub crashes: u32,
    pub rows_born: u32,
    pub banner_until: u32,
    pub start_ms: u32,
    pub over_ms: u32,
    pub rng: u32,
    pub banner: [u8; 20],
    pub row: [Row; ROWS],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    pub now: u32,
    pub bars: u32,
    pub playing: u32,
    pub kick: f32,
}
impl State {
    pub fn restart_candidate(&self, keys: u32, previous: u32) -> bool {
        keys & !previous != 0 && self.over != 0
    }
    pub fn restart(&mut self, now: u32) -> bool {
        if now.wrapping_sub(self.over_ms) > 900 {
            self.reset();
            true
        } else {
            false
        }
    }
    /// Play and exit are actions; the caller dispatches them after this borrow.
    pub fn buttons(pressed: u32, held: u32, masks: [u32; 4]) -> u32 {
        u32::from(pressed & masks[0] != 0)
            | (u32::from(
                pressed & masks[1] != 0 || (pressed & masks[2] != 0 && held & masks[3] != 0),
            ) << 1)
    }
    pub fn valid(&self) -> bool {
        self.on <= 1
            && self.over <= 1
            && self.lane < LANES
            && self.layer <= 5
            && self.ships <= 3
            && [
                self.start_beat,
                self.view,
                self.beat,
                self.kick_prev,
                self.kick,
                self.lp_until,
                self.flash,
                self.crash,
            ]
            .iter()
            .all(|v| v.is_finite())
            && self
                .row
                .iter()
                .all(|r| r.beat.is_finite() && r.done <= 1 && r.taken <= 1)
    }
    fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }
    fn fraction(&mut self) -> f32 {
        (self.random() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
    pub fn say(&mut self, text: &[u8]) {
        let n = text
            .iter()
            .position(|v| *v == 0)
            .unwrap_or(text.len())
            .min(19);
        self.banner[..n].copy_from_slice(&text[..n]);
        self.banner[n] = 0; // Preserve the legacy unused tail.
    }
    /// Caller timestamps the new banner after this borrow ends.
    pub fn reset(&mut self) {
        self.row.fill(Row::default());
        self.over = 0;
        self.layer = 0;
        self.ships = 3;
        self.start_beat = self.beat;
        self.score = 0;
        self.gates = 0;
        self.crashes = 0;
        self.lp_until = 0.0;
        self.flash = 0.0;
        self.crash = 0.0;
        self.say(b"KICK RUNNER");
    }
    pub fn start(&mut self, clock: Observation, times: [u32; 3]) {
        self.on = 1;
        self.rng ^= clock.now.wrapping_mul(2_654_435_761) | 1;
        self.lane = 0;
        self.view = 0.25 - 0.5 / f32::from(LANES);
        self.s16 = clock.bars;
        self.s16_ms = times[0];
        if self.step_ms == 0 {
            self.step_ms = 125;
        }
        self.beat = self.s16 as f32 * 0.25;
        self.start_ms = times[1];
        self.reset();
        self.banner_until = times[2].wrapping_add(2500);
    }
    fn spawn(&mut self) {
        let at = floor(self.beat + 0.5) + 8.0;
        let mut slot = None;
        for (i, row) in self.row.iter().enumerate() {
            if row.beat == at && row.wall | row.gate != 0 {
                return;
            }
            if slot.is_none() && (row.done != 0 || row.wall | row.gate == 0) {
                slot = Some(i);
            }
        }
        let Some(slot) = slot else {
            return;
        };
        let (mut wall, mut gate) = (0u8, 0u8);
        if at - 8.0 >= self.start_beat + 8.0
            && self.fraction() < 0.42 + 0.06 * f32::from(self.layer)
        {
            let width = (1 + (self.fraction() * (1.0 + 0.5 * f32::from(self.layer))) as i32).min(3);
            let start = (self.fraction() * f32::from(LANES)) as i32;
            for i in 0..width {
                wall |= 1 << ((start + i) % i32::from(LANES));
            }
        }
        if self.fraction() < 0.34 {
            for _ in 0..6 {
                let lane = (i32::from(self.lane) + (self.fraction() * 5.0) as i32 - 2
                    + i32::from(LANES))
                    % i32::from(LANES);
                if wall >> lane & 1 == 0 {
                    gate = 1 << lane;
                    break;
                }
            }
        }
        if wall | gate == 0 {
            return;
        }
        self.row[slot] = Row {
            beat: at,
            wall,
            gate,
            done: 0,
            taken: 0,
        };
        self.rows_born = self.rows_born.wrapping_add(1);
    }
    /// The clock/kick phase precedes row resolution, like the original frame.
    pub fn advance(&mut self, clock: Observation) -> Result<(), Error> {
        if !self.valid() || !clock.kick.is_finite() {
            return Err(Error::State);
        }
        if clock.bars != self.s16 {
            let elapsed = clock.now.wrapping_sub(self.s16_ms);
            let steps = clock.bars.wrapping_sub(self.s16);
            if steps != 0 && steps < 8 && elapsed > 20 && elapsed < 2000 {
                self.step_ms = self.step_ms.wrapping_mul(3).wrapping_add(elapsed / steps) / 4;
            }
            self.s16 = clock.bars;
            self.s16_ms = clock.now;
        }
        let fraction = if clock.playing != 0 {
            min(
                clock.now.wrapping_sub(self.s16_ms) as f32
                    / (if self.step_ms != 0 { self.step_ms } else { 125 }) as f32,
                0.999,
            )
        } else {
            0.0
        };
        let beat = (self.s16 as f32 + fraction) * 0.25;
        if beat > self.beat || beat < self.beat - 4.0 {
            self.beat = beat;
        }
        let want = 0.25 - (f32::from(self.lane) + 0.5) / f32::from(LANES);
        let mut difference = want - self.view;
        difference -= floor(difference + 0.5);
        self.view += difference * 0.35;
        if clock.kick > 0.8 && self.kick_prev < 0.6 {
            self.kick = 1.0;
            if clock.playing != 0 && self.over == 0 {
                self.spawn();
            }
        }
        self.kick_prev = clock.kick;
        self.kick *= 0.82;
        self.flash *= 0.85;
        self.crash *= 0.9;
        Ok(())
    }
    /// Return true for a banner; caller samples its timestamp after this borrow.
    pub fn resolve(&mut self, index: usize, now: u32) -> Result<bool, Error> {
        if index >= ROWS {
            return Err(Error::Index);
        }
        if self.lane >= LANES || self.layer > 5 {
            return Err(Error::State);
        }
        let row = &mut self.row[index];
        if row.done != 0 || row.wall | row.gate == 0 || self.beat < row.beat {
            return Ok(false);
        }
        row.done = 1;
        if self.over != 0 {
            return Ok(false);
        }
        if row.wall >> self.lane & 1 != 0 {
            self.crashes = self.crashes.wrapping_add(1);
            self.crash = 1.0;
            self.lp_until = self.beat + 4.0;
            if self.layer == 0 {
                if self.ships != 0 {
                    self.ships -= 1;
                }
                if self.ships == 0 {
                    self.over = 1;
                    self.over_ms = now;
                    if self.score > self.best {
                        self.best = self.score;
                    }
                    return Ok(false);
                }
                self.say(if self.ships == 1 {
                    b"LAST SHIP"
                } else {
                    b"SHIP LOST"
                });
            } else {
                self.layer -= 1;
                let mut bytes = [0u8; 20];
                let mut text = crate::text::Text::new(&mut bytes);
                text.append(b"LOST THE ");
                text.append(PARTS[usize::from(self.layer)]);
                self.say(text.bytes());
            }
            return Ok(true);
        }
        if row.gate >> self.lane & 1 != 0 {
            row.taken = 1;
            self.gates = self.gates.wrapping_add(1);
            self.flash = 1.0;
            self.score = self.score.wrapping_add(10 * (u32::from(self.layer) + 1));
            if self.layer < 5 {
                let mut bytes = [0u8; 20];
                let mut text = crate::text::Text::new(&mut bytes);
                text.append(PARTS[usize::from(self.layer)]);
                self.layer += 1;
                text.append(if self.layer == 5 {
                    b" - FULL TRACK"
                } else {
                    b" IN"
                });
                self.say(text.bytes());
                return Ok(true);
            }
            self.score = self.score.wrapping_add(40);
        } else {
            self.score = self.score.wrapping_add(u32::from(self.layer));
        }
        Ok(false)
    }
    pub fn steer(&mut self, select: i32, down: bool, up: bool) -> Result<(), Error> {
        if self.lane >= LANES {
            return Err(Error::State);
        }
        if select != 0 {
            self.lane =
                ((i32::from(self.lane) + if select > 0 { 1 } else { -1 } + i32::from(LANES))
                    % i32::from(LANES)) as u8;
        }
        if down {
            self.lane = (self.lane + LANES - 1) % LANES;
        }
        if up {
            self.lane = (self.lane + 1) % LANES;
        }
        Ok(())
    }
    pub fn masks(&self, lowpass: u32, order: &[u32; 5]) -> Result<[u32; 2], Error> {
        if self.layer > 5 || lowpass >= 32 || order.iter().any(|v| *v >= 32) {
            return Err(Error::Index);
        }
        let (mut effects, mut mute) = (0u32, 0u32);
        if self.on != 0 && self.over == 0 {
            for track in &order[usize::from(self.layer)..] {
                mute |= 1u32 << track;
            }
            if self.beat < self.lp_until {
                effects |= 1u32 << lowpass;
            }
        }
        Ok([effects, mute])
    }
    pub fn leds(
        &self,
        playing: bool,
        frame: u32,
        buttons: [u32; 2],
        glow: &[f32],
        white: &[u8; 8],
        black: &[u8; 11],
    ) -> Result<[u32; 2], Error> {
        if self.lane >= LANES
            || buttons.iter().any(|v| *v >= 32)
            || white.iter().chain(black.iter()).any(|v| *v >= 32)
        {
            return Err(Error::Index);
        }
        let (mut buttons_out, mut keys) = (0u32, 0u32);
        if playing {
            buttons_out |= 1u32 << buttons[0];
        }
        for (i, level) in glow.iter().take(11).enumerate() {
            if *level > 0.35 {
                keys |= 1u32 << black[i];
            }
        }
        if self.over == 0 {
            keys |= 1u32 << white[usize::from(self.lane)];
        }
        for row in &self.row {
            let depth = row.beat - self.beat;
            if row.done == 0 && (0.0..2.0).contains(&depth) && (frame / 6) & 1 != 0 {
                for (lane, key) in white.iter().enumerate() {
                    if row.wall >> lane & 1 != 0 {
                        keys |= 1u32 << key;
                    }
                }
            }
        }
        if self.layer >= 5 {
            buttons_out |= 1u32 << buttons[1];
        }
        Ok([buttons_out, keys])
    }
}
const _: [(); 236] = [(); core::mem::size_of::<State>()];
const _: [(); 16] = [(); core::mem::size_of::<Observation>()];
