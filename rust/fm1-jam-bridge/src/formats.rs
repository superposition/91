// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-copy ABI for legacy C codec consumers. No pointers are retained.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_formats::{storage, voice, wire, CodecError};
static DECODE: AtomicU32 = AtomicU32::new(0);
static ENCODE: AtomicU32 = AtomicU32::new(0);
static REJECT: AtomicU32 = AtomicU32::new(0);
static COPIED: AtomicU32 = AtomicU32::new(0);
pub fn stat(index: u32) -> u32 {
    match index {
        36 => DECODE.load(Ordering::Relaxed),
        37 => ENCODE.load(Ordering::Relaxed),
        38 => REJECT.load(Ordering::Relaxed),
        39 => COPIED.load(Ordering::Relaxed),
        _ => 0,
    }
}
pub(crate) fn rejected() -> i32 {
    REJECT.fetch_add(1, Ordering::Relaxed);
    -1
}
pub(crate) fn done(encode: bool, n: usize) -> i32 {
    if encode {
        ENCODE.fetch_add(1, Ordering::Relaxed);
    } else {
        DECODE.fetch_add(1, Ordering::Relaxed);
    }
    COPIED.fetch_add(n as u32, Ordering::Relaxed);
    n as i32
}
/// # Safety
/// `input` covers N initialized elements. Reads permit unaligned C storage.
unsafe fn load<T: Copy + Default, const N: usize>(input: *const T) -> [T; N] {
    let mut out = [T::default(); N];
    for (i, value) in out.iter_mut().enumerate() {
        // SAFETY: each read is within the caller's promised fixed initialized range.
        *value = unsafe { input.add(i).read_unaligned() };
    }
    out
}
/// # Safety
/// Output covers input.len() writable elements, disjoint from this local input.
unsafe fn store<T: Copy>(input: &[T], output: *mut T) {
    for (i, value) in input.iter().enumerate() {
        // SAFETY: each write stays in the caller's promised fixed output range.
        unsafe { output.add(i).write_unaligned(*value) };
    }
}
/// # Safety
/// Pointers cover the supplied byte lengths. Voice output permits overlap with
/// input: all source bytes are copied before writes. Decode includes C's NUL.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_voice(
    kind: u32,
    input: *const u8,
    length: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if input.is_null() || output.is_null() {
        return rejected();
    }
    match kind {
        0 if length == 128 && capacity >= 156 => {
            // SAFETY: checked fixed lengths; caller supplies live storage.
            let b = unsafe { load::<u8, 128>(input) };
            let v = voice::unpack(&b);
            // SAFETY: bounded local source and156 writable output bytes.
            unsafe {
                store(&v, output);
                output.add(155).write(0);
            }
            done(false, 156)
        }
        1 if length == 155 && capacity >= 128 => {
            // SAFETY: checked fixed lengths; source copied before overlapping output.
            let v = unsafe { load::<u8, 155>(input) };
            let b = voice::pack(&v);
            // SAFETY: bounded local source and128 writable output bytes.
            unsafe {
                store(&b, output);
            }
            done(true, 128)
        }
        _ => rejected(),
    }
}
/// # Safety
/// Parameters/patterns cover176i16/36u16; mod/acid/pluck cover96/18/12 bytes.
/// Output covers capacity writable bytes. Inputs may be unaligned.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_record_pack(
    parameters: *const i16,
    pool: u16,
    patterns: *const u16,
    modifiers: *const u8,
    acid: *const u8,
    pluck: *const u8,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if [
        parameters.cast::<u8>(),
        patterns.cast(),
        modifiers,
        acid,
        pluck,
    ]
    .iter()
    .any(|p| p.is_null())
        || output.is_null()
        || capacity < 376
    {
        return rejected();
    }
    // SAFETY: caller promises each fixed input range; all are copied locally.
    let values = unsafe {
        storage::RecordValues {
            parameters: load(parameters),
            pool,
            patterns: load(patterns),
            modifiers: load(modifiers),
            acid: load(acid),
            pluck: load(pluck),
        }
    };
    let mut record = [0; 376];
    values.encode_legacy(&mut record).unwrap();
    // SAFETY: output covers376 bytes, disjoint from this local record.
    unsafe {
        store(&record, output);
    }
    done(true, 376)
}
/// # Safety
/// Input covers376 bytes; all outputs cover the fixed ranges stated in pack.
/// Parameters are initialized recipe values. Output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_record_unpack(
    input: *const u8,
    length: u32,
    count: u32,
    parameters: *mut i16,
    pool: *mut u16,
    patterns: *mut u16,
    modifiers: *mut u8,
    acid: *mut u8,
    pluck: *mut u8,
) -> i32 {
    if input.is_null()
        || length != 376
        || count > 65535
        || [
            parameters.cast::<u8>(),
            pool.cast(),
            patterns.cast(),
            modifiers,
            acid,
            pluck,
        ]
        .iter()
        .any(|p| p.is_null())
    {
        return rejected();
    }
    // SAFETY: checked and caller-promised fixed source ranges, read unaligned.
    let (record, recipe) = unsafe { (load::<u8, 376>(input), load::<i16, 176>(parameters)) };
    let decoded = storage::Record::parse(&record)
        .unwrap()
        .decode_legacy(count as u16, &recipe);
    let v = decoded.values;
    // SAFETY: bounded fixed local sources copied into the disjoint caller ranges.
    unsafe {
        store(&v.parameters, parameters);
        pool.write_unaligned(v.pool);
        store(&v.patterns, patterns);
        store(&v.modifiers, modifiers);
        store(&v.acid, acid);
        store(&v.pluck, pluck);
    }
    done(false, 552);
    if !decoded.valid {
        REJECT.fetch_add(1, Ordering::Relaxed);
    }
    i32::from(decoded.valid)
}
/// # Safety
/// Input covers length bytes when nonzero; no access occurs for empty input.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_restore(
    object: u32,
    input: *const u8,
    length: u32,
    effects: u32,
    settings_len: u32,
) -> i32 {
    if (input.is_null() && length != 0) || length > 3840 || effects > 65535 {
        return rejected();
    }
    // SAFETY: bounded borrowed byte storage promised by the caller, never retained.
    let bytes = if length == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(input, length as usize) }
    };
    let valid = storage::restore_valid(
        object,
        bytes,
        storage::RestoreLimits {
            effects: effects as u16,
            settings_len: settings_len as usize,
        },
    );
    DECODE.fetch_add(1, Ordering::Relaxed);
    if !valid {
        REJECT.fetch_add(1, Ordering::Relaxed);
    }
    i32::from(valid)
}
/// # Safety
/// Input covers length initialized bytes; data covers256 writable bytes and
/// fields four disjoint writable u16s (object,index,total,count).
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_chunk(
    input: *const u8,
    length: u32,
    data: *mut u8,
    fields: *mut u16,
) -> i32 {
    if input.is_null() || data.is_null() || fields.is_null() || length > 524 {
        return 2;
    }
    // SAFETY: bounded initialized caller input, borrowed only for this call.
    let bytes = unsafe { core::slice::from_raw_parts(input, length as usize) };
    match wire::BackupData::parse_chunk_legacy(bytes) {
        Ok(chunk) => {
            let mut decoded = [0; 256];
            let n = chunk.decode(&mut decoded).unwrap();
            let meta = [chunk.object as u16, chunk.index, chunk.total, n as u16];
            // SAFETY: disjoint local data copied into caller-provided bounded outputs.
            unsafe {
                store(&decoded[..n], data);
                store(&meta, fields);
            }
            done(false, n);
            0
        }
        Err(error) => {
            REJECT.fetch_add(1, Ordering::Relaxed);
            if error == CodecError::Checksum {
                1
            } else {
                2
            }
        }
    }
}
/// # Safety
/// Output covers capacity bytes; nonempty input covers count initialized bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_backup_frame(
    output: *mut u8,
    capacity: u32,
    object: u32,
    index: u32,
    total: u32,
    input: *const u8,
    count: u32,
) -> u32 {
    if output.is_null()
        || count > 64
        || (input.is_null() && count != 0)
        || capacity < 13 + count * 2
    {
        rejected();
        return 0;
    }
    let mut data = [0; 64];
    for (i, value) in data.iter_mut().enumerate().take(count as usize) {
        // SAFETY: count checked<=64 and caller supplies readable source bytes.
        *value = unsafe { input.add(i).read() };
    }
    let mut frame = [0; 144];
    let n = wire::backup_frame(
        (object & 127) as u8,
        (index & 16383) as u16,
        (total & 16383) as u16,
        &data[..count as usize],
        &mut frame,
    )
    .unwrap();
    // SAFETY: local frame fits checked caller output capacity.
    unsafe {
        store(&frame[..n], output);
    }
    done(true, n);
    n as u32
}
/// # Safety
/// Output covers capacity; values covers count initialized i32s, count<=24.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_dump(
    output: *mut u8,
    capacity: u32,
    first: u32,
    values: *const i32,
    count: u32,
) -> u32 {
    if output.is_null()
        || count > 24
        || (values.is_null() && count != 0)
        || capacity < 8 + count * 2
    {
        rejected();
        return 0;
    }
    let mut input = [0; 24];
    for (i, value) in input.iter_mut().enumerate().take(count as usize) {
        // SAFETY: bounded caller source, including permitted unaligned words.
        *value = unsafe { values.add(i).read_unaligned() };
    }
    let mut frame = [0; 144];
    let n =
        wire::editor_dump((first & 16383) as u16, &input[..count as usize], &mut frame).unwrap();
    // SAFETY: bounded local frame copied to checked caller output capacity.
    unsafe {
        store(&frame[..n], output);
    }
    done(true, n);
    n as u32
}
/// # Safety
/// Output covers capacity. Non-null voice covers128 initialized bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_editor_voice(
    output: *mut u8,
    capacity: u32,
    slot: u32,
    loaded: u32,
    voice: *const u8,
) -> u32 {
    let n = if voice.is_null() { 7 } else { 136 };
    if output.is_null() || capacity < n {
        rejected();
        return 0;
    }
    // SAFETY: non-null source has the caller-promised128 bytes; copied before write.
    let input = if voice.is_null() {
        None
    } else {
        Some(unsafe { load::<u8, 128>(voice) })
    };
    let mut frame = [0; 144];
    let n = wire::editor_voice(
        (slot & 127) as u8,
        (loaded & 127) as u8,
        input.as_ref(),
        &mut frame,
    )
    .unwrap();
    // SAFETY: local frame fits checked output range.
    unsafe {
        store(&frame[..n], output);
    }
    done(true, n);
    n as u32
}

