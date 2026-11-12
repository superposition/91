// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::{package, package_write::*, CodecError};
#[test]
fn typed_ufw_fields_preserve_unknown_bytes_and_bounded_writes() {
    let header = HeaderParts {
        total: 0x12345678,
        entries: 17,
        flags: 0xabcd,
        version: 0x2345,
        reserved: 0xcdef,
        chip: b"ABCDEF",
        directory_checksum: 0x98ba,
    };
    let base = [0x93; 64];
    let bytes = header.encode_preserving(&base);
    assert_eq!(&bytes[22..], &base[22..]);
    assert_eq!(
        u16::from_le_bytes(bytes[..2].try_into().unwrap()),
        package::crc16(&bytes[2..], 0)
    );
    let extra = [0x91; 44];
    let name = *b"unknown\0tail\xff\x80\x07\x09";
    let entry = EntryParts {
        kind: 255,
        index: 65535,
        checksum: 0xabab,
        reserved: 0xffff,
        offset: u32::MAX,
        length: u32::MAX,
        padded_length: u32::MAX,
        extra: &extra,
        name: &name,
    }
    .encode();
    assert_eq!(&entry[20..64], &extra);
    assert_eq!(&entry[64..], &name);
    let mut output = [0xa5; 1100];
    assert_eq!(
        UfwParts {
            flash: &[0; 80],
            ota: &[],
            product: b"FM-1_7000300"
        }
        .encode(&mut output),
        Err(CodecError::Capacity)
    );
    assert_eq!(output, [0xa5; 1100]);
    assert_eq!(
        UfwParts {
            flash: &[],
            ota: &[],
            product: &[255; 21]
        }
        .encode(&mut output),
        Err(CodecError::Range)
    );
    assert_eq!(output, [0xa5; 1100]);
    let n = UfwParts {
        flash: &[],
        ota: &[],
        product: b"FM-1_7000300",
    }
    .encode(&mut output)
    .unwrap();
    assert_eq!(n, 1044);
    assert_eq!(&output[n..], &[0xa5; 56]);
    let package = package::Fwsc::parse(&output[..n]).unwrap();
    let directory = package.directory().unwrap();
    assert_eq!(directory.len(), 2);
    directory.validate_data(0).unwrap();
    directory.validate_data(1).unwrap();
}
#[test]
fn rescue_profile_keeps_the_original_truncated_flash_error() {
    let mut raw = vec![0; 2048];
    let n = UfwParts {
        flash: &[1, 2, 3],
        ota: &[],
        product: b"FM-1_7000300",
    }
    .encode(&mut raw)
    .unwrap();
    raw.truncate(n);
    let package = package::Fwsc::parse(&raw).unwrap();
    let mut logical = vec![0; package.logical_len()];
    package.copy_logical(0, &mut logical).unwrap();
    let mut entry: [u8; 80] = logical[64..144].try_into().unwrap();
    package::enc(&mut entry, 65535);
    entry[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    package::enc(&mut entry, 65535);
    logical[64..144].copy_from_slice(&entry);
    package::wrap_logical(&logical, b"FM-1_7000300", &mut raw).unwrap();
    let mut output = [0; 2048];
    assert_eq!(
        fm1_formats::tool_wire::rescue_flash_compatible(&raw, &mut output),
        Err(CodecError::Checksum)
    );
}
