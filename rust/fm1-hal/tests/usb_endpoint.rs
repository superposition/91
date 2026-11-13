// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::usb_endpoint::*;
#[test]
fn every_lease_byte_keeps_other_endpoints_and_unknown_never_grants_reuse() {
    for state in 0..256 {
        for (ep, shift) in [(0, 0), (1, 2), (3, 4), (4, 6)] {
            let mask = 3 << shift;
            let phase = (state >> shift) & 3;
            let unknown = transition(state, ep, UNKNOWN);
            assert_eq!(unknown, state);
            let clear = transition(state, ep, CLEAR);
            assert_eq!(clear & !mask & 255, state & !mask);
            assert_eq!(clear & ALLOWED != 0, phase == 0 || phase == 2);
            let uncertain = transition(state & !mask, ep, UNCERTAIN_ARM);
            assert_eq!(transition(uncertain, ep, CLEAR) & ALLOWED, 0);
            assert_eq!(transition(uncertain, ep, UNKNOWN), uncertain);
            let accepted = transition(uncertain, ep, BUSY);
            assert_eq!(accepted & ALLOWED, 0);
            let complete = transition(accepted, ep, CLEAR);
            assert_ne!(complete & ALLOWED, 0);
            assert_eq!(complete & mask, 0);
            let sealed = transition(state, ep, SEAL);
            for event in [UNKNOWN, CLEAR, BUSY, ARMED, UNCERTAIN_ARM, SEAL] {
                assert_eq!(transition(sealed, ep, event), sealed);
            }
            let invalidated = transition(state, ep, INVALIDATE);
            assert_eq!((invalidated >> shift) & 3, if phase == 0 { 0 } else { 3 });
            assert_eq!(invalidated & !mask & 255, state & !mask);
            let stopped = transition(uncertain, ep, STOPPED);
            assert_eq!(stopped & 255, state & !mask);
            assert_ne!(stopped & ALLOWED, 0);
        }
        assert_eq!(transition(state, 255, STOPPED), ALLOWED);
        let invalidated = transition(state, 255, INVALIDATE);
        for shift in [0, 2, 4, 6] {
            assert_eq!(
                (invalidated >> shift) & 3,
                if (state >> shift) & 3 == 0 { 0 } else { 3 }
            );
        }
        for invalid in [2, 5, 127, 255] {
            assert_eq!(transition(state, invalid, CLEAR), state);
        }
    }
}
