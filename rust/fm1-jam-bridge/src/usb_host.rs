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
}
