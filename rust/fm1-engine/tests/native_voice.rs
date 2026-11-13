// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::fm6::{
    program::{self, Profile},
    *,
};
use fm1_formats::voice;

fn packed() -> [u8; 128] {
    let mut v = [0; 155];
    for op in 0..6 {
        v[op * OP + KVS] = (op + 1) as u8;
        v[op * OP + R1] = 90;
        v[op * OP + R1 + 3] = 50;
        v[op * OP + OL] = 70;
    }
    v[LPMD] = 37;
    v[TRNSP] = 24;
    voice::pack(&v)
}

#[test]
fn native_neutral_preserves_every_unpacked_byte_for_both_roles() {
    let p = packed();
    for part in 0..2 {
        let c = Profile {
            part,
            touch: 50,
            release: 65,
            brightness: 50,
            attack: 0,
            voice_flags: 3,
        };
        let mut output = [0; 155];
        program::songbook(&p, &mut output, &c);
        assert_eq!(output, voice::unpack(&p));
    }
}

#[test]
fn native_flags_are_independent_and_touch_uses_signed_truncation() {
    let p = packed();
    let original = voice::unpack(&p);
    for flags in 0..4 {
        for part in 0..2 {
            for (touch, delta) in [
                (0, -7),
                (42, -1),
                (43, 0),
                (49, 0),
                (50, 0),
                (51, 0),
                (57, 0),
                (58, 1),
                (100, 7),
            ] {
                let c = Profile {
                    part,
                    touch,
                    release: 65,
                    brightness: 50,
                    attack: 0,
                    voice_flags: flags,
                };
                let mut output = [0; 155];
                program::songbook(&p, &mut output, &c);
                assert_eq!(output[LPMD], if flags & 2 != 0 { 37 } else { 0 });
                for op in 0..6 {
                    let expected = if part == 1 {
                        original[op * OP + KVS]
                    } else if flags & 1 != 0 {
                        (original[op * OP + KVS] as i32 + delta).clamp(0, 7) as u8
                    } else {
                        (touch * 7 / 100) as u8
                    };
                    assert_eq!(output[op * OP + KVS], expected);
                }
            }
        }
    }
    assert_eq!(core::mem::size_of::<Profile>(), 24);
}
