// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::usb_tx::packet;
#[test]
fn every_message_length_and_offset_visits_only_the_bounded_source_prefix() {
    for total in 0..=640 {
        for index in 0..=total + 1 {
            let mut reads = Vec::new();
            let result = packet(total, index, |at| {
                assert!(at < total);
                reads.push(at);
                at as u8
            });
            if index >= total {
                assert_eq!(result, None);
                assert!(reads.is_empty());
            } else {
                let (word, n) = result.unwrap();
                assert!(n <= 3 && index + n <= total);
                assert_eq!(reads, (index..index + n).collect::<Vec<_>>());
                for j in 0..n {
                    assert_eq!(((word >> (8 + 8 * j)) & 255) as u8, (index + j) as u8);
                }
                assert_eq!(word & 15, if index + n < total { 4 } else { 4 + n as u32 });
            }
        }
    }
}
