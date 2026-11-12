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
    /// Reserved MAX means no active bus/configuration, or exhausted epoch.
    fn epoch(&self) -> u32;
    fn enter(&mut self) -> u32;
    fn exit(&mut self, flags: u32);
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
pub fn legacy_send(io: &mut impl Io, total: usize, blocking: bool) -> bool {
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

/// Reset generations never repeat. Exhaustion must latch the device inactive;
/// the driver keeps its original buffers alive and admits no new endpoint work.
pub fn next_epoch(previous: u32) -> u32 {
    previous.saturating_add(1)
}
/// App producer transaction. Its only masked operations are scalar capture,
/// a bounded three-byte event publication and flag/index updates. Every clock
/// read/wait/idle executes after restoring the caller's exact incoming IRQ state.
#[allow(clippy::manual_div_ceil)]
pub fn send(io: &mut impl Io, total: usize, blocking: bool) -> bool {
    if total > if blocking { 640 } else { 192 } {
        return false;
    }
    let start = if blocking { io.now() } else { 0 };
    let irq = io.enter();
    let epoch = io.epoch();
    let used = io
        .write_index(Queue::Sysex)
        .wrapping_sub(io.read_index(Queue::Sysex));
    // The app's timer consumer cannot make progress with incoming IRQs off.
    // A bounded message may still complete entirely from current free slots.
    if epoch == u32::MAX
        || used > 64
        || (!blocking || irq & 256 == 0) && (total + 2) / 3 > 64 - used as usize
    {
        io.exit(irq);
        return false;
    }
    io.set_busy(true);
    io.exit(irq);
    let mut index = 0;
    while index < total {
        let (word, n) = packet(total, index, |i| io.input(i)).unwrap();
        loop {
            let irq = io.enter();
            if io.epoch() != epoch {
                io.set_busy(false);
                io.exit(irq);
                return false;
            }
            let w = io.write_index(Queue::Sysex);
            let r = io.read_index(Queue::Sysex);
            if w.wrapping_sub(r) < 64 {
                io.put(Queue::Sysex, (w % 64) as usize, word);
                io.publish();
                io.set_write(Queue::Sysex, w.wrapping_add(1));
                io.exit(irq);
                index += n;
                break;
            }
            io.exit(irq);
            if !blocking || io.now().wrapping_sub(start) > 200 {
                let irq = io.enter();
                io.set_busy(false);
                io.exit(irq);
                return false;
            }
            io.idle();
        }
    }
    let irq = io.enter();
    let complete = io.epoch() == epoch;
    io.set_busy(false);
    io.exit(irq);
    complete
}
