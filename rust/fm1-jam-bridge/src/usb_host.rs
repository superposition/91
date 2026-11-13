// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::usb_host::{self, Midi, Receive};
/// # Safety
/// `output` is an aligned writable 16-byte C result (possibly uninitialized),
/// disjoint from `count` initialized CPU-owned input bytes. No DMA/callback may
/// mutate the copied configuration during this scoped call. No pointer remains.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_host_parse(
    input: *const u8,
    count: u32,
    output: *mut Midi,
) -> i32 {
    if output.is_null() || output as usize & (core::mem::align_of::<Midi>() - 1) != 0 {
        return 0;
    }
    if count as usize > usb_host::CONFIG_BYTES || (input.is_null() && count != 0) {
        // SAFETY: result validity/alignment is a caller contract checked above.
        unsafe { output.write(Midi::default()) };
        return 0;
    }
    let start = input as usize;
    let end = start.checked_add(count as usize);
    let destination = output as usize;
    let result_end = destination.checked_add(core::mem::size_of::<Midi>());
    if end.is_none() || result_end.is_none() {
        return 0;
    }
    if count != 0 && start < result_end.unwrap_or(0) && destination < end.unwrap_or(0) {
        return 0;
    }
    let bytes = if count == 0 {
        &[]
    } else {
        // SAFETY: initialized copied configuration, bounded and disjoint;
        // neither DMA nor another context writes this CPU-owned span here.
        unsafe { core::slice::from_raw_parts(input, count as usize) }
    };
    let result = usb_host::parse(bytes);
    let found = i32::from(result.found);
    // SAFETY: raw write initializes every byte, including explicit padding;
    // no reference to an uninitialized C result is ever constructed.
    unsafe { output.write(result) };
    found
}
#[no_mangle]
pub extern "C" fn fm1_rust_usb_host_packet(word: u32) -> u32 {
    usb_host::packet(word)
}
#[repr(C)]
pub struct Context {
    input: *const u8,
    data: *mut u32,
    write: *mut u32,
    read: *const u32,
    publish: Option<unsafe extern "C" fn()>,
}
const _: [(); 5 * core::mem::size_of::<*mut u8>()] = [(); core::mem::size_of::<Context>()];
struct Raw<'a>(&'a Context);
impl Receive for Raw<'_> {
    fn write(&self) -> u32 {
        // SAFETY: serialized fixed producer/consumer roots; no typed index view.
        unsafe { self.0.write.read_volatile() }
    }
    fn read(&self) -> u32 {
        // SAFETY: independent consumer publication, no typed index view.
        unsafe { self.0.read.read_volatile() }
    }
    fn byte(&self, index: usize) -> u8 {
        // SAFETY: count <=64, known-ready retained RX root, cache sync precedes
        // entry; no next RX request occurs until return. No DMA slice is made.
        unsafe { self.0.input.add(index).read_volatile() }
    }
    fn put(&mut self, index: usize, value: u32) {
        // SAFETY: bounded free slot, sole serialized producer, no slot reference.
        unsafe { self.0.data.add(index).write_volatile(value) };
    }
    fn publish(&mut self) {
        if let Some(publish) = self.0.publish {
            // SAFETY: leaf barrier only; no producer, engine or view reentry.
            unsafe { publish() };
        }
    }
    fn set_write(&mut self, value: u32) {
        // SAFETY: exclusive producer role, after payload/barrier publication.
        unsafe { self.0.write.write_volatile(value) };
    }
}
/// # Safety
/// Scoped initialized immutable context; live disjoint RX<=64 bytes, queue64
/// words and aligned index roots. RX is known complete/cache-synced and remains
/// retained until return; queue roles are serialized, publish is a leaf barrier.
/// No pointer or foreign-memory reference is retained or produced.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_host_receive(context: *const Context, count: u32) -> i32 {
    if context.is_null() || context as usize & (core::mem::align_of::<Context>() - 1) != 0 {
        return -1;
    }
    // SAFETY: caller-stack descriptor is initialized and immutable through call.
    let c = unsafe { &*context };
    if count > 64
        || c.input.is_null()
        || c.data.is_null()
        || c.write.is_null()
        || c.read.is_null()
        || c.data as usize & 3 != 0
        || c.write as usize & 3 != 0
        || c.read as usize & 3 != 0
    {
        return -1;
    }
    usb_host::receive(&mut Raw(c), count as usize).map_or(-1, |n| n as i32)
}

