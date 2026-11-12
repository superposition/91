// SPDX-License-Identifier: GPL-3.0-only
//! Synchronous wire ABI. C owns every buffer and operation lifetime.
use crate::formats::{done, rejected};
use fm1_formats::{device_wire, CodecError};

fn error(error: CodecError) -> i32 {
    rejected();
    -match error {
        CodecError::Length => 1,
        CodecError::Magic => 2,
        CodecError::Version => 3,
        CodecError::Index => 4,
        CodecError::Range => 5,
        CodecError::Checksum => 6,
        CodecError::Nibble => 7,
        CodecError::Capacity => 8,
        CodecError::State => 9,
    }
}

/// # Safety
/// Input covers length initialized readable bytes for this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_wire_ota_valid(input: *const u8, length: u32) -> u32 {
    if input.is_null() || length > 640 {
        rejected();
        return 0;
    }
    // SAFETY: caller supplies initialized readable input, no retained borrow.
    let data = unsafe { core::slice::from_raw_parts(input, length as usize) };
    let valid = fm1_formats::wire::OtaMessage::parse_decoded(data).is_ok();
    if valid {
        done(false, length as usize);
    } else {
        rejected();
    }
    u32::from(valid)
}

/// # Safety
/// Input and arguments cover initialized readable ranges of their stated sizes.
/// Output covers capacity writable bytes, disjoint from both input ranges.
/// Arguments may be unaligned. No other access to output is permitted during
/// this call. The complete output capacity is initialized, including its tail;
/// only the returned prefix is part of the encoded result. Nothing is retained.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_wire(
    operation: u32,
    arguments: *const u32,
    argument_count: u32,
    input: *const u8,
    length: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if argument_count > 16
        || length > 8192
        || capacity > 8192
        || (argument_count != 0 && arguments.is_null())
        || (length != 0 && input.is_null())
        || (capacity != 0 && output.is_null())
    {
        return error(CodecError::Length);
    }
    let mut args = [0u32; 16];
    for (i, arg) in args[..argument_count as usize].iter_mut().enumerate() {
        // SAFETY: bounded caller-promised argument range; no alignment assumed.
        *arg = unsafe { arguments.add(i).read_unaligned() };
    }
    // SAFETY: writable disjoint caller range; initialize bytes before borrowing.
    if capacity != 0 {
        unsafe { output.write_bytes(0, capacity as usize) };
    }
    let data = if length == 0 {
        &[]
    } else {
        // SAFETY: caller supplies initialized readable bytes for this call.
        unsafe { core::slice::from_raw_parts(input, length as usize) }
    };
    let target = if capacity == 0 {
        &mut []
    } else {
        // SAFETY: initialized above, exclusively borrowed and disjoint from data.
        unsafe { core::slice::from_raw_parts_mut(output, capacity as usize) }
    };
    match device_wire::dispatch(operation, &args[..argument_count as usize], data, target) {
        Ok(n) => done(
            matches!(operation, 71 | 72 | 74 | 76 | 79 | 83 | 85 | 90),
            n,
        ),
        Err(e) => error(e),
    }
}

/// # Safety
/// Bytes covers length initialized writable bytes exclusively for this call.
/// This separate entry permits the firmware's in-place header cipher.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_wire_enc(bytes: *mut u8, length: u32, key: u32) -> i32 {
    if length > 8192 || (length != 0 && bytes.is_null()) {
        return error(CodecError::Length);
    }
    let target = if length == 0 {
        &mut []
    } else {
        // SAFETY: caller promises initialized exclusive storage, no retained view.
        unsafe { core::slice::from_raw_parts_mut(bytes, length as usize) }
    };
    fm1_formats::package::enc(target, key as u16);
    done(true, length as usize)
}

/// # Safety
/// Input covers length readable bytes. Output covers capacity writable bytes,
/// disjoint from input. The decoder writes at most min(capacity, decoded size)
/// bytes but returns the total size even on truncation, matching legacy OTA.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_wire_unpack7(
    input: *const u8,
    length: u32,
    output: *mut u8,
    capacity: u32,
) -> u32 {
    if length > 8192
        || capacity > 8192
        || (length != 0 && input.is_null())
        || (capacity != 0 && output.is_null())
    {
        rejected();
        return 0;
    }
    let total = length as usize * 7 / 8;
    let written = total.min(capacity as usize);
    // SAFETY: initialize only the prefix that the legacy decoder would write.
    if written != 0 {
        unsafe { output.write_bytes(0, written) };
    }
    let data = if length == 0 {
        &[]
    } else {
        // SAFETY: caller promises readable initialized input.
        unsafe { core::slice::from_raw_parts(input, length as usize) }
    };
    let target = if written == 0 {
        &mut []
    } else {
        // SAFETY: initialized exclusive output prefix, disjoint from data.
        unsafe { core::slice::from_raw_parts_mut(output, written) }
    };
    match device_wire::unpack7_prefix(data, target) {
        Ok(n) => {
            done(false, written);
            n as u32
        }
        Err(_) => {
            rejected();
            0
        }
    }
}
