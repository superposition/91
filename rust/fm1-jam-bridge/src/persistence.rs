// SPDX-License-Identifier: GPL-3.0-only
//! Synchronous bounded bridge to the platform's existing NOR hooks.
use fm1_engine::persistence::{Counts, Nor, Store, MAX};
use fm1_rt::atomic::{AtomicU32, Ordering};
static COUNTS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
pub fn stat(index: u32) -> u32 {
    COUNTS
        .get(index.wrapping_sub(64) as usize)
        .map_or(0, |v| v.load(Ordering::Relaxed))
}
fn record(c: Counts) {
    for (counter, value) in COUNTS.iter().zip([
        c.loads,
        c.saves,
        c.selected_a,
        c.selected_b,
        c.missing,
        c.programs,
        c.verify_failed,
        c.bytes,
    ]) {
        counter.fetch_add(value, Ordering::Relaxed);
    }
}
#[repr(C)]
pub struct Hooks {
    pub read: unsafe extern "C" fn(u32, *mut u8, u32) -> i32,
    pub erase: unsafe extern "C" fn(u32) -> i32,
    pub program: unsafe extern "C" fn(u32, *const u8, u32) -> i32,
}
struct Platform<'a>(&'a Hooks);
impl Nor for Platform<'_> {
    fn read(&mut self, off: u32, output: &mut [u8]) -> i32 {
        // SAFETY: synchronous caller contract; output is live and unique until return.
        unsafe { (self.0.read)(off, output.as_mut_ptr(), output.len() as u32) }
    }
    fn erase(&mut self, off: u32) -> i32 {
        // SAFETY: the platform owns flash bounds and the synchronous erase primitive.
        unsafe { (self.0.erase)(off) }
    }
    fn program(&mut self, off: u32, input: &[u8]) -> i32 {
        // SAFETY: initialized RAM source is live throughout this synchronous call.
        unsafe { (self.0.program)(off, input.as_ptr(), input.len() as u32) }
    }
}
/// # Safety
/// Hooks are valid synchronous functions, may not re-enter the store or retain
/// any pointer. scratch covers MAX writable bytes, exclusively owned this call;
/// data covers length bytes and is disjoint from scratch/hooks. For load it is
/// writable, for save initialized readable. No pointer survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_store(
    save: u32,
    object: u32,
    data: *mut u8,
    length: u32,
    scratch: *mut u8,
    hooks: *const Hooks,
) -> i32 {
    if data.is_null()
        || scratch.is_null()
        || hooks.is_null()
        || save > 1
        || (save == 1 && length as usize > MAX)
    {
        return -1;
    }
    // SAFETY: fixed scratch and hook lifetime are guaranteed by the C caller.
    let mut store = Store::new(Platform(unsafe { &*hooks }), unsafe {
        &mut *scratch.cast::<[u8; MAX]>()
    });
    let result = if save == 1 {
        // SAFETY: checked bounded initialized input disjoint from scratch.
        store.save(object, unsafe {
            core::slice::from_raw_parts(data, length as usize)
        })
    } else {
        // No load can write more than MAX even if the caller offers more space.
        // SAFETY: caller guarantees this writable span and exclusive access.
        store.load(object, unsafe {
            core::slice::from_raw_parts_mut(data, (length as usize).min(MAX))
        })
    };
    record(store.counts);
    result
}