/// # Safety
/// Input covers length bytes. Outputs cover176i16, one u16 and36u16,
/// are initialized/writable, disjoint and may be unaligned.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_legacy(
    input: *const u8,
    length: u32,
    count: u32,
    flags: u32,
    parameters: *mut i16,
    pool: *mut u16,
    patterns: *mut u16,
) -> i32 {
    if input.is_null()
        || parameters.is_null()
        || pool.is_null()
        || patterns.is_null()
        || !(1..=176).contains(&count)
        || length != count * 2 + 74
        || flags > 3
    {
        return rejected();
    }
    // SAFETY: checked bounded source range, borrowed only until decoded locally.
    let bytes = unsafe { core::slice::from_raw_parts(input, length as usize) };
    let values = storage::LegacyRecord::parse(bytes, count as u16)
        .unwrap()
        .decode(flags & 1 != 0, flags & 2 != 0);
    // SAFETY: fixed local arrays copied to caller-promised disjoint writable ranges.
    unsafe {
        store(&values.parameters, parameters);
        pool.write_unaligned(values.pool);
        store(&values.patterns, patterns);
    }
    done(false, 426);
    if !values.valid {
        REJECT.fetch_add(1, Ordering::Relaxed);
    }
    i32::from(values.valid)
}

/// # Safety
/// Nonempty input covers length initialized bytes, at most the original4112B
/// USB collector buffer. Collection/IRQ/publication stay with the C caller.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_classify(input: *const u8, length: u32) -> u32 {
    if length > 4112 || (length != 0 && input.is_null()) {
        rejected();
        return 0;
    }
    // SAFETY: bounded initialized caller storage, never retained or mutated.
    let bytes = if length == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(input, length as usize) }
    };
    DECODE.fetch_add(1, Ordering::Relaxed);
    voice::classify_legacy(bytes).map_or(0, |k| k as u32)
}

