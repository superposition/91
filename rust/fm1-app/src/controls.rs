// SPDX-License-Identifier: GPL-3.0-only
//! Bounded panel-control policy over the existing foreground UI fields.
use crate::ui::State;
use fm1_types::ParamId;

pub fn voice_of(view: i32) -> Option<u8> {
    if (1..=4).contains(&view) {
        Some(0)
    } else if (6..=10).contains(&view) {
        Some(1)
    } else if (11..=13).contains(&view) {
        Some(2)
    } else {
        None
    }
}
pub fn kit_view(view: i32) -> bool {
    matches!(view, 20 | 21 | 22 | 35)
}
pub fn octave_parameter(view: i32) -> ParamId {
    if (1..=4).contains(&view) {
        ParamId::PAD_OCT
    } else if (6..=10).contains(&view) {
        ParamId::BASSOCT
    } else if (11..=13).contains(&view) || view == 15 {
        ParamId::AC_OCT
    } else if (16..=19).contains(&view) {
        ParamId::CM_OCT
    } else {
        ParamId::OCT
    }
}
pub fn group_of(view: i32) -> Option<usize> {
    crate::buttons::GROUPS
        .iter()
        .position(|g| view >= i32::from(g.first) && view < i32::from(g.first) + i32::from(g.count))
}

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
