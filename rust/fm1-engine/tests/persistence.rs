// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::persistence::*;
struct Flash {
    bytes: Vec<u8>,
    calls: Vec<(u8, u32, usize)>,
    fail: Option<usize>,
    fail_code: i32,
}
impl Flash {
    fn new() -> Self {
        Self {
            bytes: vec![255; 0x100000],
            calls: vec![],
            fail: None,
            fail_code: -9,
        }
    }
}
impl Nor for Flash {
    fn read(&mut self, off: u32, out: &mut [u8]) -> i32 {
        self.calls.push((0, off, out.len()));
        if self.fail == Some(self.calls.len()) {
            return self.fail_code;
        }
        let Some(b) = self.bytes.get(off as usize..off as usize + out.len()) else {
            return -8;
        };
        out.copy_from_slice(b);
        0
    }
    fn erase(&mut self, off: u32) -> i32 {
        self.calls.push((1, off, 4096));
        if self.fail == Some(self.calls.len()) {
            return self.fail_code;
        }
        self.bytes[off as usize..off as usize + 4096].fill(255);
        0
    }
    fn program(&mut self, off: u32, input: &[u8]) -> i32 {
        self.calls.push((2, off, input.len()));
        if self.fail == Some(self.calls.len()) {
            return self.fail_code;
        }
        assert!((off & 255) as usize + input.len() <= 256);
        for (a, b) in self.bytes[off as usize..].iter_mut().zip(input) {
            *a &= *b;
        }
        0
    }
}
#[test]
fn all_commit_boundaries_keep_a_complete_object() {
    for length in [0, 1, 255, 256, 257, 3836, 3840] {
        for cut in 1..25 {
            let (mut scratch, mut got) = ([0; MAX], [0; MAX]);
            let mut store = Store::new(Flash::new(), &mut scratch);
            let prior = vec![91; length];
            let next = vec![37; length];
            assert_eq!(store.save(0, &prior), 0);
            store.io.calls.clear();
            store.io.fail = Some(cut);
            let rc = store.save(0, &next);
            let writes: Vec<_> = store
                .io
                .calls
                .iter()
                .filter(|(op, _, _)| *op == 2)
                .collect();
            if let Some(last) = writes.last() {
                if last.1 == sector(0, 1) {
                    assert_eq!(last.2, 32);
                }
            }
            store.io.fail = None;
            assert_eq!(store.load(0, &mut got), length as i32);
            assert!(
                got[..length] == prior || got[..length] == next,
                "cut{cut} rc{rc}"
            );
            if rc == 0 {
                assert_eq!(got[..length], next);
            }
        }
    }
}
#[test]
fn malformed_headers_are_bounded_and_fail_without_writes() {
    let mut x = 0x9e3779b9u32;
    for n in 0..32768 {
        let mut b = [0u8; 32];
        for v in &mut b {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *v = x as u8;
        }
        if n % 2 == 0 {
            b[..4].copy_from_slice(&MAGIC.to_le_bytes());
            b[4..6].copy_from_slice(&0u16.to_le_bytes());
            let c = fm1_formats::crc32(&b[..28]);
            b[28..].copy_from_slice(&c.to_le_bytes());
        }
        if let Some(h) = Header::decode(0, &b) {
            assert!(h.length <= MAX as u32);
            assert_eq!(h.encode(), b);
        }
    }
    let mut scratch = [0; MAX];
    let mut store = Store::new(Flash::new(), &mut scratch);
    assert_eq!(store.save(0, &[0; MAX + 1]), -1);
    assert!(store.io.calls.is_empty());
    assert_eq!(store.load(0, &mut []), -1);
}
#[test]
fn object_pairs_are_disjoint_and_fixed() {
    let expected = [
        0xfc000, 0xdc000, 0xde000, 0x97000, 0x99000, 0x9b000, 0x9d000, 0x9f000, 0xa1000, 0xa3000,
        0xa5000, 0xa7000, 0xa9000, 0xab000, 0xad000, 0xaf000, 0xb1000, 0xb3000, 0xb5000, 0xb7000,
        0xb9000, 0xbb000, 0xbd000, 0xbf000, 0xc1000, 0xc3000, 0xc5000, 0xc7000, 0xc9000, 0xcb000,
        0xcd000,
    ];
    for (o, base) in expected.iter().enumerate() {
        assert_eq!(sector(o as u32, 0), *base);
        assert_eq!(sector(o as u32, 1), base + 4096);
    }
}

#[test]
fn busy_is_not_a_missing_or_damaged_record() {
    for cut in 1..=22 {
        let mut scratch = [0; MAX];
        let mut store = Store::new(Flash::new(), &mut scratch);
        let old = [91; MAX];
        assert_eq!(store.save(0, &old), 0);
        store.io.calls.clear();
        store.io.fail = Some(cut);
        store.io.fail_code = BUSY;
        let prior = store.io.bytes.clone();
        store.counts = Counts::default();
        assert_eq!(store.save(0, &[37; MAX]), BUSY, "boundary {cut}");
        assert_eq!(store.counts.missing, 0);
        assert_eq!(store.counts.verify_failed, 0);
        if cut <= 3 {
            assert_eq!(store.io.bytes, prior);
        }
        store.io.fail = None;
        let mut output = [0; MAX];
        assert_eq!(store.load(0, &mut output), MAX as i32);
        assert!(output == old || output == [37; MAX]);
    }
    for cut in 1..=3 {
        let mut scratch = [0; MAX];
        let mut store = Store::new(Flash::new(), &mut scratch);
        assert_eq!(store.save(0, &[91; MAX]), 0);
        store.io.calls.clear();
        store.io.fail = Some(cut);
        store.io.fail_code = BUSY;
        let mut output = [73; MAX];
        let missing = store.counts.missing;
        assert_eq!(store.load(0, &mut output), BUSY);
        assert_eq!(output, [73; MAX]);
        assert_eq!(store.counts.missing, missing);
    }
}
