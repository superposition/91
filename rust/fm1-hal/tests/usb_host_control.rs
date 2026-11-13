// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::{
    usb_endpoint,
    usb_host_control::{self as control, Io, Request},
};
struct Model {
    buffer: [u8; 68],
    output: [u8; 16],
    clock: u32,
    phase: u8,
    error: u8,
    csr: u8,
    stage: u8,
    fault_stage: u8,
    fault: u8,
    unknown: bool,
    feeds: u32,
    sent: Vec<[u8; 8]>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            buffer: [0xa5; 68],
            output: [0xcc; 16],
            clock: 0xfffffff0,
            phase: 0,
            error: 0x55,
            csr: 0,
            stage: 0,
            fault_stage: 9,
            fault: 0,
            unknown: false,
            feeds: 0,
            sent: Vec::new(),
        }
    }
}
impl Io for Model {
    fn now(&self) -> u32 {
        self.clock
    }
    fn read(&mut self, register: u32) -> Option<u8> {
        if self.unknown {
            return None;
        }
        Some(if register == control::COUNT {
            3
        } else if self.stage == self.fault_stage && self.fault != 0 {
            self.fault
        } else {
            self.csr
        })
    }
    fn write(&mut self, register: u32, value: u32) -> bool {
        if register == control::CSR {
            match value {
                0x0a => {
                    self.stage = 0;
                    self.csr = 0;
                }
                0x20 => {
                    self.stage = 1;
                    self.csr = 1;
                    self.buffer[..3].copy_from_slice(&[17, 34, 51]);
                }
                0x42 => {
                    self.stage = 2;
                    self.csr = 0;
                }
                0x60 => {
                    self.stage = 3;
                    self.csr = 1;
                }
                _ => self.csr = 0,
            }
        }
        true
    }
    fn phase(&self) -> u8 {
        self.phase
    }
    fn lease(&mut self, event: u32) {
        self.phase = usb_endpoint::transition(u32::from(self.phase), 0, event) as u8;
    }
    fn error(&mut self, value: u8) {
        self.error = value;
    }
    fn buffer(&self, index: usize) -> u8 {
        self.buffer[index]
    }
    fn set_buffer(&mut self, index: usize, value: u8) {
        self.buffer[index] = value;
    }
    fn output(&mut self, index: usize, value: u8) {
        self.output[index] = value;
    }
    fn send(&mut self, count: u32) {
        assert_eq!(count, 8);
        self.sent.push(self.buffer[..8].try_into().unwrap());
    }
    fn sync(&mut self) {}
    fn feed(&mut self) {
        self.clock = self.clock.wrapping_add(1);
        self.feeds += 1;
    }
}
fn request() -> Request {
    Request {
        address: 3,
        request_type: 0x80,
        request: 6,
        maximum: 8,
        value: 0x1234,
        index: 0xabcd,
        length: 8,
        reserved: 0,
    }
}
#[test]
fn short_packet_initializes_only_its_prefix_and_preserves_wire_and_status_diagnostics() {
    let mut io = Model::default();
    assert_eq!(control::transfer(&mut io, &request()), 3);
    assert_eq!(io.sent, vec![[0x80, 6, 0x34, 0x12, 0xcd, 0xab, 8, 0]]);
    assert_eq!(&io.output[..8], &[17, 34, 51, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc]);
    assert_eq!((io.phase, io.error, io.feeds), (0, 0x55, 0));
    let mut io = Model {
        fault_stage: 2,
        fault: 4,
        ..Model::default()
    };
    assert_eq!(control::transfer(&mut io, &request()), -2);
    assert_eq!(io.error, 0x55); // existing status-stage diagnostic quirk
    assert_eq!(&io.output[..3], &[17, 34, 51]);
    assert_eq!(io.phase & 3, 3);
}
#[test]
fn timeout_uses_wrapping_clock_and_seals_without_replaying_the_retained_source() {
    let mut io = Model {
        fault_stage: 0,
        fault: 2,
        ..Model::default()
    };
    assert_eq!(control::transfer(&mut io, &request()), -3);
    assert_eq!(io.feeds, 501);
    assert_eq!(io.clock, 0xfffffff0u32.wrapping_add(501));
    assert_eq!((io.phase & 3, io.error), (3, 3));
    let source = io.buffer;
    assert_eq!(control::transfer(&mut io, &request()), -3);
    assert_eq!(io.sent.len(), 1);
    assert_eq!(io.buffer, source);
    assert_eq!(io.output, [0xcc; 16]);
}
#[test]
fn unknown_zero_is_not_a_completed_zero_observation() {
    let mut io = Model {
        unknown: true,
        ..Model::default()
    };
    assert_eq!(control::transfer(&mut io, &request()), -3);
    assert_eq!((io.phase & 3, io.error), (3, 3));
    assert_eq!(io.output, [0xcc; 16]);
    let mut ready = Model::default();
    assert_eq!(control::transfer(&mut ready, &request()), 3);
    assert_eq!(ready.phase & 3, 0);
}
