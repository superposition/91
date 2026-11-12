// SPDX-License-Identifier: GPL-3.0-only
//! CPU0 transaction admission. The platform saves IRQ state around each update;
//! CPU1 never reads or writes this word. This is not an atomic/worker-stop API.
pub const BUSY: i32 = -10;
const RENDER: u32 = 1 << 31;
const DEPTH: u32 = 0xffff;

/// Zero means admission denied; a successful admission always has nonzero depth.
pub const fn enter(state: u32, worker_unknown: bool) -> u32 {
    if state & !DEPTH != 0 || worker_unknown || state == DEPTH {
        0
    } else {
        state + 1
    }
}
pub const fn leave(state: u32) -> u32 {
    if state & DEPTH != 0 {
        state - 1
    } else {
        state
    }
}
pub const fn render(state: u32, active: bool) -> u32 {
    if active {
        state | RENDER
    } else {
        state & !RENDER
    }
}
pub const fn serial(state: u32) -> bool {
    state & DEPTH != 0
}
/// Only a complete admitted foreground transaction can enter an SFC-off window.
pub const fn window(state: u32, worker_unknown: bool) -> bool {
    state & !DEPTH == 0 && state & DEPTH != 0 && !worker_unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_is_nonblocking_nested_and_never_grants_a_live_worker() {
        for depth in 0..=DEPTH {
            assert_eq!(enter(depth, true), 0);
            assert_eq!(enter(render(depth, true), false), 0);
            assert!(!window(render(depth, true), false));
            assert!(!window(depth, true));
            assert_eq!(serial(render(depth, true)), depth != 0);
            assert_eq!(render(render(depth, true), false), depth);
            if depth < DEPTH {
                let next = enter(depth, false);
                assert_eq!(next, depth + 1);
                assert!(window(next, false));
                assert_eq!(leave(next), depth);
            } else {
                assert_eq!(enter(depth, false), 0);
            }
        }
        assert!(!window(0, false));
        assert_eq!(leave(0), 0);
    }
}
