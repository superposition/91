// SPDX-License-Identifier: GPL-3.0-only
//! Scoped raw CPU0 queue roles. No references into foreign slots/indexes/output.
use fm1_hal::usb_queues::{self, Io};
struct Ring {
    data: *mut u32,
    write: *mut u32,
    read: *mut u32,
    publish: unsafe extern "C" fn(),
}
// SAFETY for raw IO below: ring() validates non-null/aligned roots; the
// unsafe ABI caller supplies disjoint live capacity/role ownership. Every slot
// access is independently bounded and no foreign reference is constructed.
impl Io for Ring {
    fn write_index(&self) -> u32 {
        unsafe { self.write.read_volatile() }
    }
    fn read_index(&self) -> u32 {
        unsafe { self.read.read_volatile() }
    }
    fn set_write(&mut self, index: u32) {
        unsafe { self.write.write_volatile(index) }
    }
    fn set_read(&mut self, index: u32) {
        unsafe { self.read.write_volatile(index) }
    }
    fn word(&self, index: usize) -> u32 {
        if index >= 64 {
            return 0;
        }
        unsafe { self.data.add(index).read_volatile() }
    }
    fn put(&mut self, index: usize, value: u32) {
        if index < 64 {
            unsafe { self.data.add(index).write_volatile(value) }
        }
    }
    fn publish(&mut self) {
        unsafe { (self.publish)() }
    }
}
fn ring(
    data: *mut u32,
    write: *mut u32,
    read: *mut u32,
    publish: Option<unsafe extern "C" fn()>,
) -> Option<Ring> {
    if [data, write, read]
        .iter()
        .any(|p| p.is_null() || *p as usize & 3 != 0)
    {
        return None;
    }
    Some(Ring {
        data,
        write,
        read,
        publish: publish?,
    })
}
/// # Safety
/// Roots cover one initialized 64-word queue and disjoint aligned indexes, live
/// throughout this call. The caller holds whole-producer CPU0 IRQ exclusion
/// with exact saved-state restoration; no other writer/consumer may violate the
/// published-slot protocol. Callback is a leaf publication barrier; no queue
/// reference is constructed or retained. This is not a multi-thread backend.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_midi_push(
    data: *mut u32,
    write: *mut u32,
    read: *mut u32,
    value: u32,
    enabled: u32,
    publish: Option<unsafe extern "C" fn()>,
) -> i32 {
    let Some(mut ring) = ring(data, write, read, publish) else {
        return -1;
    };
    i32::from(usb_queues::push(&mut ring, enabled != 0, value))
}
/// # Safety
/// Same live/disjoint queue/index contract as push. One CPU0 consumer owns its
/// read index; an interrupt may produce only into free slots, with its own
/// scoped raw wrapper. Output is a disjoint live aligned writable word, possibly
/// uninitialized. Callback cannot retain/reenter the consumer; no pointer or
/// output/slot reference survives. No output is touched on empty/rejected input.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_midi_pop(
    data: *mut u32,
    write: *mut u32,
    read: *mut u32,
    output: *mut u32,
    publish: Option<unsafe extern "C" fn()>,
) -> i32 {
    if output.is_null() || output as usize & 3 != 0 {
        return -1;
    }
    let Some(mut ring) = ring(data, write, read, publish) else {
        return -1;
    };
    i32::from(usb_queues::pop(&mut ring, |value| unsafe {
        output.write_volatile(value)
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::UnsafeCell;
    unsafe extern "C" fn barrier() {}
    #[test]
    fn retained_raw_queue_roots_and_uninitialized_delivery_are_scoped() {
        let data = UnsafeCell::new([0u32; 64]);
        let indices = UnsafeCell::new([u32::MAX - 3; 2]);
        let root = data.get().cast::<u32>();
        let w = indices.get().cast::<u32>();
        // SAFETY: live disjoint initialized roots, serialized provider; raw
        // observer roots survive without any temporary slot/output reference.
        unsafe {
            let r = w.add(1);
            let mut output = core::mem::MaybeUninit::<u32>::uninit();
            assert_eq!(
                fm1_rust_usb_midi_pop(root, w, r, output.as_mut_ptr(), Some(barrier)),
                0
            );
            for i in 0..64 {
                assert_eq!(fm1_rust_usb_midi_push(root, w, r, i, 1, Some(barrier)), 1);
            }
            assert_eq!(fm1_rust_usb_midi_push(root, w, r, 99, 1, Some(barrier)), 0);
            for i in 0..64 {
                assert_eq!(
                    fm1_rust_usb_midi_pop(root, w, r, output.as_mut_ptr(), Some(barrier)),
                    1
                );
                assert_eq!(output.assume_init_read(), i);
            }
            assert_eq!(
                fm1_rust_usb_midi_pop(root, w, r, output.as_mut_ptr(), Some(barrier)),
                0
            );
            assert_eq!(fm1_rust_usb_midi_push(root, w, r, 99, 0, Some(barrier)), 0);
            assert_eq!(
                fm1_rust_usb_midi_push(root.cast::<u8>().add(1).cast(), w, r, 99, 1, Some(barrier)),
                -1
            );
            assert_eq!(w.read(), r.read());
        }
    }
}
