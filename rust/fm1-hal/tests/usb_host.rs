// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::usb_host::{packet, parse, receive, Receive};
#[test]
fn first_usable_alternate_keeps_its_identity_and_never_borrows_another_endpoint() {
    let config = [
        9, 2, 0, 0, 2, 7, 0, 0, 0, 9, 4, 6, 3, 1, 1, 3, 0, 0, 7, 5, 0x82, 3, 5, 0, 4, 9, 4, 8, 0,
        1, 1, 3, 0, 0, 7, 5, 3, 2, 64, 0, 0,
    ];
    let selected = parse(&config);
    assert_eq!((selected.cfg, selected.iface, selected.alt), (7, 6, 3));
    assert_eq!((selected.in_ep, selected.in_mps), (2, 5));
    assert_eq!((selected.out_ep, selected.out_mps), (0, 0));
    assert_eq!((selected.reserved0, selected.reserved1), (0, 0));
    for end in 0..25 {
        assert_eq!(parse(&config[..end]).found, 0);
    }
    let mut short = config;
    short[18] = 1;
    assert_eq!(parse(&short).found, 0);
}
#[test]
fn legacy_packet_filter_keeps_two_byte_and_realtime_tail_quirks() {
    assert_eq!(packet(0xff7f_cdac), 0xff7f_cd0c);
    assert_eq!(packet(0xffff_f8ff), 0xffff_f80f);
    assert_eq!(packet(0xff7f_90a9), 0);
    assert_eq!(packet(0x647f_90a9), 0x647f_9009);
    assert_eq!(packet(0x0080_c00c), 0);
    assert_eq!(packet(0x0000_f00f), 0);
}
struct Ring {
    input: [u8; 64],
    words: [u32; 64],
    read: u32,
    write: u32,
    published: usize,
    steps: Vec<(u32, u32)>,
}
impl Receive for Ring {
    fn write(&self) -> u32 {
        self.write
    }
    fn read(&self) -> u32 {
        self.read
    }
    fn byte(&self, index: usize) -> u8 {
        self.input[index]
    }
    fn put(&mut self, index: usize, value: u32) {
        self.words[index] = value;
    }
    fn publish(&mut self) {
        self.steps
            .push((self.write, self.words[(self.write % 64) as usize]));
        self.published += 1;
    }
    fn set_write(&mut self, value: u32) {
        self.write = value;
    }
}
#[test]
fn conservative_admission_and_wrap_publish_each_slot_before_its_index() {
    let mut ring = Ring {
        input: [0; 64],
        words: [0; 64],
        read: 0,
        write: 49,
        published: 0,
        steps: Vec::new(),
    };
    ring.input[..4].copy_from_slice(&0x7f40_91a9u32.to_le_bytes());
    assert_eq!(receive(&mut ring, 4), None);
    assert_eq!(ring.write, 49);
    assert_eq!(ring.published, 0);
    ring.write = u32::MAX - 1;
    ring.read = ring.write.wrapping_sub(48);
    ring.input[4..8].copy_from_slice(&0x0000_faffu32.to_le_bytes());
    ring.input[8..12].copy_from_slice(&0x0080_9009u32.to_le_bytes());
    ring.input[12..16].copy_from_slice(&0x0042_c01cu32.to_le_bytes());
    assert_eq!(receive(&mut ring, 19), Some(3));
    assert_eq!(ring.write, 1);
    assert_eq!(
        ring.steps,
        vec![
            (u32::MAX - 1, 0x7f40_9109),
            (u32::MAX, 0x0000_fa0f),
            (0, 0x0042_c00c)
        ]
    );
    assert_eq!(receive(&mut ring, 65), None);
}
