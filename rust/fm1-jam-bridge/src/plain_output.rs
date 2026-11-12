// SPDX-License-Identifier: GPL-3.0-only
//! HAL0 output has no DMA registry/global state/provider dependency. Physical
//! retained DMA publication must explicitly enable the bridge `hal` feature.
pub(crate) struct Sink {
    out: *mut i32,
    words: usize,
}
impl Sink {
    /// # Safety
    /// Aligned live output disjoint from DSP/USB state; no other CPU writer or
    /// Rust reference into it is live. Reads require initialized words. Foreign
    /// DMA readers may observe interleaved raw stores: HAL0 grants no deadline,
    /// snapshot or completion guarantee. The caller retains the allocation.
    /// No pointer escapes a call and volatile supplies no exclusion/atomicity.
    pub(crate) unsafe fn new(out: *mut i32, words: usize) -> Self {
        Self { out, words }
    }
    pub(crate) fn write(&mut self, index: usize, value: i32) -> bool {
        if index >= self.words {
            return false;
        }
        // SAFETY: constructor's bounded external allocation contract.
        unsafe { self.out.add(index).write_volatile(value) };
        true
    }
    pub(crate) fn read(&mut self, index: usize) -> Option<i32> {
        if index >= self.words {
            return None;
        }
        // SAFETY: caller initializes output before a fade/read operation.
        Some(unsafe { self.out.add(index).read_volatile() })
    }
}
