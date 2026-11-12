// SPDX-License-Identifier: GPL-3.0-only
//! USB device TX packet traversal and SysEx encoding/wait policy. Retained
//! endpoint roots and interrupt/clock bindings stay outside this safe core.
#[derive(Clone, Copy)]
pub enum Queue {
    Midi,
    Sysex,
}
pub trait Io {
    fn write_index(&self, queue: Queue) -> u32;
    fn read_index(&self, queue: Queue) -> u32;
    fn set_write(&mut self, queue: Queue, value: u32);
    fn set_read(&mut self, queue: Queue, value: u32);
    fn word(&self, queue: Queue, index: usize) -> u32;
    fn put(&mut self, queue: Queue, index: usize, value: u32);
    fn output(&mut self, index: usize, value: u8);
    fn input(&self, index: usize) -> u8;
    fn busy(&self) -> bool;
    fn set_busy(&mut self, value: bool);
    fn configured(&self) -> bool;
    fn publish(&mut self);
    fn now(&mut self) -> u32;
    fn idle(&mut self);
}
fn output(io: &mut impl Io, offset: usize, word: u32) {
    for (index, byte) in word.to_le_bytes().into_iter().enumerate() {
        io.output(offset + index, byte);
    }
}
/// The controller has released the existing64-byte endpoint buffer. SysEx
/// precedes MIDI and an active SysEx producer suppresses MIDI interleaving.
pub fn fill(io: &mut impl Io, sysex: bool) -> usize {
    let mut n = 0;
    if sysex {
        while io.read_index(Queue::Sysex) != io.write_index(Queue::Sysex) && n < 64 {
            let r = io.read_index(Queue::Sysex);
            let word = io.word(Queue::Sysex, (r % 64) as usize);
            output(io, n, word);
            n += 4;
            io.set_read(Queue::Sysex, r.wrapping_add(1));
        }
    }
    while (!sysex || !io.busy() && io.read_index(Queue::Sysex) == io.write_index(Queue::Sysex))
        && io.read_index(Queue::Midi) != io.write_index(Queue::Midi)
        && n < 64
    {
        io.publish();
        let r = io.read_index(Queue::Midi);
        let word = io.word(Queue::Midi, (r % 64) as usize);
        output(io, n, word);
        n += 4;
        io.publish();
        io.set_read(Queue::Midi, r.wrapping_add(1));
    }
    n
}
pub fn packet(
    total: usize,
    index: usize,
    mut byte: impl FnMut(usize) -> u8,
) -> Option<(u32, usize)> {
    let left = total.checked_sub(index)?;
    if left == 0 {
        return None;
    }
    let n = left.min(3);
    let cin = if n == 3 && left > 3 { 4 } else { 4 + n };
    let mut word = cin as u32;
    for offset in 0..n {
        word |= u32::from(byte(index + offset)) << (8 + 8 * offset);
    }
    Some((word, n))
}
/// Compatibility checkpoint: the existing ready/config checks are retained.
/// Reset-epoch admission is a separate explicit correction, not hidden here.
// Preserve arithmetic supported by the pinned mrustc core subset.
#[allow(clippy::manual_div_ceil)]
pub fn send(io: &mut impl Io, total: usize, blocking: bool) -> bool {
    if total > if blocking { 640 } else { 192 } {
        return false;
    }
    let start = if blocking { io.now() } else { 0 };
    if !blocking {
        if !io.configured() {
            return false;
        }
        let used = io
            .write_index(Queue::Sysex)
            .wrapping_sub(io.read_index(Queue::Sysex));
        if used > 64 || (total + 2) / 3 > 64 - used as usize {
            return false;
        }
    }
    io.set_busy(true);
    let mut index = 0;
    while index < total {
        let (word, n) = packet(total, index, |i| io.input(i)).unwrap();
        if blocking {
            while io
                .write_index(Queue::Sysex)
                .wrapping_sub(io.read_index(Queue::Sysex))
                >= 64
            {
                if !io.configured() || io.now().wrapping_sub(start) > 200 {
                    io.set_busy(false);
                    return false;
                }
                io.idle();
            }
        }
        let w = io.write_index(Queue::Sysex);
        io.put(Queue::Sysex, (w % 64) as usize, word);
        io.publish();
        io.set_write(Queue::Sysex, w.wrapping_add(1));
        index += n;
    }
    io.set_busy(false);
    true
}
