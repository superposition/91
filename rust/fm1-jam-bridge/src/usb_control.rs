// SPDX-License-Identifier: GPL-3.0-only
//! Raw EP0 publication of immutable descriptor bytes, with no endpoint borrow.
use fm1_hal::usb_descriptors::Descriptors;
#[no_mangle]
pub extern "C" fn fm1_rust_usb_ep0_start(length: u32, wanted: u32) -> u32 {
    fm1_hal::usb_control::start(length as u16, wanted as u16)
}
#[no_mangle]
pub extern "C" fn fm1_rust_usb_ep0_packet(left: u32, zlp: u32) -> u32 {
    fm1_hal::usb_control::packet(left as u16, zlp != 0)
}
#[no_mangle]
pub extern "C" fn fm1_rust_usb_descriptor_length(flags: u32, pid: u32, request: u32) -> u32 {
    Descriptors::new(flags, pid as u16)
        .length(request)
        .map_or(0, |n| n as u32)
}
/// SETUP words are copied scalar values, never an endpoint slice. The result
/// selects C's bounded publication/MMIO action after all Rust work has returned.
#[no_mangle]
pub extern "C" fn fm1_rust_usb_ep0_request(
    flags: u32,
    pid: u32,
    first: u32,
    second: u32,
    config: u32,
) -> u32 {
    fm1_hal::usb_control::request(
        flags,
        pid as u16,
        ((first & 255) << 8 | (first >> 8 & 255)) as u16,
        (first >> 16) as u16,
        second as u8,
        (second >> 16) as u16,
        config != 0,
    )
    .word()
}
/// # Safety
/// Output is a live writable external endpoint allocation covering capacity
/// bytes, serialized by its controller owner. No Rust reference aliases it.
/// DMA may retain/read the allocation; the caller supplies the existing endpoint
/// handshake and keeps all original spans alive. No pointer is retained here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_descriptor_copy(
    flags: u32,
    pid: u32,
    request: u32,
    offset: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if capacity > 64 || (capacity != 0 && output.is_null()) {
        return -1;
    }
    Descriptors::new(flags, pid as u16)
        .copy(
            request,
            offset as usize,
            capacity as usize,
            |index, byte| {
                // SAFETY: checked capacity bounds the core's visited prefix. A raw
                // volatile store creates no reference into the retained DMA allocation.
                unsafe { output.add(index).write_volatile(byte) };
            },
        )
        .map_or(-1, |n| n as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::UnsafeCell;
    #[test]
    fn retained_ep0_root_and_uninitialized_destination_need_no_mutable_dma_reference() {
        let bytes = UnsafeCell::new([0xa5u8; 70]);
        let root = bytes.get().cast::<u8>();
        let set = Descriptors::new(3, 1);
        // SAFETY: live disjoint external70-byte allocation, serialized producer;
        // raw observer roots survive every call without a temporary output ref.
        unsafe {
            let out = root.add(1);
            assert_eq!(fm1_rust_usb_descriptor_copy(3, 1, 0x0200, 63, out, 64), 64);
            assert_eq!(root.read(), 0xa5);
            assert_eq!(root.add(65).read(), 0xa5);
            for i in 0..64 {
                assert_eq!(out.add(i).read(), set.byte(0x0200, 63 + i).unwrap());
            }
            let old = out.read();
            assert_eq!(fm1_rust_usb_descriptor_copy(3, 1, 0x0303, 0, out, 64), -1);
            assert_eq!(out.read(), old);
            let mut destination = core::mem::MaybeUninit::<[u8; 18]>::uninit();
            assert_eq!(
                fm1_rust_usb_descriptor_copy(0, 2, 0x0100, 0, destination.as_mut_ptr().cast(), 18),
                18
            );
            let device = destination.assume_init();
            assert_eq!(device[10], 2);
            assert_eq!(device[11], 0);
        }
    }
}
