// SPDX-License-Identifier: GPL-3.0-only
//! Scoped raw foreign collector roots. No reference into endpoint/ring/payload
//! memory is created, and no pointer is retained beyond its synchronous call.
use core::mem::align_of;
use fm1_hal::usb_transport::{self, Buffer, Io, Scalar};
#[repr(C)]
pub struct Context {
    flags: u32,
    slots: [*mut u8; 21],
    publish: Option<unsafe extern "C" fn()>,
}
const _: [(); 23 * core::mem::size_of::<*mut u8>()] = [(); core::mem::size_of::<Context>()];
struct Raw<'a> {
    context: &'a Context,
    endpoint: *const u8,
    count: usize,
}
impl Raw<'_> {
    fn location(key: Scalar) -> (usize, usize, bool) {
        use Scalar::*;
        match key {
            ShortLength => (1, 0, false),
            ShortOn => (1, 1, false),
            Uboot => (1, 2, false),
            OtaRequest => (1, 3, false),
            Telemetry => (1, 4, false),
            ParameterPosition => (3, 0, true),
            ParameterOn => (4, 0, false),
            DxPosition => (9, 0, true),
            DxOn => (10, 0, false),
            DxReady => (11, 0, false),
            DxKind => (12, 0, false),
            OtaPosition => (14, 0, true),
            OtaCollect => (15, 0, false),
            OtaReady => (16, 0, false),
            OtaLength => (17, 0, true),
        }
    }
    fn payload(buffer: Buffer) -> (usize, usize) {
        match buffer {
            Buffer::Short => (0, 16),
            Buffer::Parameter => (2, 16),
            Buffer::Dx => (8, 4112),
            Buffer::Ota => (13, 640),
        }
    }
    fn queue(&mut self, slot: usize, capacity: u32, event: u32) {
        // SAFETY: validated aligned external roots; producer execution is
        // serialized by the caller's CPU0 poll/IRQ ownership. Consumer touches
        // only published slots; publication is the existing C publication barrier.
        unsafe {
            let write = self.context.slots[slot + 1].cast::<u32>();
            let w = write.read_volatile();
            let r = self.context.slots[slot + 2].cast::<u32>().read_volatile();
            if w.wrapping_sub(r) < capacity {
                self.context.slots[slot]
                    .cast::<u32>()
                    .add((w % capacity) as usize)
                    .write_volatile(event);
                self.publish();
                write.write_volatile(w.wrapping_add(1));
            }
        }
    }
}
impl Io for Raw<'_> {
    fn get(&self, key: Scalar) -> u32 {
        let (slot, offset, word) = Self::location(key);
        // SAFETY: accepted Context supplies a live initialized scalar of its
        // declared width. No Rust scalar reference exists across an IRQ.
        unsafe {
            let p = self.context.slots[slot].add(offset);
            if word {
                p.cast::<u32>().read_volatile()
            } else {
                p.read_volatile().into()
            }
        }
    }
    fn set(&mut self, key: Scalar, value: u32) {
        let (slot, offset, word) = Self::location(key);
        // SAFETY: same disjoint scalar contract as get; no payload span borrow.
        unsafe {
            let p = self.context.slots[slot].add(offset);
            if word {
                p.cast::<u32>().write_volatile(value)
            } else {
                p.write_volatile(value as u8)
            }
        }
    }
    fn read(&self, buffer: Buffer, index: usize) -> u8 {
        let (slot, capacity) = Self::payload(buffer);
        if index >= capacity || self.context.slots[slot].is_null() {
            return 0;
        }
        // SAFETY: independently checked payload bound and live external root.
        unsafe { self.context.slots[slot].add(index).read_volatile() }
    }
    fn write(&mut self, buffer: Buffer, index: usize, value: u8) {
        let (slot, capacity) = Self::payload(buffer);
        if index >= capacity || self.context.slots[slot].is_null() {
            return;
        }
        // SAFETY: checked bound; busy-ready collectors never write leased payload.
        unsafe { self.context.slots[slot].add(index).write_volatile(value) }
    }
    fn classify_dx(&self, length: usize) -> u8 {
        if length > 4112 || self.context.slots[8].is_null() {
            return 0;
        }
        // Shared safe format classification asks only for bounded raw header
        // bytes; there is no reference into a ready/busy external payload.
        fm1_formats::voice::classify_legacy_with(length, |index| self.read(Buffer::Dx, index))
            .map_or(0, |kind| kind as u8)
    }
    fn publish(&mut self) {
        // SAFETY: validated leaf callback only emits the existing C memory
        // barrier. It cannot retain/reenter any context/collector/endpoint.
        if let Some(publish) = self.context.publish {
            unsafe { publish() }
        }
    }
    fn parameter(&mut self, event: u32) {
        self.queue(5, 32, event);
    }
    fn midi(&mut self, packet: u32) {
        self.queue(18, 64, packet);
    }
    fn endpoint(&self, index: usize) -> u8 {
        if index >= self.count {
            return 0;
        }
        // SAFETY: count<=64, raw endpoint is live; it is not a Rust slice.
        unsafe { self.endpoint.add(index).read_volatile() }
    }
}
unsafe fn raw<'a>(context: *const Context, input: *const u8, count: u32) -> Option<Raw<'a>> {
    if context.is_null()
        || context as usize & (align_of::<Context>() - 1) != 0
        || count > 64
        || count != 0 && input.is_null()
    {
        return None;
    }
    // SAFETY: caller supplies initialized immutable stack descriptor covering the
    // call. All roots are validated before their first target access below.
    let context = unsafe { &*context };
    let required = |slot: usize| {
        slot <= 1
            || slot >= 18
            || context.flags & 1 != 0 && (2..=12).contains(&slot)
            || context.flags & 2 != 0 && (13..=17).contains(&slot)
    };
    for slot in 0..21 {
        if required(slot)
            && (context.slots[slot].is_null()
                || matches!(slot, 3 | 5 | 6 | 7 | 9 | 14 | 17 | 18 | 19 | 20)
                    && context.slots[slot] as usize & 3 != 0)
        {
            return None;
        }
    }
    context.publish?;
    Some(Raw {
        context,
        endpoint: input,
        count: count as usize,
    })
}
/// # Safety
/// Context is a live immutable initialized descriptor with the 21 documented
/// disjoint roots. Mandatory scalar slots are initialized and naturally aligned;
/// buffers cover their fixed capacity and remain alive. One CPU0 poll owns the
/// collector; foreground may observe/release only ready payloads and consumed
/// ring slots. No concurrent/reentrant producer, context modification or payload
/// overwrite is allowed. Publish is a no-reentry leaf barrier. Input covers count
/// readable bytes under the existing endpoint handshake; no pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_sysex(
    context: *const Context,
    input: *const u8,
    count: u32,
) -> i32 {
    let Some(mut io) = (unsafe { raw(context, input, count) }) else {
        return -1;
    };
    let flags = io.context.flags;
    for i in 0..count as usize {
        let byte = io.endpoint(i);
        usb_transport::sysex(&mut io, flags, byte);
    }
    0
}
/// # Safety
/// Same external context/endpoint/serialization contract as fm1_rust_usb_sysex.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_device_receive(
    context: *const Context,
    input: *const u8,
    count: u32,
) -> i32 {
    let Some(mut io) = (unsafe { raw(context, input, count) }) else {
        return -1;
    };
    let flags = io.context.flags;
    usb_transport::receive(&mut io, flags, count as usize);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::UnsafeCell;
    unsafe extern "C" fn barrier() {}
    #[test]
    fn retained_foreign_roots_keep_busy_payload_length_and_packet_bounds() {
        let short = UnsafeCell::new([0u8; 16]);
        let parameter = UnsafeCell::new([0u8; 16]);
        let dx = UnsafeCell::new([0u8; 4112]);
        let ota = UnsafeCell::new([0u8; 640]);
        let pq = UnsafeCell::new([0u32; 32]);
        let mq = UnsafeCell::new([0u32; 64]);
        let words = UnsafeCell::new([0u32; 8]);
        let bytes = UnsafeCell::new([0u8; 11]);
        let input = UnsafeCell::new([0u8; 64]);
        let w = words.get().cast::<u32>();
        let b = bytes.get().cast::<u8>();
        let mut context = Context {
            flags: 3,
            slots: [core::ptr::null_mut(); 21],
            publish: Some(barrier),
        };
        // SAFETY: disjoint live fixed allocations; all roots originate directly
        // at UnsafeCell::get, never temporary Rust payload/endpoint references.
        unsafe {
            context.slots = [
                short.get().cast(),
                b,
                parameter.get().cast(),
                w.cast(),
                b.add(5),
                pq.get().cast(),
                w.add(1).cast(),
                w.add(2).cast(),
                dx.get().cast(),
                w.add(3).cast(),
                b.add(6),
                b.add(7),
                b.add(8),
                ota.get().cast(),
                w.add(4).cast(),
                b.add(9),
                b.add(10),
                w.add(5).cast(),
                mq.get().cast(),
                w.add(6).cast(),
                w.add(7).cast(),
            ];
            let root = input.get().cast::<u8>();
            let retained = dx.get().cast::<u8>();
            for (i, byte) in [0xf0, 0x7d, 0x50, 0x60, 1, 4, 0, 0xf7]
                .into_iter()
                .enumerate()
            {
                root.add(i).write(byte);
            }
            assert_eq!(fm1_rust_usb_sysex(core::ptr::addr_of!(context), root, 8), 0);
            assert_eq!(w.add(3).read(), 6);
            assert_eq!(b.add(7).read(), 2);
            assert_eq!(b.add(8).read(), 6);
            let pending = [0x7d, 0x50, 0x60, 1, 4, 0];
            for (i, byte) in [0xf0, 0x7d, 0x50, 0x50, 1, 1, 2, 3, 4, 0xf7]
                .into_iter()
                .enumerate()
            {
                root.add(i).write(byte);
            }
            assert_eq!(
                fm1_rust_usb_sysex(core::ptr::addr_of!(context), root, 10),
                0
            );
            assert_eq!(w.add(3).read(), 6);
            assert_eq!(b.add(7).read(), 2);
            for (i, byte) in pending.into_iter().enumerate() {
                assert_eq!(retained.add(i).read(), byte);
            }
            assert_eq!(w.add(1).read(), 1);
            assert_eq!(pq.get().cast::<u32>().read(), 1 << 28 | 130 << 14 | 388);
            for (i, byte) in [0x99, 0x90, 60, 127, 15, 0xf8, 0, 0]
                .into_iter()
                .enumerate()
            {
                root.add(i).write(byte);
            }
            assert_eq!(
                fm1_rust_usb_device_receive(core::ptr::addr_of!(context), root, 8),
                0
            );
            assert_eq!(w.add(6).read(), 2);
            assert_eq!(mq.get().cast::<u32>().read(), 0x7f3c9099);
            assert_eq!(mq.get().cast::<u32>().add(1).read(), 0x0000f80f);
            assert_eq!(
                fm1_rust_usb_device_receive(core::ptr::addr_of!(context), root, 65),
                -1
            );
            assert_eq!(fm1_rust_usb_sysex(core::ptr::null(), root, 8), -1);
            context.slots[19] = w.add(6).cast::<u8>().add(1);
            assert_eq!(
                fm1_rust_usb_device_receive(core::ptr::addr_of!(context), root, 8),
                -1
            );
            assert_eq!(w.add(6).read(), 2);
        }
    }
}
