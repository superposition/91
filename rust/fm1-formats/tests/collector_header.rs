// SPDX-License-Identifier: GPL-3.0-only
use fm1_formats::voice::{classify_legacy_with, IncomingKind};
#[test]
fn raw_header_access_never_visits_payload_or_reads_beyond_the_actual_length() {
    for length in 0..=4114 {
        let header = [0x43, 0, 9, 32, 0];
        let result = classify_legacy_with(length, |index| {
            assert!(index < 5 && index < length);
            header[index]
        });
        assert_eq!(
            result,
            if length == 4102 {
                Some(IncomingKind::Bulk)
            } else {
                None
            }
        );
    }
    for (header, length, kind) in [
        ([0x7d, 0x50, 0x56, 127, 0], 4, IncomingKind::Clear),
        ([0x7d, 0x50, 0x42, 3, 0], 12, IncomingKind::BackupChunk),
        ([0x7d, 0x50, 0x60, 0, 0], 6, IncomingKind::Jam),
    ] {
        assert_eq!(
            classify_legacy_with(length, |i| {
                assert!(i < 5 && i < length);
                header[i]
            }),
            Some(kind)
        );
    }
}
