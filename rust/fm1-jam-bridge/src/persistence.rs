// SPDX-License-Identifier: GPL-3.0-only
//! Synchronous bounded bridge to the platform's existing NOR hooks.
use fm1_engine::persistence::{Counts, Nor, Store, MAX};
use fm1_rt::atomic::{AtomicU32, Ordering};
static COUNTS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
pub(crate) fn project_call() {
    COUNTS[2].fetch_add(1, Ordering::Relaxed);
}
pub(crate) fn object_save(bytes: usize) {
    COUNTS[3].fetch_add(1, Ordering::Relaxed);
    COUNTS[7].fetch_add(bytes as u32, Ordering::Relaxed);
}
pub fn stat(index: u32) -> u32 {
    COUNTS
        .get(index.wrapping_sub(64) as usize)
        .map_or(0, |v| v.load(Ordering::Relaxed))
}
fn record(c: Counts) {
    for (counter, value) in COUNTS.iter().zip([
        c.loads,
        c.saves,
        0,
        0,
        c.missing,
        c.programs,
        c.verify_failed,
        c.bytes,
    ]) {
        counter.fetch_add(value, Ordering::Relaxed);
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_store_access(available: u32, locked: u32, restore: u32) -> i32 {
    fm1_engine::persistence::access(available != 0, locked != 0, restore != 0)
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
/// any pointer. scratch covers MAX initialized writable bytes, exclusively owned this call;
/// data covers length bytes and is disjoint from scratch/hooks. For load it is
/// writable (possibly uninitialized), for save initialized readable. No pointer
/// survives this call. A denied load does not read or write the destination.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_store(
    save: u32,
    object: u32,
    data: *mut u8,
    length: u32,
    scratch: *mut u8,
    hooks: *const Hooks,
) -> i32 {
    if (data.is_null() && length != 0)
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
        store.save(
            object,
            if length == 0 {
                &[]
            } else {
                unsafe { core::slice::from_raw_parts(data, length as usize) }
            },
        )
    } else {
        // C commonly supplies an uninitialized automatic struct. Never create
        // a typed reference into it: verification uses initialized st_buf only.
        match store.load_bytes(object, (length as usize).min(MAX)) {
            Err(rc) => rc,
            Ok(bytes) => {
                if !bytes.is_empty() {
                    // SAFETY: initialized verified source; caller's destination
                    // covers length writable bytes and is disjoint from scratch.
                    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
                }
                bytes.len() as i32
            }
        }
    };
    record(store.counts);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm1_engine::persistence::{sector, Header, BUSY, HEADER_LEN, PAYLOAD_OFFSET};
    unsafe extern "C" fn read(offset: u32, output: *mut u8, length: u32) -> i32 {
        let bytes = if offset == sector(0, 0) {
            Header {
                kind: 0,
                length: 16,
                checksum: fm1_formats::crc32(&[13; 16]),
                ..Header::default()
            }
            .encode()
        } else if offset == sector(0, 1) {
            [255; HEADER_LEN]
        } else {
            assert_eq!(offset, sector(0, 0) + PAYLOAD_OFFSET);
            [13; HEADER_LEN]
        };
        assert!(length as usize <= bytes.len());
        // SAFETY: synchronous hook receives the caller's live bounded scratch.
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), output, length as usize) };
        0
    }
    unsafe extern "C" fn busy(_: u32, _: *mut u8, _: u32) -> i32 {
        BUSY
    }
    unsafe extern "C" fn erase(_: u32) -> i32 {
        panic!("load tried to erase")
    }
    unsafe extern "C" fn program(_: u32, _: *const u8, _: u32) -> i32 {
        panic!("load tried to program")
    }
    #[test]
    fn foreign_uninitialized_load_is_initialized_only_after_verified_success() {
        let mut destination = core::mem::MaybeUninit::<[u8; 16]>::uninit();
        let mut scratch = [0; MAX];
        let mut hooks = Hooks {
            read: busy,
            erase,
            program,
        };
        // SAFETY: disjoint live output (initially uninitialized), initialized
        // scratch and synchronous hooks. No reference into output is formed.
        unsafe {
            assert_eq!(
                fm1_rust_store(
                    0,
                    0,
                    destination.as_mut_ptr().cast(),
                    16,
                    scratch.as_mut_ptr(),
                    &hooks
                ),
                BUSY
            );
            hooks.read = read;
            assert_eq!(
                fm1_rust_store(
                    0,
                    0,
                    destination.as_mut_ptr().cast(),
                    16,
                    scratch.as_mut_ptr(),
                    &hooks
                ),
                16
            );
            assert_eq!(destination.assume_init(), [13; 16]);
        }
    }
}
