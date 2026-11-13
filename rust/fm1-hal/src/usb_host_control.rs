// SPDX-License-Identifier: GPL-3.0-only
//! Foreground host control transfers. Hardware observations/cache/DMA/watchdog
//! are supplied by leaf bindings; all payload accesses remain bounded IO.
use crate::usb_endpoint::{ARMED, CLEAR, SEAL, UNCERTAIN_ARM};
pub const FADDR: u32 = 0;
pub const INDEX: u32 = 14;
pub const CSR: u32 = 17;
pub const COUNT: u32 = 22;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Request {
    pub address: u8,
    pub request_type: u8,
    pub request: u8,
    pub maximum: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
    pub reserved: u16,
}
const _: [(); 12] = [(); core::mem::size_of::<Request>()];
pub trait Io {
    fn now(&self) -> u32;
    fn read(&mut self, register: u32) -> Option<u8>;
    fn write(&mut self, register: u32, value: u32) -> bool;
    fn phase(&self) -> u8;
    fn lease(&mut self, event: u32);
    fn error(&mut self, value: u8);
    fn buffer(&self, index: usize) -> u8;
    fn set_buffer(&mut self, index: usize, value: u8);
    fn output(&mut self, index: usize, value: u8);
    fn send(&mut self, count: u32);
    fn sync(&mut self);
    fn feed(&mut self);
}
fn failed(io: &mut impl Io, error: i32, report: bool) -> i32 {
    if report {
        io.error((-error) as u8);
    }
    io.lease(SEAL);
    error
}
fn wait(io: &mut impl Io, set: u8, clear: u8) -> i32 {
    let start = io.now();
    loop {
        let Some(value) = io.read(CSR) else {
            return failed(io, -3, false);
        };
        if value & 0x14 != 0 {
            let _ = io.write(CSR, 0);
            return failed(io, if value & 4 != 0 { -2 } else { -1 }, false);
        }
        if value & set == set && value & clear == 0 {
            io.lease(CLEAR);
            return i32::from(value);
        }
        if io.now().wrapping_sub(start) > 500 {
            return failed(io, -3, false);
        }
        io.feed();
    }
}
fn kick(io: &mut impl Io, value: u32) -> bool {
    io.lease(UNCERTAIN_ARM);
    if !io.write(CSR, value) {
        failed(io, -3, true);
        return false;
    }
    io.lease(ARMED);
    true
}
/// Only requests with no OUT data stage, matching all existing callers. The
/// owner keeps active device/data polling disabled, IRQs enabled and the clock
/// progressing during this synchronous foreground operation with the existing >500ms timeout decision.
/// Stage diagnostics and unknown-completion sealing retain the prior contract.
pub fn transfer(io: &mut impl Io, p: &Request) -> i32 {
    if usize::from(p.length) > crate::usb_host::CONFIG_BYTES
        || (p.request_type & 0x80 != 0 && p.length != 0 && !matches!(p.maximum, 8 | 16 | 32 | 64))
    {
        return failed(io, -3, true);
    }
    if io.phase() & 3 != 0 {
        return failed(io, -3, true);
    }
    if !io.write(FADDR, u32::from(p.address)) || !io.write(INDEX, 0) {
        return failed(io, -3, true);
    }
    io.set_buffer(0, p.request_type);
    io.set_buffer(1, p.request);
    io.set_buffer(2, p.value as u8);
    io.set_buffer(3, (p.value >> 8) as u8);
    io.set_buffer(4, p.index as u8);
    io.set_buffer(5, (p.index >> 8) as u8);
    io.set_buffer(6, p.length as u8);
    io.set_buffer(7, (p.length >> 8) as u8);
    io.lease(UNCERTAIN_ARM);
    io.send(8);
    if !kick(io, 0x0a) {
        return -3;
    }
    let mut result = wait(io, 0, 2);
    if result < 0 {
        io.error((-result) as u8);
        return result;
    }
    let mut got = 0u32;
    if p.length != 0 && p.request_type & 0x80 != 0 {
        while got < u32::from(p.length) {
            if !kick(io, 0x20) {
                return -3;
            }
            result = wait(io, 1, 0);
            if result < 0 {
                io.error((-result) as u8);
                return result;
            }
            let Some(count) = io.read(COUNT) else {
                return failed(io, -3, true);
            };
            let count = u32::from(count).min(64);
            io.sync();
            for index in 0..count.min(u32::from(p.length) - got) {
                let value = io.buffer(index as usize);
                io.output((got + index) as usize, value);
            }
            if !io.write(CSR, 0) {
                return failed(io, -3, true);
            }
            got += count;
            if count < u32::from(p.maximum) {
                break;
            }
        }
        got = got.min(u32::from(p.length));
        if !kick(io, 0x42) {
            return -3;
        }
        result = wait(io, 0, 2);
    } else {
        if !kick(io, 0x60) {
            return -3;
        }
        result = wait(io, 1, 0);
        if result >= 0 && !io.write(CSR, 0) {
            return failed(io, -3, true);
        }
    }
    if result < 0 {
        result
    } else {
        got as i32
    }
}
