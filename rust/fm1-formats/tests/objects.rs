// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::{objects::*, storage, voice::PackedVoice, CodecError};
#[test]
fn constructors_preserve_reserved_bytes_and_check_boundaries_before_writing() {
    let mut output = [0xa5; 3842];
    let settings = SettingsParts {
        effects: &[1, 2, 3],
        performance: &[17; 12],
        targets: &[19; 4],
        pattern: 0xabcd,
        usb: 0xdead,
    };
    let mut base = [0x91; 40];
    base[..4].copy_from_slice(b"SET1");
    assert_eq!(settings.encode_preserving(Some(&base), &mut output), Ok(40));
    assert_eq!(output[27], 0x91);
    assert_eq!(&output[32..40], &[0x91; 8]);
    assert_eq!(&output[40..], &[0xa5; 3802]);
    let entries = [MidiEntry::parse([0xb1, 127, 175]).unwrap()];
    let mut base = [0x93; 152];
    base[..4].copy_from_slice(b"MAP2");
    MidiMapParts {
        version: None,
        entries: &entries,
        reserved: [1, 2, 3],
    }
    .encode_preserving(Some(&base), &mut output)
    .unwrap();
    assert_eq!(&output[11..152], &base[11..]);
    assert_eq!(
        storage::MidiMap::parse(&output[..152]).unwrap().entry(0),
        Ok([0xb1, 127, 175])
    );
    for entry in [
        [0x80, 1, 2],
        [0x70, 1, 2],
        [0x10, 128, 2],
        [0x10, 1, 14],
        [0x90, 1, 176],
    ] {
        assert_eq!(MidiEntry::parse(entry), Err(CodecError::Range));
    }
    let packed = [7; 128];
    let voice = PackedVoice::parse(&packed).unwrap();
    let voices = [voice; 16];
    let original = [0x98; 2056];
    VoicesParts {
        voices: &voices[..1],
    }
    .encode_preserving(Some(&original), &mut output)
    .unwrap();
    assert_eq!(&output[136..2056], &original[136..]);
    assert!(VoicesParts {
        voices: &[voice; 17]
    }
    .encode(&mut output)
    .is_err());
    let invalid = GlobalParts {
        values: [-32768, 32767, -1, 0],
        list: Some(&[255; 16]),
        play: 255,
        reserved: [1, 128, 255],
    };
    output.fill(0xa5);
    assert_eq!(invalid.encode(&mut output), Err(CodecError::Index));
    assert!(output.iter().all(|v| *v == 0xa5));
    invalid.encode_compatible(&mut output).unwrap();
    assert_eq!(&output[12..28], &[255; 16]);
    assert_eq!(&output[29..32], &[1, 128, 255]);
    let records = [0; 3760];
    let parts = BankParts {
        genre: 18,
        parameters: 65535,
        saved: 65535,
        answer_encoding: 253,
        voice_numbering: 254,
        progression_flag: 255,
        reserved: 252,
        records: &records,
        progressions: Some(&[0x91; 60]),
        trailing: &[1, 2, 3, 4],
    };
    assert_eq!(parts.encode(&mut output), Ok(3840));
    let bank = storage::Bank::parse(&output[..3840], 18).unwrap();
    let mut copied = [0; 3840];
    bank.encode(&mut copied).unwrap();
    assert_eq!(copied, &output[..3840]);
    assert_eq!(bank.record(10).err(), Some(CodecError::Index));
    let mut guarded = [0xa5; 15];
    assert_eq!(
        bank_header(19, 176, 0, [1, 1, 1, 0], &mut guarded),
        Err(CodecError::Index)
    );
    assert_eq!(
        bank_header(0, 176, 0, [1, 1, 1, 0], &mut guarded),
        Err(CodecError::Capacity)
    );
    assert_eq!(guarded, [0xa5; 15]);
}
#[test]
fn extension_and_piece_mutators_reject_bad_indices_and_keep_other_records() {
    let mut a = [0; 3048];
    extension_update(
        &[],
        2,
        3,
        19,
        1,
        Some(&[1, 2, 3]),
        Some(&[7; 7]),
        None,
        &mut a,
    )
    .unwrap();
    let old = a;
    extension_update(&old, 18, 9, 19, 1, None, None, None, &mut a).unwrap();
    let mut kinds = [0; 3];
    assert_eq!(extension_get(&a, 2, 3, 0, &mut kinds), Ok(3));
    assert_eq!(kinds, [1, 2, 3]);
    assert_eq!(&a[8 + 23 * 16..8 + 24 * 16], &old[8 + 23 * 16..8 + 24 * 16]);
    let mut output = [0xa5; 3840];
    assert!(extension_update(&a, 19, 0, 19, 1, None, None, None, &mut output).is_err());
    assert!(output.iter().all(|v| *v == 0xa5));
    assert!(import_piece(&[], 0, 10, 176, 176, &[0; 376], &[0; 6], &mut output).is_err());
    assert!(output.iter().all(|v| *v == 0xa5));
}
