// SPDX-License-Identifier: GPL-3.0-only
//! Bounded panel-control policy over the existing foreground UI fields.
use crate::ui::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Encoder,
    Overflow,
}

pub fn accelerate(
    ui: &mut State,
    role: usize,
    step: i32,
    range: i32,
    now: u32,
    disabled: bool,
) -> Result<i32, Error> {
    let Some(previous) = ui.enc_t.get(role) else {
        return Err(Error::Encoder);
    };
    let mut elapsed = now.wrapping_sub(*previous);
    let absolute = if step < 0 {
        step.wrapping_neg() as u32
    } else {
        step as u32
    };
    if absolute == 0 {
        ui.enc_t[role] = now;
        return Ok(step);
    }
    if absolute > 1 {
        elapsed /= absolute;
    }
    let run = if elapsed < 60 {
        (u32::from(ui.enc_run[role]) + absolute).min(255) as u8
    } else {
        absolute as u8
    };
    let mut factor = 1i32;
    if range > 24 && !disabled && run >= 4 {
        factor = if elapsed < 12 {
            8
        } else if elapsed < 25 {
            5
        } else if elapsed < 45 {
            3
        } else {
            1
        };
        if range < 100 && factor > 3 {
            factor = 3;
        }
    }
    // Malformed detents that previously caused signed C overflow are rejected
    // without touching state. The physical and emulator defined domain is exact.
    let result = step.checked_mul(factor).ok_or(Error::Overflow)?;
    ui.enc_t[role] = now;
    ui.enc_run[role] = run;
    Ok(result)
}