/// # Safety
/// Nonempty input covers length bytes; shared original USB bound is4112 bytes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_sum7(input: *const u8, length: u32) -> u32 {
    if length > 4112 || (length != 0 && input.is_null()) {
        rejected();
        return 0;
    }
    // SAFETY: bounded live source bytes, borrowed only during the checksum.
    let bytes = if length == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(input, length as usize) }
    };
    wire::sum7(bytes) as u32
}
/// # Safety
/// Output covers capacity writable bytes; kind0=backup ack, kind1=about reply.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_formats_control(
    output: *mut u8,
    capacity: u32,
    kind: u32,
    a: u32,
    b: u32,
    c: u32,
) -> u32 {
    let n = match kind {
        0 => 8,
        1 => 10,
        _ => {
            rejected();
            return 0;
        }
    };
    if output.is_null() || capacity < n {
        rejected();
        return 0;
    }
    let mut frame = [0; 10];
    let n = if kind == 0 {
        wire::backup_ack(
            (a & 127) as u8,
            (b & 127) as u8,
            (c & 127) as u8,
            &mut frame,
        )
    } else {
        wire::about(
            3,
            (a & 16383) as u16,
            (b & 127) as u8,
            (c & 127) as u8,
            &mut frame,
        )
    }
    .unwrap();
    // SAFETY: bounded local frame fits the checked output allocation.
    unsafe {
        store(&frame[..n], output);
    }
    done(true, n);
    n as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_abi_ranges_preserve_sentinels_and_voice_copy_supports_overlap() {
        let input = [255u8; 128];
        let mut output = [0xa5; 160];
        let mut chunk_output = [0xa5; 256];
        // SAFETY: all non-null pointers identify complete initialized arrays;
        // rejected null/capacity/kind cases must return before touching memory.
        unsafe {
            assert_eq!(
                fm1_rust_formats_voice(0, input.as_ptr(), 128, output.as_mut_ptr(), 155),
                -1
            );
            assert_eq!(
                fm1_rust_formats_voice(0, core::ptr::null(), 128, output.as_mut_ptr(), 160),
                -1
            );
            assert_eq!(
                fm1_rust_formats_voice(2, input.as_ptr(), 128, output.as_mut_ptr(), 160),
                -1
            );
            assert_eq!(
                fm1_rust_formats_dump(output.as_mut_ptr(), 160, 0, core::ptr::null(), 1),
                0
            );
            assert_eq!(
                fm1_rust_formats_chunk(
                    input.as_ptr(),
                    128,
                    chunk_output.as_mut_ptr(),
                    core::ptr::null_mut()
                ),
                2
            );
        }
        assert_eq!(output, [0xa5; 160]);
        assert_eq!(chunk_output, [0xa5; 256]);
        output[..128].copy_from_slice(&input);
        // SAFETY: the160-byte allocation covers both source128 and destination156
        // ranges, including their intentional overlap; bridge copies before writes.
        unsafe {
            assert_eq!(
                fm1_rust_formats_voice(0, output.as_ptr(), 128, output.as_mut_ptr().add(1), 156),
                156
            );
        }
        assert_eq!(&output[1..156], &voice::unpack(&input));
        assert_eq!(output[156], 0);
        assert_eq!(&output[157..], &[0xa5; 3]);
    }
    #[test]
    fn unaligned_record_parameters_are_copied_without_serializing_runtime_layout() {
        let mut parameters = [0u8; 354];
        let mut recipe = [0i16; 176];
        for (i, info) in fm1_types::PARAMETERS.iter().enumerate() {
            recipe[i] = info.default;
            parameters[1 + i * 2..3 + i * 2].copy_from_slice(&info.default.to_ne_bytes());
        }
        let patterns = [0xabcd; 36];
        let modifiers = [0x87; 96];
        let acid = [0x13; 18];
        let pluck = [0xe1; 12];
        let mut record = [0xa5; 378];
        let mut pool = 0u16;
        let mut output_patterns = [0u16; 36];
        let mut output_modifiers = [0; 96];
        let mut output_acid = [0; 18];
        let mut output_pluck = [0; 12];
        // SAFETY: parameters' intentionally unaligned176-word range remains inside
        // its354-byte allocation; every other source/output has its exact live size.
        unsafe {
            assert_eq!(
                fm1_rust_formats_record_pack(
                    parameters.as_ptr().add(1).cast(),
                    0xab5,
                    patterns.as_ptr(),
                    modifiers.as_ptr(),
                    acid.as_ptr(),
                    pluck.as_ptr(),
                    record.as_mut_ptr().add(1),
                    376
                ),
                376
            );
            assert_eq!(
                fm1_rust_formats_record_unpack(
                    record.as_ptr().add(1),
                    376,
                    176,
                    parameters.as_mut_ptr().add(1).cast(),
                    &mut pool,
                    output_patterns.as_mut_ptr(),
                    output_modifiers.as_mut_ptr(),
                    output_acid.as_mut_ptr(),
                    output_pluck.as_mut_ptr()
                ),
                1
            );
        }
        assert_eq!(record[0], 0xa5);
        assert_eq!(record[377], 0xa5);
        assert_eq!(parameters[0], 0);
        assert_eq!(parameters[353], 0);
        assert_eq!(pool, 0xab5);
        assert_eq!(output_patterns, patterns);
        assert_eq!(output_modifiers, modifiers);
        assert_eq!(output_acid, acid);
        assert_eq!(output_pluck, pluck);
        for (i, value) in recipe.iter().enumerate() {
            assert_eq!(
                i16::from_ne_bytes([parameters[1 + i * 2], parameters[2 + i * 2]]),
                *value
            );
        }
    }
}