#[repr(C)]
pub struct TxContext {
    data: *const u32,
    write: *const u32,
    read: *mut u32,
    output: *mut u8,
    clock: *const u32,
    publish: Option<unsafe extern "C" fn()>,
    lease: *mut u8,
}
const _: [(); 7 * core::mem::size_of::<*mut u8>()] = [(); core::mem::size_of::<TxContext>()];
struct RawTx<'a>(&'a TxContext);
// SAFETY: validated live/aligned fixed roots, serialized queue/USB roles; safe
// core bounds each slot/output, preserves publication and clock-read order.
// Only the private initialized8B metadata packet is mutably referenced.
impl usb_host::Transmit for RawTx<'_> {
    fn write(&self) -> u32 {
        unsafe { self.0.write.read_volatile() }
    }
    fn read(&self) -> u32 {
        unsafe { self.0.read.read_volatile() }
    }
    fn set_read(&mut self, value: u32) {
        unsafe { self.0.read.write_volatile(value) }
    }
    fn word(&self, index: usize) -> u32 {
        unsafe { self.0.data.add(index).read_volatile() }
    }
    fn output(&mut self, index: usize, value: u8) {
        unsafe { self.0.output.add(index).write_volatile(value) }
    }
    fn publish(&mut self) {
        if let Some(f) = self.0.publish {
            unsafe { f() }
        }
    }
    fn now(&self) -> u32 {
        unsafe { self.0.clock.read_volatile() }
    }
    fn grant(&mut self, csr: u32) -> bool {
        use fm1_hal::usb_endpoint::{transition, ALLOWED, BUSY, CLEAR};
        // SAFETY: the serialized USB context owns this disjoint lease byte;
        // no whole USB state reference or C callback is made while state is borrowed.
        let current = unsafe { self.0.lease.read_volatile() };
        let plan = transition(
            u32::from(current),
            1,
            if csr & 3 != 0 { BUSY } else { CLEAR },
        );
        unsafe { self.0.lease.write_volatile(plan as u8) };
        plan & ALLOWED != 0
    }
}
/// # Safety
/// Initialized immutable context, disjoint initialized mutable8B caller-stack
/// state, existing queue64 words, aligned index/clock roots and output64 bytes.
/// Queue roles are serialized; source reuse uses the disjoint raw USB lease
/// and the existing safe scalar endpoint policy. No callback may enter this
/// producer, application, engine or foreign-memory views. No DMA/queue reference
/// is formed, no root retained. The caller arms DMA only after this call returns.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_host_transmit(
    context: *const TxContext,
    state: *mut usb_host::TxState,
    csr: u32,
    maximum: u32,
) -> u32 {
    if context.is_null()
        || state.is_null()
        || context as usize & (core::mem::align_of::<TxContext>() - 1) != 0
        || state as usize & 3 != 0
    {
        return 2;
    }
    // SAFETY: immutable initialized caller descriptor, disjoint from mutable state.
    let c = unsafe { &*context };
    if c.data.is_null()
        || c.write.is_null()
        || c.read.is_null()
        || c.output.is_null()
        || c.clock.is_null()
        || c.data as usize & 3 != 0
        || c.write as usize & 3 != 0
        || c.read as usize & 3 != 0
        || c.clock as usize & 3 != 0
        || c.lease.is_null()
    {
        return 2;
    }
    // SAFETY: initialized private caller-stack metadata only; callbacks cannot
    // access it and every foreign queue/DMA/clock access uses raw roots below.
    let state = unsafe { &mut *state };
    match usb_host::transmit(&mut RawTx(c), state, csr, maximum) {
        usb_host::TxAction::Wait => 0,
        usb_host::TxAction::Retry => 1,
        usb_host::TxAction::Fail => 2,
        usb_host::TxAction::Send(n) => 3 | (u32::from(n) << 8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{cell::UnsafeCell, mem::MaybeUninit};
    #[test]
    fn copied_configuration_initializes_every_result_byte_including_reserved() {
        let config = [9, 4, 6, 1, 1, 1, 3, 0, 0, 7, 5, 0x82, 2, 64, 0, 0];
        let mut output = MaybeUninit::<Midi>::uninit();
        // SAFETY: disjoint live initialized input / writable uninitialized output.
        assert_eq!(
            unsafe { fm1_rust_usb_host_parse(config.as_ptr(), 16, output.as_mut_ptr()) },
            1
        );
        // SAFETY: successful complete raw output write, no implicit padding.
        let bytes = unsafe { core::slice::from_raw_parts(output.as_ptr().cast::<u8>(), 16) };
        assert_eq!(bytes, &[1, 0, 6, 1, 2, 2, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0]);
        // SAFETY: zero input length permits null; valid output still initialized.
        assert_eq!(
            unsafe { fm1_rust_usb_host_parse(core::ptr::null(), 0, output.as_mut_ptr()) },
            0
        );
        assert_eq!(unsafe { output.assume_init() }, Midi::default());
    }
    unsafe extern "C" fn publish() {}
    #[test]
    fn retained_rx_and_queue_roots_survive_scoped_calls_without_foreign_views() {
        let input = UnsafeCell::new([0u8; 64]);
        let queue = UnsafeCell::new([0u32; 64]);
        let write = UnsafeCell::new(0u32);
        let read = UnsafeCell::new(0u32);
        let context = Context {
            input: input.get().cast(),
            data: queue.get().cast(),
            write: write.get(),
            read: read.get(),
            publish: Some(publish),
        };
        // SAFETY: model external hardware/C storage through its original cell
        // roots, without manufacturing persistent/temporary foreign slices.
        unsafe {
            for i in 0..16 {
                input
                    .get()
                    .cast::<u32>()
                    .add(i)
                    .write_unaligned(0x7f40_90a9);
            }
            assert_eq!(fm1_rust_usb_host_receive(&context, 64), 16);
            assert_eq!(write.get().read(), 16);
            for i in 0..16 {
                assert_eq!(queue.get().cast::<u32>().add(i).read(), 0x7f40_9009);
            }
            write.get().write(49);
            assert_eq!(fm1_rust_usb_host_receive(&context, 4), -1);
            assert_eq!(write.get().read(), 49);
            input.get().cast::<u32>().write_unaligned(0xffff_f8ff);
            read.get().write(1);
            assert_eq!(fm1_rust_usb_host_receive(&context, 4), 1);
            assert_eq!(write.get().read(), 50);
            assert_eq!(queue.get().cast::<u32>().add(49).read(), 0xffff_f80f);
            assert_eq!(fm1_rust_usb_host_receive(&context, 65), -1);
            assert_eq!(write.get().read(), 50);
        }
    }
    #[test]
    fn host_tx_retains_raw_source_through_busy_then_reuses_only_after_retirement() {
        use fm1_hal::{usb_endpoint as ep, usb_host::TxState};
        let queue = UnsafeCell::new([0x7f40_9009u32; 64]);
        let output = UnsafeCell::new([0xa5u8; 64]);
        let write = UnsafeCell::new(16u32);
        let read = UnsafeCell::new(0u32);
        let clock = UnsafeCell::new(123u32);
        let lease = UnsafeCell::new(0u8);
        let context = TxContext {
            data: queue.get().cast(),
            write: write.get(),
            read: read.get(),
            output: output.get().cast(),
            clock: clock.get(),
            publish: Some(publish),
            lease: lease.get(),
        };
        let mut state = TxState::default();
        // SAFETY: original cell roots model disjoint C/retained DMA storage;
        // metadata is private initialized caller storage, with no foreign views.
        unsafe {
            assert_eq!(
                fm1_rust_usb_host_transmit(&context, &mut state, 0, 64),
                3 | (64 << 8)
            );
            assert_eq!((state.count, state.timestamp), (16, 123));
            assert_eq!(read.get().read(), 0);
            lease.get().write(ep::transition(0, 1, ep::ARMED) as u8);
            let submitted = output.get().read();
            assert_eq!(fm1_rust_usb_host_transmit(&context, &mut state, 1, 64), 0);
            assert_eq!(output.get().read(), submitted);
            assert_eq!(read.get().read(), 0);
            assert_eq!(fm1_rust_usb_host_transmit(&context, &mut state, 0, 64), 0);
            assert_eq!(read.get().read(), 16);
            assert_eq!(state.count, 0);
            assert_eq!(lease.get().read(), 0);
            queue.get().cast::<u32>().add(16).write(0x0000_fa0f);
            write.get().write(17);
            clock.get().write(124);
            assert_eq!(
                fm1_rust_usb_host_transmit(&context, &mut state, 0, 64),
                3 | (4 << 8)
            );
            assert_eq!(output.get().cast::<u32>().read_unaligned(), 0x0000_fa0f);
            lease
                .get()
                .write(ep::transition(0, 1, ep::UNCERTAIN_ARM) as u8);
            clock.get().write(125);
            let submitted = output.get().read();
            assert_eq!(fm1_rust_usb_host_transmit(&context, &mut state, 0, 64), 2);
            assert_eq!(read.get().read(), 16);
            assert_eq!(state.count, 1);
            assert_eq!(output.get().read(), submitted);
        }
    }
}
