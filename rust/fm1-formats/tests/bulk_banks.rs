// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::voice_file::{BulkBanks, FileVoices};
#[test]
fn header_only_borrowed_profile_keeps_corrupt_trailers_and_high_bytes() {
    let mut data = vec![0x80; 4104];
    data[..6].copy_from_slice(&[240, 67, 255, 9, 32, 0]);
    let mut banks = BulkBanks::new(&data);
    let bank = banks.next().unwrap();
    assert_eq!(bank.as_ptr(), data[6..].as_ptr());
    assert_eq!(bank.len(), 4096);
    assert!(banks.next().is_none());
    assert!(banks.next().is_none());
    assert!(FileVoices::new(&data).next().is_none());
    assert!(fm1_formats::voice::DxMessage::parse(&data).is_err());
}
#[test]
fn profile_skips_accepted_payload_and_rejects_truncation_raw_and_single() {
    let mut bank = vec![0; 4104];
    bank[..6].copy_from_slice(&[240, 67, 0, 9, 32, 0]);
    bank[100..106].copy_from_slice(&[240, 67, 0, 9, 32, 0]);
    let mut data = vec![240, 67, 0, 0, 0, 0];
    data.extend_from_slice(&bank);
    data.extend_from_slice(&bank);
    assert_eq!(BulkBanks::new(&data).count(), 2);
    for end in 0..4104 {
        assert!(BulkBanks::new(&bank[..end]).next().is_none());
    }
    assert!(BulkBanks::new(&[0; 4096]).next().is_none());
    let mut single = vec![0; 163];
    single[..6].copy_from_slice(&[240, 67, 0, 0, 1, 27]);
    single[162] = 247;
    assert!(FileVoices::new(&single).next().is_some());
    assert!(BulkBanks::new(&single).next().is_none());
}
