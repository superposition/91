// SPDX-License-Identifier: GPL-3.0-only
use std::io::{self, BufRead, BufReader, Read};
pub const BUTTONS: [&str; 14] = [
    "FX", "SEL", "ENV", "LFO", "EDIT", "GLO", "HOME", "SAVE", "ARP", "SEQ", "PLAY", "REC", "OCT-",
    "OCT+",
];
pub const ENCODERS: [&str; 7] = ["SELECT", "ALGO", "PRESET", "K1", "K2", "K3", "K4"];
pub const WHITE: [u8; 16] = [0, 2, 4, 6, 7, 9, 11, 12, 14, 16, 18, 19, 21, 23, 24, 26];
pub const BLACK: [u8; 11] = [1, 3, 5, 8, 10, 13, 15, 17, 20, 22, 25];
/// C fgets/sscanf widths, including long lines split into511-byte records.
pub fn records(input: impl Read) -> Records<impl BufRead> {
    Records(BufReader::new(input))
}
pub struct Records<R>(R);
impl<R: BufRead> Iterator for Records<R> {
    type Item = io::Result<Vec<u8>>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut out = Vec::with_capacity(511);
        while out.len() < 511 {
            let bytes = match self.0.fill_buf() {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            if bytes.is_empty() {
                break;
            }
            let size = bytes.len().min(511 - out.len());
            let n = bytes[..size]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(size, |i| i + 1);
            let ended = bytes[n - 1] == b'\n';
            out.extend_from_slice(&bytes[..n]);
            self.0.consume(n);
            if ended {
                break;
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(Ok(out))
        }
    }
}
pub fn fields(line: &[u8]) -> [String; 4] {
    let mut p = 0;
    core::array::from_fn(|i| {
        while p < line.len() && line[p].is_ascii_whitespace() {
            p += 1;
        }
        let start = p;
        while p < line.len() && !line[p].is_ascii_whitespace() && p - start < [31, 255, 63, 63][i] {
            p += 1;
        }
        String::from_utf8_lossy(&line[start..p]).into_owned()
    })
}
/// C strtol/atoi prefix parsing for the bounded scenario arguments.
pub fn number(s: &str, mut radix: u32) -> i32 {
    let s = s.trim_start();
    let (neg, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let mut t = s;
    if radix == 0 {
        if s.starts_with("0x") || s.starts_with("0X") {
            radix = 16;
            t = &s[2..];
        } else if s.starts_with('0') {
            radix = 8;
        } else {
            radix = 10;
        }
    } else if radix == 16 && (s.starts_with("0x") || s.starts_with("0X")) {
        t = &s[2..];
    }
    let mut n = 0u32;
    for c in t.chars() {
        if let Some(d) = c.to_digit(radix) {
            n = n.wrapping_mul(radix).wrapping_add(d)
        } else {
            break;
        }
    }
    if neg {
        n.wrapping_neg() as i32
    } else {
        n as i32
    }
}
pub fn key(s: &str) -> i32 {
    if let Some(t) = s.strip_prefix('w') {
        WHITE[(number(t, 10) & 15) as usize] as i32
    } else if let Some(t) = s.strip_prefix('b') {
        BLACK[(number(t, 10) % 11) as usize] as i32
    } else {
        number(s, 10)
    }
}
#[derive(Debug, PartialEq)]
pub enum Comparison {
    Equal(i32),
    Below(i32),
    Above(i32),
}
impl Comparison {
    pub fn parse(s: &str) -> Self {
        match s.as_bytes().first() {
            Some(b'<') => Self::Below(number(&s[1..], 0)),
            Some(b'>') => Self::Above(number(&s[1..], 0)),
            _ => Self::Equal(number(s, 0)),
        }
    }
    pub fn accepts(&self, value: i32) -> bool {
        match *self {
            Self::Equal(n) => value == n,
            Self::Below(n) => value < n,
            Self::Above(n) => value > n,
        }
    }
}
#[test]
fn parser_keeps_c_widths_prefixes_and_strict_bounds() {
    assert_eq!(
        fields(b" turn K1 -4 80 #comment\n"),
        ["turn", "K1", "-4", "80"]
    );
    let long = vec![b'a'; 600];
    let r: Vec<_> = records(&long[..]).map(Result::unwrap).collect();
    assert_eq!((r[0].len(), r[1].len()), (511, 89));
    assert_eq!(fields(&long)[0].len(), 31);
    assert_eq!(
        (number("-0xff", 0), number("077z", 0), number("12tail", 10)),
        (-255, 63, 12)
    );
    assert!(Comparison::parse(">0x10").accepts(17));
    assert!(!Comparison::parse("<010").accepts(8));
    assert_eq!((key("w17"), key("b12")), (2, 3));
}
