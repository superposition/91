// SPDX-License-Identifier: GPL-3.0-only
//! Four 2-bit retained source leases in the existing USB struct padding byte.
//! State is supplied/returned by value; observing 0 is distinct from unknown IO.
pub const ALLOWED: u32 = 1 << 8;
pub const CLEAR: u32 = 0;
pub const BUSY: u32 = 1;
pub const UNKNOWN: u32 = 2;
pub const ARMED: u32 = 3;
pub const UNCERTAIN_ARM: u32 = 4;
pub const STOPPED: u32 = 5;
pub const SEAL: u32 = 6;
pub const INVALIDATE: u32 = 7;
///0 ready,1 uncertain arm,2 accepted/pending,3 sealed until confirmed stop.
/// A confirmed busy after an uncertain arm establishes acceptance; a subsequent
/// confirmed free retires it. Unknown IO and a fresh0 alone cannot retire it.
/// STOPPED is an external positive fact: no future DMA read/write of old roots.
pub fn transition(state: u32, endpoint: u32, event: u32) -> u32 {
    if event > INVALIDATE {
        return state & 255;
    }
    let shift = match endpoint {
        0 => 0,
        1 => 2,
        3 => 4,
        4 => 6,
        255 if event == STOPPED || event == INVALIDATE => 7,
        _ => return state & 255,
    };
    request((state & 255) | (shift << 8) | ((event & 7) << 11))
}
/// Compact immutable decision table, 32 bytes; no expanded runtime cache.
/// The scalar ABI uses one argument/result; target frames need emitted evidence.
#[inline(always)]
pub fn request(value: u32) -> u32 {
    static PLANS: [[u8; 4]; 8] = [
        [4, 1, 4, 3],
        [2, 2, 2, 3],
        [0, 1, 2, 3],
        [2, 2, 2, 3],
        [1, 1, 2, 3],
        [4, 4, 4, 4],
        [3, 3, 3, 3],
        [0, 3, 3, 3],
    ];
    let state = value & 255;
    let shift = (value >> 8) & 7;
    let event = (value >> 11) & 7;
    if shift == 7 {
        if event == STOPPED {
            return ALLOWED;
        }
        if event == INVALIDATE {
            let occupied = (state | (state >> 1)) & 0x55;
            return occupied | (occupied << 1);
        }
    }
    if shift & 1 != 0 || shift > 6 || event > INVALIDATE {
        return state;
    }
    let phase = (state >> shift) & 3;
    let plan = u32::from(PLANS[event as usize][phase as usize]);
    (state & !(3 << shift)) | ((plan & 3) << shift) | ((plan & 4) << 6)
}
