// SPDX-License-Identifier: GPL-3.0-only
//! NUL-terminated byte text. Glyph selection is byte-oriented like the original.
pub struct Text<'a> {
    bytes: &'a mut [u8],
    length: usize,
    truncated: bool,
}
impl<'a> Text<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        if let Some(first) = bytes.first_mut() {
            *first = 0;
        }
        Self {
            bytes,
            length: 0,
            truncated: false,
        }
    }
    pub fn push(&mut self, byte: u8) {
        if self.length + 1 < self.bytes.len() {
            self.bytes[self.length] = byte;
            self.length += 1;
            self.bytes[self.length] = 0;
        } else {
            self.truncated = true;
        }
    }
    pub fn append(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().take_while(|byte| **byte != 0) {
            self.push(*byte);
        }
    }
    pub fn capitals(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().take_while(|byte| **byte != 0) {
            self.push(if byte.is_ascii_lowercase() {
                byte - 32
            } else {
                *byte
            });
        }
    }
    pub fn unsigned(&mut self, mut number: u32) {
        let mut digits = [0; 10];
        let mut n = 0;
        loop {
            digits[n] = b'0' + (number % 10) as u8;
            n += 1;
            number /= 10;
            if number == 0 {
                break;
            }
        }
        for byte in digits[..n].iter().rev() {
            self.push(*byte);
        }
    }
    pub fn signed(&mut self, number: i32) {
        if number < 0 {
            self.push(b'-');
        }
        self.unsigned(number.unsigned_abs());
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}
