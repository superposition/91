// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::midi::{self, Trs, TrsIo, TRS_EMPTY};
struct Io {
    ring: [u8; 128],
    packets: Vec<u32>,
    capacity: usize,
    reads: usize,
}
impl TrsIo for Io {
    fn read(&mut self, index: u32) -> u8 {
        self.reads += 1;
        self.ring[index as usize]
    }
    fn write(&mut self, index: u32, value: u8) {
        self.ring[index as usize] = value;
    }
    fn send(&mut self, packet: u32) -> bool {
        if self.packets.len() == self.capacity {
            return false;
        }
        self.packets.push(packet);
        true
    }
}
#[test]
fn content_not_count_preserves_notes_realtime_and_running_status() {
    let mut io = Io {
        ring: [0; 128],
        packets: vec![],
        capacity: 64,
        reads: 0,
    };
    midi::clear(&mut io);
    let bytes = [
        0x91, 60, 100, 62, 0xf8, 90, 0xfd, 64, 0, 0xf0, 4, 5, 0xfc, 0xf7, 7, 9,
    ];
    io.ring[..bytes.len()].copy_from_slice(&bytes);
    let mut state = Trs::default();
    assert!(state.take(1, &mut io));
    assert_eq!(
        io.packets,
        [0x643c9109, 0x0000f80f, 0x5a3e9109, 0x00409109, 0x0000fc0f]
    );
    assert_eq!(state.rd, bytes.len() as u32);
    assert_eq!(state.bytes, bytes.len() as u32);
    assert_eq!(state.messages, 5);
}
#[test]
fn laps_drops_wrap_and_partial_reset_keep_the_original_limits() {
    let mut io = Io {
        ring: [TRS_EMPTY; 128],
        packets: vec![],
        capacity: 0,
        reads: 0,
    };
    let mut state = Trs {
        rd: 1,
        bytes: u32::MAX,
        drops: u32::MAX,
        messages: u32::MAX,
        status: 0x91,
        need: 2,
        got: 1,
        first: 37,
        sysex: 1,
    };
    io.ring[0] = 0x90;
    io.ring[1] = 60;
    io.ring[2] = 100;
    assert!(state.take(u32::MAX, &mut io));
    assert_eq!(state.rd, 3);
    assert_eq!(state.bytes, 2);
    assert_eq!(state.drops, u32::MAX - 128);
    assert_eq!(state.messages, u32::MAX);
    assert_eq!(state.first, 60);
    assert_eq!(state.need, 2);
    state.rd = 128;
    let old = state;
    let reads = io.reads;
    assert!(!state.take(1, &mut io));
    assert_eq!(state, old);
    assert_eq!(io.reads, reads);
}
