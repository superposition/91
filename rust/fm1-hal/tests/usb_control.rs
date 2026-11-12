// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::{usb_control, usb_descriptors::Descriptors};
#[test]
fn descriptors_have_bounded_exact_lengths_and_invalid_copies_do_not_write() {
    for flags in 0..8 {
        let set = Descriptors::new(flags, 0xbeef);
        for request in [0x0100, 0x01ff, 0x0200, 0x02ff, 0x0300, 0x0301, 0x0302] {
            let length = set.length(request).unwrap();
            for index in 0..length {
                assert!(set.byte(request, index).is_some());
            }
            assert_eq!(set.byte(request, length), None);
            let mut bytes = [0xa5; 66];
            assert_eq!(
                set.copy(request, length + 1, 64, |i, v| bytes[i + 1] = v),
                None
            );
            assert_eq!(bytes, [0xa5; 66]);
            let n = set.copy(request, 0, 64, |i, v| bytes[i + 1] = v).unwrap();
            assert_eq!(n, length.min(64));
            assert_eq!(bytes[0], 0xa5);
            assert_eq!(bytes[n + 1], 0xa5);
            if request >> 8 == 2 {
                assert_eq!(u16::from_le_bytes([bytes[3], bytes[4]]) as usize, length);
                assert_eq!(
                    bytes[5],
                    2 + u8::from(flags & 2 != 0) + 2 * u8::from(flags & 1 != 0)
                );
            } else if request >> 8 == 3 {
                assert_eq!(usize::from(bytes[1]), length);
            }
        }
        let mut written = false;
        assert_eq!(set.copy(0x0303, 0, 64, |_, _| written = true), None);
        assert!(!written);
    }
}
#[test]
fn ep0_clips_payload_and_finishes_once_with_bounded_packets() {
    for length in 0..=1024u16 {
        for wanted in (0..=1088u16).step_by(63) {
            let mut state = usb_control::start(length, wanted);
            let mut total = 0;
            let mut packets = 0;
            loop {
                let packet = usb_control::packet((state >> 16) as u16, state & 256 != 0);
                let bytes = packet & 127;
                assert!(bytes <= 64);
                total += bytes;
                packets += 1;
                assert!(packets <= 18);
                if packet & 128 != 0 {
                    assert_eq!(packet >> 16, 0);
                    break;
                }
                state = packet;
            }
            assert_eq!(total, u32::from(length.min(wanted)));
        }
    }
    let first = usb_control::packet((usb_control::start(64, 65) >> 16) as u16, true);
    assert_eq!(first & 255, 64);
    assert_eq!(usb_control::packet(0, false) & 255, 128);
}

#[test]
fn request_actions_keep_low_byte_and_class_compatibility_rules() {
    use fm1_hal::usb_control::{request, Action::*};
    assert_eq!(request(3, 1, 9, 0x0101, 0, 0, false).action, Configuration);
    assert_eq!(request(3, 1, 9, 2, 0, 0, true).action, Stall);
    assert_eq!(request(3, 1, 0x010b, 1, 2, 0, false).action, Stall);
    assert_eq!(request(3, 1, 0x010b, 1, 2, 0, true).action, Stream);
    assert_eq!(request(3, 1, 0x2120, 0, 0xff, 7, false).action, SetLine);
    assert_eq!(
        request(3, 1, 0xa281, 0x01ff, 0x84, 3, false).action,
        GetRate
    );
    assert_eq!(request(3, 1, 0x0201, 0, 0x84, 0, false).action, Ack);
    assert_eq!(request(1, 1, 0x0201, 0, 0x84, 0, false).action, Stall);
    assert_eq!(request(3, 1, 0x8006, 0x0303, 0, 0, false).action, Stall);
    assert_eq!(request(3, 1, 0x8006, 0x0100, 0, 0, false).action, Ack);
}
