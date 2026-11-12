// SPDX-License-Identifier: GPL-3.0-only
//! Scoped TRS state, raw retained UART DMA slots and a disjoint MIDI queue leaf.
use fm1_hal::midi::{self, Trs, TrsIo};
type Send = unsafe extern "C" fn(u32) -> i32;
struct Input {
    ring: *mut u8,
    send: Option<Send>,
}
impl TrsIo for Input {
    fn read(&mut self, index: u32) -> u8 {
        debug_assert!(index < midi::TRS_RING);
        // SAFETY: core bounds every index; the retained allocation stays live.
        unsafe { self.ring.add(index as usize).read_volatile() }
    }
    fn write(&mut self, index: u32, value: u8) {
        debug_assert!(index < midi::TRS_RING);
        // SAFETY: bounded raw slot; no reference into the DMA ring is formed.
        unsafe { self.ring.add(index as usize).write_volatile(value) };
    }
    fn send(&mut self, packet: u32) -> bool {
        self.send.is_some_and(|send| unsafe { send(packet) != 0 })
    }
}
const _: [(); 24] = [(); core::mem::size_of::<Trs>()];
/// # Safety
/// ring identifies 128 live external UART-DMA bytes. Initialization occurs before
/// starting UART DMA/polling; no reference aliases those bytes. Nothing escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_trs_clear(ring: *mut u8) {
    if !ring.is_null() {
        midi::clear(&mut Input { ring, send: None });
    }
}
/// # Safety
/// state is an initialized aligned exclusive 24-byte CPU0 parser. Calls serialize
/// with the MIDI poll context; the leaf send callback touches only the disjoint
/// existing MIDI queue, never re-enters or retains this state/ring/pointer.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_trs_byte(state: *mut Trs, byte: u32, send: Send) {
    if state.is_null() {
        return;
    }
    unsafe { &mut *state }.byte(byte, &mut |packet| unsafe { send(packet) != 0 });
}
/// # Safety
/// Same exclusive parser/callback contract as byte. ring identifies 128 retained
/// external DMA bytes; DMA may advance between any raw read/write. No reference
/// into that allocation is formed, and no pointer survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_trs_take(
    state: *mut Trs,
    ring: *mut u8,
    count: u32,
    send: Send,
) -> u32 {
    if state.is_null() || ring.is_null() {
        return 0;
    }
    u32::from(unsafe { &mut *state }.take(
        count,
        &mut Input {
            ring,
            send: Some(send),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicU32, Ordering};
    static PACKET: AtomicU32 = AtomicU32::new(0);
    unsafe extern "C" fn send(packet: u32) -> i32 {
        PACKET.store(packet, Ordering::Relaxed);
        1
    }
    #[test]
    fn retained_uart_root_survives_scoped_parser_calls_without_dma_references() {
        let ring = UnsafeCell::new([0u8; 128]);
        let raw = ring.get().cast::<u8>();
        let mut state = Trs::default();
        // SAFETY: one serialized owner, disjoint initialized parser, 128 live
        // externally updated bytes, and a leaf callback with separate storage.
        unsafe {
            fm1_rust_usb_trs_clear(raw);
            for i in 0..128 {
                assert_eq!(raw.add(i).read(), midi::TRS_EMPTY);
            }
            raw.write(0x91);
            raw.add(1).write(60);
            raw.add(2).write(100);
            assert_eq!(fm1_rust_usb_trs_take(&mut state, raw, 2, send), 1);
            assert_eq!(PACKET.load(Ordering::Relaxed), 0x643c9109);
            assert_eq!(state.bytes, 3);
            raw.add(3).write(62);
            raw.add(4).write(90);
            assert_eq!(fm1_rust_usb_trs_take(&mut state, raw, 0, send), 1);
            assert_eq!(PACKET.load(Ordering::Relaxed), 0x5a3e9109);
            assert_eq!(state.messages, 2);
        }
    }
}
