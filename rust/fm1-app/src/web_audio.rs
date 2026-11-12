// SPDX-License-Identifier: GPL-3.0-only
//! Serialized browser audio storage; no callback or allocation during a borrow.
pub const RING_FRAMES: usize = 4096;
pub const MAX_RENDER: usize = 1024;

pub struct Audio {
    ring: [[f32; 2]; RING_FRAMES],
    write: u32,
    read: u32,
}
impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
impl Audio {
    pub const fn new() -> Self {
        Self {
            ring: [[0.0; 2]; RING_FRAMES],
            write: 0,
            read: 0,
        }
    }
    pub fn available(&self) -> u32 {
        self.write.wrapping_sub(self.read)
    }
    /// Preserve the legacy overwrite-on-overrun ring. The normal renderer asks
    /// for at most 1024 frames and produces 256 at a time, so at most 1279 wait.
    /// Reject a malformed odd sample span before changing any state.
    pub fn push(&mut self, stereo: &[i32]) -> bool {
        if stereo.len() & 1 != 0 {
            return false;
        }
        for frame in stereo.chunks_exact(2) {
            let at = self.write as usize % RING_FRAMES;
            self.write = self.write.wrapping_add(1);
            self.ring[at] = [frame[0] as f32 / 8388608.0, frame[1] as f32 / 8388608.0];
        }
        true
    }
    /// The adapter runs the device between calls until this bounded request is
    /// available. A short ring is rejected without consuming or clearing tails.
    pub fn drain(
        &mut self,
        requested: u32,
        left: &mut [f32; MAX_RENDER],
        right: &mut [f32; MAX_RENDER],
    ) -> Option<usize> {
        let n = requested.min(MAX_RENDER as u32) as usize;
        if self.available() < n as u32 {
            return None;
        }
        for k in 0..n {
            let at = self.read as usize % RING_FRAMES;
            self.read = self.read.wrapping_add(1);
            left[k] = self.ring[at][0];
            right[k] = self.ring[at][1];
        }
        Some(n)
    }
}
pub fn background(colors: [u16; 4], output: &mut [f32; 12]) {
    for (k, color) in colors.iter().copied().enumerate() {
        output[k * 3] = (color >> 11) as f32 / 31.0;
        output[k * 3 + 1] = ((color >> 5) & 63) as f32 / 63.0;
        output[k * 3 + 2] = (color & 31) as f32 / 31.0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_and_short_requests_do_not_consume_or_clear() {
        let mut audio = Audio::new();
        let (mut left, mut right) = ([0.0; MAX_RENDER], [0.0; MAX_RENDER]);
        assert!(!audio.push(&[1]));
        assert_eq!(audio.available(), 0);
        assert_eq!(audio.drain(1, &mut left, &mut right), None);
        assert_eq!(audio.drain(0, &mut left, &mut right), Some(0));
        assert!(audio.push(&[-8388608, 8388607]));
        assert_eq!(audio.drain(2, &mut left, &mut right), None);
        assert_eq!(audio.available(), 1);
        assert_eq!(left[0].to_bits(), 0);
        assert_eq!(audio.drain(1, &mut left, &mut right), Some(1));
        assert_eq!(left[0], -1.0);
        assert_eq!(right[0], 8388607.0 / 8388608.0);
        assert_eq!(left[1].to_bits(), 0);
    }
    #[test]
    fn wrapping_indices_and_overwrite_follow_unsigned_ring_contract() {
        let mut audio = Audio::new();
        let (mut left, mut right) = ([0.0; MAX_RENDER], [0.0; MAX_RENDER]);
        audio.write = u32::MAX - 1;
        audio.read = u32::MAX - 1;
        assert!(audio.push(&[1, -1, 2, -2, 3, -3]));
        assert_eq!(audio.write, 1);
        assert_eq!(audio.available(), 3);
        assert_eq!(audio.drain(3, &mut left, &mut right), Some(3));
        assert_eq!(audio.read, 1);
        assert_eq!(left[2], 3.0 / 8388608.0);
        for k in 0..RING_FRAMES + 1 {
            assert!(audio.push(&[k as i32, -(k as i32)]));
        }
        assert_eq!(audio.available(), RING_FRAMES as u32 + 1);
        assert_eq!(
            audio.drain(u32::MAX, &mut left, &mut right),
            Some(MAX_RENDER)
        );
        assert_eq!(left[0], RING_FRAMES as f32 / 8388608.0);
    }
}
