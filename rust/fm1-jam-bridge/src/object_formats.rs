// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-copy constructors for actual project/settings save consumers.
use crate::formats::{done, rejected};
use fm1_formats::{objects::*, voice::PackedVoice};
unsafe fn load<T: Copy + Default, const N: usize>(p: *const T) -> [T; N] {
    let mut a = [T::default(); N];
    for (i, v) in a.iter_mut().enumerate() {
        // SAFETY: caller promises N initialized readable values, alignment not required.
        *v = unsafe { p.add(i).read_unaligned() };
    }
    a
}
unsafe fn write(bytes: &[u8], p: *mut u8) {
    for (i, v) in bytes.iter().enumerate() {
        // SAFETY: caller promises bytes.len writable bytes, no pointer retained.
        unsafe {
            p.add(i).write(*v);
        }
    }
}
/// # Safety
/// output covers capacity writable bytes. The header is the only modified span.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_object_header(
    kind: u32,
    genre: u32,
    parameters: u32,
    saved: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if output.is_null() || genre > 19 || parameters > 65535 || saved > 65535 {
        return rejected();
    }
    let mut b = [0u8; 16];
    let n = match kind {
        0 => bank_header(
            genre as u8,
            parameters as u16,
            saved as u16,
            [1, 1, 1, 0],
            &mut b,
        ),
        1 | 2 => extension_header(
            if kind == 1 {
                ExtensionKind::Extras
            } else {
                ExtensionKind::Bass
            },
            genre as u8,
            [0; 2],
            &mut b,
        ),
        _ => return rejected(),
    };
    match n {
        Ok(n) if n <= capacity as usize => {
            // SAFETY: checked capacity and local initialized source.
            unsafe {
                write(&b[..n], output);
            }
            done(true, n)
        }
        _ => rejected(),
    }
}
/// # Safety
/// values/list/reserved cover4i16/16u8/3u8; output covers capacity writable bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_global(
    values: *const i16,
    list: *const u8,
    play: u32,
    reserved: *const u8,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if values.is_null() || list.is_null() || reserved.is_null() || output.is_null() || capacity < 32
    {
        return rejected();
    }
    // SAFETY: fixed caller ranges copied before output writes (overlap permitted).
    let (v, l, r) = unsafe { (load(values), load(list), load(reserved)) };
    let mut b = [0; 32];
    match (GlobalParts {
        values: v,
        list: Some(&l),
        play: play as u8,
        reserved: r,
    })
    .encode_compatible(&mut b)
    {
        Ok(n) => {
            // SAFETY: checked32 writable output bytes.
            unsafe {
                write(&b, output);
            }
            done(true, n)
        }
        Err(_) => rejected(),
    }
}
/// # Safety
/// input covers length initialized bytes (non-null even for an empty input).
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_crc32(input: *const u8, length: u32) -> u32 {
    if input.is_null() {
        rejected();
        return 0;
    }
    // SAFETY: caller supplies the initialized live byte span; no reference escapes.
    let bytes = unsafe { core::slice::from_raw_parts(input, length as usize) };
    let crc = fm1_formats::crc32(bytes);
    done(false, length as usize);
    crc
}
/// # Safety
/// effects covers count<=256 bytes, performance12, targets4. Output covers capacity.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_settings(
    effects: *const u8,
    count: u32,
    performance: *const u8,
    targets: *const u8,
    pattern: u32,
    usb: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if effects.is_null()
        || performance.is_null()
        || targets.is_null()
        || output.is_null()
        || count > 256
    {
        return rejected();
    }
    let mut fx = [0; 256];
    for (i, v) in fx.iter_mut().enumerate().take(count as usize) {
        // SAFETY: caller promises count readable bytes.
        *v = unsafe { effects.add(i).read() };
    }
    // SAFETY: caller promises fixed12/4 initialized values.
    let (p, t) = unsafe { (load(performance), load(targets)) };
    let mut b = [0; 288];
    match (SettingsParts {
        effects: &fx[..count as usize],
        performance: &p,
        targets: &t,
        pattern: pattern as u16,
        usb: usb as u16,
    })
    .encode(&mut b)
    {
        Ok(n) if n <= capacity as usize => {
            // SAFETY: checked writable capacity, local source.
            unsafe {
                write(&b[..n], output);
            }
            done(true, n)
        }
        _ => rejected(),
    }
}
/// # Safety
/// input covers count*128 initialized bytes, count<=16; output covers capacity.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_voices(
    input: *const u8,
    count: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if input.is_null() || output.is_null() || count > 16 || capacity < 2056 {
        return rejected();
    }
    let mut data = [[0; 128]; 16];
    for (i, voice) in data.iter_mut().enumerate().take(count as usize) {
        // SAFETY: within caller's count*128 initialized span.
        *voice = unsafe { load(input.add(i * 128)) };
    }
    let empty = PackedVoice::parse(&data[0]).unwrap();
    let mut views = [empty; 16];
    for (i, v) in views.iter_mut().enumerate().take(count as usize) {
        *v = PackedVoice::parse(&data[i]).unwrap();
    }
    // SAFETY: fixed output capacity checked; every overlapping input byte has
    // already been copied into local data. The output borrow cannot escape.
    let bytes = unsafe { core::slice::from_raw_parts_mut(output, 2056) };
    let n = VoicesParts {
        voices: &views[..count as usize],
    }
    .encode(bytes)
    .unwrap();
    done(true, n)
}
