// SPDX-License-Identifier: GPL-3.0-only
//! Fixed64-event CPU0 USB rings. The provider serializes producer transactions;
//! raw interrupt bindings and scalar publication are outside this safe core.
pub const EVENTS: u32 = 64;
pub trait Io {
    fn write_index(&self) -> u32;
    fn read_index(&self) -> u32;
    fn set_write(&mut self, index: u32);
    fn set_read(&mut self, index: u32);
    fn word(&self, index: usize) -> u32;
    fn put(&mut self, index: usize, value: u32);
    fn publish(&mut self);
}
pub fn push(io: &mut impl Io, enabled: bool, value: u32) -> bool {
    if !enabled {
        return false;
    }
    let w = io.write_index();
    let r = io.read_index();
    if w.wrapping_sub(r) >= EVENTS {
        return false;
    }
    io.put((w % EVENTS) as usize, value);
    io.publish();
    io.set_write(w.wrapping_add(1));
    true
}
pub fn pop(io: &mut impl Io, mut deliver: impl FnMut(u32)) -> bool {
    let r = io.read_index();
    let w = io.write_index();
    if r == w {
        return false;
    }
    io.publish();
    let value = io.word((r % EVENTS) as usize);
    deliver(value);
    io.publish();
    io.set_read(r.wrapping_add(1));
    true
}
