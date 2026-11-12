// SPDX-License-Identifier: GPL-3.0-only
//! Raw retained TX roots; only the caller-stack descriptor is referenced.
use fm1_hal::usb_tx::{self, Io, Queue};
#[repr(C)]
pub struct Context {
    flags: u32,
    slots: [*mut u8; 9],
    publish: Option<unsafe extern "C" fn()>,
    now: Option<unsafe extern "C" fn() -> u32>,
    idle: Option<unsafe extern "C" fn()>,
    epoch: Option<unsafe extern "C" fn() -> u32>,
    enter: Option<unsafe extern "C" fn() -> u32>,
    exit: Option<unsafe extern "C" fn(u32)>,
}
const _: [(); 16 * core::mem::size_of::<*mut u8>()] = [(); core::mem::size_of::<Context>()];
struct Raw<'a> {
    context: &'a Context,
    input: *const u8,
    count: usize,
}
impl Raw<'_> {
    fn slot(queue: Queue) -> usize {
        match queue {
            Queue::Midi => 0,
            Queue::Sysex => 3,
        }
    }
}
// SAFETY: the unsafe ABI caller supplies live disjoint fixed roots and queue
// roles; construction validates pointer/alignment/capacity and callback bounds.
// No reference into endpoint, source message, slot, scalar or index is made.
impl Io for Raw<'_> {
    fn write_index(&self, q: Queue) -> u32 {
        unsafe {
            self.context.slots[Self::slot(q) + 1]
                .cast::<u32>()
                .read_volatile()
        }
    }
    fn read_index(&self, q: Queue) -> u32 {
        unsafe {
            self.context.slots[Self::slot(q) + 2]
                .cast::<u32>()
                .read_volatile()
        }
    }
    fn set_write(&mut self, q: Queue, v: u32) {
        unsafe {
            self.context.slots[Self::slot(q) + 1]
                .cast::<u32>()
                .write_volatile(v)
        }
    }
    fn set_read(&mut self, q: Queue, v: u32) {
        unsafe {
            self.context.slots[Self::slot(q) + 2]
                .cast::<u32>()
                .write_volatile(v)
        }
    }
    fn word(&self, q: Queue, i: usize) -> u32 {
        if i >= 64 {
            return 0;
        }
        unsafe {
            self.context.slots[Self::slot(q)]
                .cast::<u32>()
                .add(i)
                .read_volatile()
        }
    }
    fn put(&mut self, q: Queue, i: usize, v: u32) {
        if i < 64 {
            unsafe {
                self.context.slots[Self::slot(q)]
                    .cast::<u32>()
                    .add(i)
                    .write_volatile(v)
            }
        }
    }
    fn output(&mut self, i: usize, v: u8) {
        if i < 64 {
            unsafe { self.context.slots[7].add(i).write_volatile(v) }
        }
    }
    fn input(&self, i: usize) -> u8 {
        if i >= self.count {
            return 0;
        }
        unsafe { self.input.add(i).read_volatile() }
    }
    fn busy(&self) -> bool {
        unsafe { self.context.slots[6].read_volatile() != 0 }
    }
    fn set_busy(&mut self, v: bool) {
        unsafe { self.context.slots[6].write_volatile(u8::from(v)) }
    }
    fn configured(&self) -> bool {
        unsafe { self.context.slots[8].read_volatile() != 0 }
    }
    fn publish(&mut self) {
        if let Some(f) = self.context.publish {
            unsafe { f() }
        }
    }
    fn now(&mut self) -> u32 {
        self.context.now.map_or(0, |f| unsafe { f() })
    }
    fn idle(&mut self) {
        if let Some(f) = self.context.idle {
            unsafe { f() }
        }
    }
    fn epoch(&self) -> u32 {
        self.context.epoch.map_or(u32::MAX, |f| unsafe { f() })
    }
    fn enter(&mut self) -> u32 {
        self.context.enter.map_or(0, |f| unsafe { f() })
    }
    fn exit(&mut self, flags: u32) {
        if let Some(f) = self.context.exit {
            unsafe { f(flags) }
        }
    }
}
unsafe fn raw<'a>(
    context: *const Context,
    input: *const u8,
    count: u32,
    mode: u32,
) -> Option<Raw<'a>> {
    if context.is_null()
        || context as usize & (core::mem::align_of::<Context>() - 1) != 0
        || count > 640
        || count != 0 && input.is_null()
    {
        return None;
    }
    let context = unsafe { &*context };
    context.publish?;
    for i in 0..9 {
        let required = i < 3 || i == 7 || context.flags & 1 != 0;
        if required && (context.slots[i].is_null() || i < 6 && context.slots[i] as usize & 3 != 0) {
            return None;
        }
    }
    if mode != 0 && (context.flags & 1 == 0 || context.slots[8].is_null()) {
        return None;
    }
    if mode != 0 && context.flags & 2 == 0 {
        context.epoch?;
        context.enter?;
        context.exit?;
    }
    if mode == 2 {
        context.now?;
        context.idle?;
    }
    Some(Raw {
        context,
        input,
        count: count as usize,
    })
}
#[no_mangle]
pub extern "C" fn fm1_rust_usb_next_epoch(previous: u32) -> u32 {
    usb_tx::next_epoch(previous)
}
/// # Safety
/// Context and all mandatory roots cover live disjoint fixed buffers/initialized
/// scalar indexes, serialized CPU0 queue roles. Controller has positively
/// released the 64-byte endpoint before rewriting; no timeout grants reuse.
/// Publish is a no-reentry leaf. Only the local descriptor is immutably borrowed;
/// no raw root is retained. DMA may retain its original endpoint root after return.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_tx_fill(context: *const Context) -> i32 {
    let Some(mut io) = (unsafe { raw(context, core::ptr::null(), 0, 0) }) else {
        return -1;
    };
    let sysex = io.context.flags & 1 != 0;
    usb_tx::fill(&mut io, sysex) as i32
}
/// # Safety
/// Same live/root/role contract as fill. Input covers count initialized bytes
/// unchanged for this call. One foreground producer owns the SysEx stream.
/// Idle may consume/poll via disjoint scoped raw wrappers but cannot retain,
/// modify the descriptor/input or start another producer. No wait occurs under
/// an IRQ mask here. App epoch callback coherently returns the active reset
/// generation or MAX while enter/exit excludes CPU0 IRQ publication and restores
/// every incoming ICFG bit. An incoming disabled gate may not wait for the timer
/// consumer. Loader bit 1 selects the explicit legacy polled policy and needs no
/// app IRQ/epoch callbacks. count<=640(blocking) or 192(try); invalid is no-mutation.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_tx_send(
    context: *const Context,
    input: *const u8,
    count: u32,
    blocking: u32,
) -> i32 {
    if blocking == 0 && count > 192 {
        return -1;
    }
    let Some(mut io) = (unsafe { raw(context, input, count, if blocking != 0 { 2 } else { 1 }) })
    else {
        return -1;
    };
    let legacy = io.context.flags & 2 != 0;
    let success = if legacy {
        usb_tx::legacy_send(&mut io, count as usize, blocking != 0)
    } else {
        usb_tx::send(&mut io, count as usize, blocking != 0)
    };
    if success {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::{Cell, UnsafeCell};
    use std::cell::RefCell;
    std::thread_local! {
        static ROOT:Cell<*const Context>=const{Cell::new(core::ptr::null())};
        static CLOCK:Cell<u32>=const{Cell::new(u32::MAX-3)};
        static WORDS:RefCell<Vec<u32>>=const{RefCell::new(Vec::new())};
        static IRQ:Cell<u32>=const{Cell::new(0x100)};
        static IDLES:Cell<u32>=const{Cell::new(0)};
    }
    unsafe extern "C" fn publish() {}
    unsafe extern "C" fn epoch() -> u32 {
        0
    }
    unsafe extern "C" fn enter() -> u32 {
        IRQ.with(|v| {
            let old = v.get();
            v.set(old & !256);
            old
        })
    }
    unsafe extern "C" fn exit(flags: u32) {
        IRQ.with(|v| v.set(flags));
    }
    unsafe extern "C" fn now() -> u32 {
        CLOCK.with(Cell::get)
    }
    unsafe extern "C" fn consume() {
        ROOT.with(|root| unsafe {
            let context = root.get();
            let n = fm1_rust_usb_tx_fill(context);
            assert!(n >= 0);
            let out = (*context).slots[7];
            WORDS.with(|words| {
                for i in (0..n as usize).step_by(4) {
                    words.borrow_mut().push(u32::from_le_bytes([
                        out.add(i).read(),
                        out.add(i + 1).read(),
                        out.add(i + 2).read(),
                        out.add(i + 3).read(),
                    ]));
                }
            });
        });
    }
    unsafe extern "C" fn idle() {
        assert_eq!(IRQ.with(Cell::get) & 256, 256);
        IDLES.with(|n| n.set(n.get() + 1));
        CLOCK.with(|n| n.set(n.get().wrapping_add(1)));
        unsafe { consume() };
    }
    #[test]
    fn retained_roots_survive_a_scoped_producer_reentered_only_by_its_consumer() {
        let midi = UnsafeCell::new([0u32; 64]);
        let sysex = UnsafeCell::new([0u32; 64]);
        let endpoint = UnsafeCell::new([0u8; 64]);
        let message = UnsafeCell::new([0u8; 640]);
        let indexes = UnsafeCell::new([0u32; 4]);
        let flags = UnsafeCell::new([0u8, 1]);
        let words = indexes.get().cast::<u32>();
        let bytes = flags.get().cast::<u8>();
        // SAFETY: disjoint live foreign roots directly from UnsafeCell, no Rust
        // payload/endpoint/index views. The model backend releases each packet
        // before its next fill. Its descriptor root lives only for this test call.
        unsafe {
            let base = u32::MAX - 4;
            words.add(2).write(base.wrapping_add(64));
            words.add(3).write(base);
            for i in 0..64 {
                sysex
                    .get()
                    .cast::<u32>()
                    .add((base.wrapping_add(i as u32) % 64) as usize)
                    .write(0x10000 + i as u32);
            }
            for i in 0..640 {
                message.get().cast::<u8>().add(i).write(i as u8);
            }
            let context = Context {
                flags: 1,
                slots: [
                    midi.get().cast(),
                    words.cast(),
                    words.add(1).cast(),
                    sysex.get().cast(),
                    words.add(2).cast(),
                    words.add(3).cast(),
                    bytes,
                    endpoint.get().cast(),
                    bytes.add(1),
                ],
                publish: Some(publish),
                now: Some(now),
                idle: Some(idle),
                epoch: Some(epoch),
                enter: Some(enter),
                exit: Some(exit),
            };
            let root = core::ptr::addr_of!(context);
            ROOT.with(|v| v.set(root));
            WORDS.with(|v| v.borrow_mut().clear());
            IDLES.with(|v| v.set(0));
            assert_eq!(fm1_rust_usb_tx_send(root, message.get().cast(), 640, 1), 0);
            assert_eq!(bytes.read(), 0);
            assert!(IDLES.with(Cell::get) > 0);
            while words.add(2).read() != words.add(3).read() {
                consume();
            }
            let mut expected: Vec<u32> = (0..64).map(|i| 0x10000 + i).collect();
            expected.extend(
                (0..640)
                    .step_by(3)
                    .map(|i| usb_tx::packet(640, i, |at| at as u8).unwrap().0),
            );
            WORDS.with(|v| assert_eq!(*v.borrow(), expected));
            IRQ.with(|v| v.set(0));
            let before = IDLES.with(Cell::get);
            assert_eq!(fm1_rust_usb_tx_send(root, message.get().cast(), 640, 1), -1);
            assert_eq!(IRQ.with(Cell::get), 0);
            assert_eq!(IDLES.with(Cell::get), before);
            IRQ.with(|v| v.set(256));
            assert_eq!(fm1_rust_usb_tx_send(root, message.get().cast(), 641, 1), -1);
            assert_eq!(fm1_rust_usb_tx_send(root, message.get().cast(), 193, 0), -1);
            assert_eq!(fm1_rust_usb_tx_fill(core::ptr::null()), -1);
            ROOT.with(|v| v.set(core::ptr::null()));
        }
    }
}
