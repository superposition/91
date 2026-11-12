// SPDX-License-Identifier: GPL-3.0-only
//! Exclusive synchronous JS calls own the input/output windows. Memory is not
//! shared, there are no callbacks/imports, and no pointers escape this module.
use crate::api;
pub const CAP: usize = 1_100_000;
#[repr(C, align(8))]
struct Windows {
    before: [u8; 32],
    input: [u8; CAP],
    between: [u8; 32],
    output: [u8; CAP],
    after: [u8; 32],
}
static mut WINDOWS: Windows = Windows {
    before: [0; 32],
    input: [0; CAP],
    between: [0; 32],
    output: [0; CAP],
    after: [0; 32],
};
static mut COUNTERS: [u32; 128] = [0; 128];
#[no_mangle]
pub extern "C" fn codec_abi() -> u32 {
    // SAFETY: called once before exposing the windows by the synchronous client;
    // only disjoint static guard bytes are initialized, no input is touched.
    unsafe {
        for p in [
            core::ptr::addr_of_mut!(WINDOWS.before).cast::<u8>(),
            core::ptr::addr_of_mut!(WINDOWS.between).cast(),
            core::ptr::addr_of_mut!(WINDOWS.after).cast(),
        ] {
            for i in 0..32 {
                p.add(i).write(0xa5);
            }
        }
    }
    1
}
#[no_mangle]
pub extern "C" fn codec_capacity() -> u32 {
    CAP as u32
}
#[no_mangle]
pub extern "C" fn codec_input() -> *mut u8 {
    // SAFETY: address only; exclusive access is enforced at codec_call.
    unsafe { core::ptr::addr_of_mut!(WINDOWS.input).cast() }
}
#[no_mangle]
pub extern "C" fn codec_output() -> *mut u8 {
    // SAFETY: address only; exclusive access is enforced at codec_call.
    unsafe { core::ptr::addr_of_mut!(WINDOWS.output).cast() }
}
#[no_mangle]
pub extern "C" fn codec_counter(index: u32) -> u32 {
    if index >= 128 {
        return 0;
    }
    // SAFETY: an in-range scalar in this module's exclusive single-thread state.
    unsafe {
        core::ptr::addr_of!(COUNTERS)
            .cast::<u32>()
            .add(index as usize)
            .read()
    }
}
#[no_mangle]
pub extern "C" fn codec_guards() -> u32 {
    // SAFETY: addresses only, all guards are initialized static storage.
    for pointer in unsafe {
        [
            core::ptr::addr_of!(WINDOWS.before).cast::<u8>(),
            core::ptr::addr_of!(WINDOWS.between).cast(),
            core::ptr::addr_of!(WINDOWS.after).cast(),
        ]
    } {
        for i in 0..32 {
            // SAFETY: each pointer names a statically allocated32-byte guard window.
            if unsafe { pointer.add(i).read() } != 0xa5 {
                return 0;
            }
        }
    }
    1
}
#[no_mangle]
pub extern "C" fn codec_call(operation: u32, length: u32) -> i32 {
    if length as usize > CAP || operation >= 127 {
        return api::error_code(fm1_formats::CodecError::Length);
    }
    // SAFETY: the ABI serializes calls without callbacks, shared memory or
    // retained views. These two static byte windows are live and disjoint.
    let result = unsafe {
        let input = core::slice::from_raw_parts(codec_input(), length as usize);
        let output = core::slice::from_raw_parts_mut(codec_output(), CAP);
        api::run(operation, input, output)
    };
    let (index, value) = match result {
        Ok(n) => (operation, n as i32),
        Err(error) => (127, api::error_code(error)),
    };
    // SAFETY: index is a checked operation or127; no other execution can access
    // this module during the synchronous call and no reference is retained.
    unsafe {
        let counter = core::ptr::addr_of_mut!(COUNTERS)
            .cast::<u32>()
            .add(index as usize);
        counter.write(counter.read().wrapping_add(1));
    }
    value
}
