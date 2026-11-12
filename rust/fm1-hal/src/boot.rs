// SPDX-License-Identifier: GPL-3.0-only
//! Stateless early boot policy: no initialized RAM, counters or callbacks.
pub const MAGIC: u32 = 0x42475244;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Guard {
    pub magic: u32,
    pub failed: u32,
    pub pending: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    SafeMode,
    UpdateLoader,
}
pub fn enter(guard: &mut Guard) -> Action {
    if guard.magic != MAGIC {
        *guard = Guard {
            magic: MAGIC,
            failed: 0,
            pending: 0,
        };
    }
    if guard.pending != 0 {
        guard.failed = guard.failed.wrapping_add(1);
    }
    guard.pending = 1;
    if guard.failed >= 4 {
        guard.failed = 0;
        guard.pending = 0;
        Action::UpdateLoader
    } else if guard.failed >= 2 {
        Action::SafeMode
    } else {
        Action::Continue
    }
}
pub fn healthy(guard: &mut Guard, uptime: u32, safe: bool) {
    if safe {
        if guard.pending != 0 && uptime > 10000 {
            guard.pending = 0;
        }
    } else if uptime > 30000 && guard.pending != 0 {
        guard.pending = 0;
        guard.failed = 0;
    }
}
pub fn intentional_reset(guard: &mut Guard, new_install: bool) {
    guard.pending = 0;
    if new_install {
        guard.failed = 0;
    }
}
/// C debug access admits only RAM/XIP, rejects wrap and permits empty end spans.
pub fn readable(address: u32, length: u32) -> bool {
    match address.checked_add(length) {
        Some(end) => {
            (address >= 0x01c00000 && end <= 0x01c80000)
                || (address >= 0x02000000 && end <= 0x02100000)
        }
        None => false,
    }
}
