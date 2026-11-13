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

struct Tx {
    queue: [u32; 64],
    output: [u8; 64],
    write: u32,
    read: u32,
    clock: core::cell::Cell<u32>,
    clock_reads: core::cell::Cell<u32>,
    denied: bool,
    published: Vec<u32>,
}
impl fm1_hal::usb_host::Transmit for Tx {
    fn write(&self) -> u32 {
        self.write
    }
    fn read(&self) -> u32 {
        self.read
    }
    fn set_read(&mut self, v: u32) {
        self.read = v;
    }
    fn word(&self, i: usize) -> u32 {
        self.queue[i]
    }
    fn output(&mut self, i: usize, v: u8) {
        self.output[i] = v;
    }
    fn publish(&mut self) {
        self.published.push(self.read);
    }
    fn now(&self) -> u32 {
        let n = self.clock.get();
        self.clock.set(n.wrapping_add(1));
        self.clock_reads.set(self.clock_reads.get() + 1);
        n
    }
    fn grant(&mut self, csr: u32) -> bool {
        !self.denied && csr & 3 == 0
    }
}
fn tx() -> Tx {
    let mut q = [0; 64];
    for (i, v) in q.iter_mut().enumerate() {
        *v = 0x7f40_9009 + ((i as u32) << 16);
    }
    Tx {
        queue: q,
        output: [0xa5; 64],
        write: 4,
        read: 0,
        clock: core::cell::Cell::new(499),
        clock_reads: core::cell::Cell::new(0),
        denied: false,
        published: Vec::new(),
    }
}
#[test]
fn host_ack_uses_the_two_original_clock_read_points_and_keeps_slots_until_ack() {
    use fm1_hal::usb_host::{transmit, TxAction, TxState};
    let mut io = tx();
    let mut state = TxState {
        timestamp: 0,
        count: 1,
        errors: 15,
        reserved: [0; 2],
    };
    assert_eq!(transmit(&mut io, &mut state, 0, 4), TxAction::Send(4));
    assert_eq!(io.read, 1);
    assert_eq!(io.published, vec![0]);
    assert_eq!(io.clock_reads.get(), 2);
    assert_eq!(state.timestamp, 500);
    assert_eq!(&io.output[..4], &io.queue[1].to_le_bytes());
    assert_eq!((state.count, state.errors), (1, 0));
    let before = io.output;
    assert_eq!(transmit(&mut io, &mut state, 1, 64), TxAction::Wait);
    assert_eq!(io.read, 1);
    assert_eq!(io.output, before);
}
#[test]
fn host_stall_retry_expiry_and_unknown_retirement_never_consume_or_rewrite() {
    use fm1_hal::usb_host::{transmit, TxAction, TxState};
    let mut io = tx();
    let mut state = TxState {
        timestamp: 0,
        count: 1,
        errors: 15,
        reserved: [0; 2],
    };
    assert_eq!(transmit(&mut io, &mut state, 0x24, 64), TxAction::Fail);
    assert_eq!(io.clock_reads.get(), 0);
    assert_eq!(state.errors, 15);
    assert_eq!(transmit(&mut io, &mut state, 4, 64), TxAction::Fail);
    assert_eq!(state.errors, 16);
    io.clock.set(2);
    state.errors = 255;
    assert_eq!(transmit(&mut io, &mut state, 4, 64), TxAction::Retry);
    assert_eq!(state.errors, 0); // existing u8 pre-increment compatibility
    io.denied = true;
    assert_eq!(transmit(&mut io, &mut state, 0, 64), TxAction::Fail);
    assert_eq!(io.read, 0);
    assert_eq!(io.output, [0xa5; 64]);
    io.denied = false;
    io.clock.set(500);
    assert_eq!(transmit(&mut io, &mut state, 0, 64), TxAction::Fail);
    assert_eq!(io.read, 0);
    assert_eq!(state.count, 1);
}
