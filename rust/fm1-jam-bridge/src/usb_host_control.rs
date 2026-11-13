// SPDX-License-Identifier: GPL-3.0-only
use fm1_hal::{
    usb_endpoint,
    usb_host_control::{self, Io, Request},
};
#[repr(C)]
pub struct Context {
    buffer: *mut u8,
    clock: *const u32,
    lease: *mut u8,
    error: *mut u8,
    write: Option<unsafe extern "C" fn(u32, u32) -> u32>,
    read: Option<unsafe extern "C" fn(u32) -> u32>,
    send: Option<unsafe extern "C" fn(*mut u8, u32)>,
    sync: Option<unsafe extern "C" fn()>,
    feed: Option<unsafe extern "C" fn()>,
}
const _: [(); 9 * core::mem::size_of::<*mut u8>()] = [(); core::mem::size_of::<Context>()];
struct Raw<'a> {
    context: &'a Context,
    output: *mut u8,
}
// SAFETY: immutable initialized descriptor/request are private CPU memory.
// All foreign spans use original raw roots, with safe-core fixed access bounds;
// no DMA/output/index/reference is constructed. Callbacks are disjoint leaves.
impl Io for Raw<'_> {
    fn now(&self) -> u32 {
        unsafe { self.context.clock.read_volatile() }
    }
    fn read(&mut self, register: u32) -> Option<u8> {
        self.context.read.and_then(|f| {
            let v = unsafe { f(register) };
            (v & 256 != 0).then_some(v as u8)
        })
    }
    fn write(&mut self, register: u32, value: u32) -> bool {
        self.context
            .write
            .is_some_and(|f| unsafe { f(register, value) != 0 })
    }
    fn phase(&self) -> u8 {
        unsafe { self.context.lease.read_volatile() }
    }
    fn lease(&mut self, event: u32) {
        let state = unsafe { self.context.lease.read_volatile() };
        let plan = usb_endpoint::transition(u32::from(state), 0, event);
        unsafe { self.context.lease.write_volatile(plan as u8) };
    }
    fn error(&mut self, value: u8) {
        unsafe { self.context.error.write_volatile(value) }
    }
    fn buffer(&self, index: usize) -> u8 {
        unsafe { self.context.buffer.add(index).read_volatile() }
    }
    fn set_buffer(&mut self, index: usize, value: u8) {
        unsafe { self.context.buffer.add(index).write_volatile(value) }
    }
    fn output(&mut self, index: usize, value: u8) {
        unsafe { self.output.add(index).write(value) }
    }
    fn send(&mut self, count: u32) {
        if let Some(f) = self.context.send {
            unsafe { f(self.context.buffer, count) }
        }
    }
    fn sync(&mut self) {
        if let Some(f) = self.context.sync {
            unsafe { f() }
        }
    }
    fn feed(&mut self) {
        if let Some(f) = self.context.feed {
            unsafe { f() }
        }
    }
}
/// # Safety
/// Context/request are initialized immutable CPU-owned spans, disjoint from
/// existing EP0 raw buffer68B, aligned clock, lease/error bytes and output's
/// writable requested prefix (possibly uninitialized). Callbacks only perform
/// observed controller/cache/DMA/watchdog operations and cannot enter this
/// producer, application/engine or foreign views. Foreground owns indexed SIE,
/// device/host polling disabled, IRQs enabled and clock progressing. No DMA or
/// uninitialized-output reference is formed, no root retained. IN output capacity
/// is at least request.length<=768; OUT requests have no data stage.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_usb_host_control(
    context: *const Context,
    request: *const Request,
    output: *mut u8,
) -> i32 {
    if context.is_null()
        || request.is_null()
        || context as usize & (core::mem::align_of::<Context>() - 1) != 0
        || request as usize & (core::mem::align_of::<Request>() - 1) != 0
    {
        return -3;
    }
    // SAFETY: initialized immutable descriptors; neither callback touches them.
    let (c, p) = unsafe { (&*context, &*request) };
    if c.buffer.is_null()
        || c.clock.is_null()
        || c.lease.is_null()
        || c.error.is_null()
        || c.clock as usize & 3 != 0
        || c.write.is_none()
        || c.read.is_none()
        || c.send.is_none()
        || c.sync.is_none()
        || c.feed.is_none()
        || (p.request_type & 0x80 != 0 && p.length != 0 && output.is_null())
    {
        return -3;
    }
    usb_host_control::transfer(&mut Raw { context: c, output }, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        ptr::{addr_of, addr_of_mut},
    };
    struct Hardware {
        buffer: [u8; 68],
        clock: u32,
        lease: u8,
        error: u8,
        csr: u8,
        unknown: u8,
        sends: u32,
        setup: [u8; 8],
    }
    std::thread_local! {static HW:UnsafeCell<Hardware>=const{UnsafeCell::new(Hardware{buffer:[0xa5;68],clock:123,lease:0,error:0x55,csr:0,unknown:0,sends:0,setup:[0;8]})};}
    unsafe extern "C" fn write(register: u32, value: u32) -> u32 {
        HW.with(|cell| unsafe {
            let root = cell.get();
            if register == usb_host_control::CSR {
                let csr = match value {
                    0x20 => {
                        for (i, b) in [17, 34, 51].into_iter().enumerate() {
                            addr_of_mut!((*root).buffer).cast::<u8>().add(i).write(b);
                        }
                        1
                    }
                    0x60 => 1,
                    _ => 0,
                };
                addr_of_mut!((*root).csr).write(csr);
            }
            1
        })
    }
    unsafe extern "C" fn read(register: u32) -> u32 {
        HW.with(|cell| unsafe {
            let root = cell.get();
            if addr_of!((*root).unknown).read() != 0 {
                return 0;
            }
            256 | if register == usb_host_control::COUNT {
                3
            } else {
                u32::from(addr_of!((*root).csr).read())
            }
        })
    }
    unsafe extern "C" fn send(buffer: *mut u8, count: u32) {
        HW.with(|cell| unsafe {
            let root = cell.get();
            assert_eq!(count, 8);
            assert_eq!(buffer, addr_of_mut!((*root).buffer).cast::<u8>());
            for i in 0..8 {
                addr_of_mut!((*root).setup)
                    .cast::<u8>()
                    .add(i)
                    .write(buffer.add(i).read());
            }
            let sends = addr_of!((*root).sends).read();
            addr_of_mut!((*root).sends).write(sends + 1);
        });
    }
    unsafe extern "C" fn sync() {}
    unsafe extern "C" fn feed() {
        HW.with(|cell| unsafe {
            let p = addr_of_mut!((*cell.get()).clock);
            p.write(p.read().wrapping_add(1));
        });
    }
    #[test]
    fn raw_control_initializes_only_completed_uninitialized_output_prefix_and_seals_unknown() {
        HW.with(|cell| unsafe {
            // Original cell roots model C globals/retained DMA. No temporary
            // buffer or whole-Hardware references are made, including in leaves.
            let root = cell.get();
            let context = Context {
                buffer: addr_of_mut!((*root).buffer).cast(),
                clock: addr_of!((*root).clock),
                lease: addr_of_mut!((*root).lease),
                error: addr_of_mut!((*root).error),
                write: Some(write),
                read: Some(read),
                send: Some(send),
                sync: Some(sync),
                feed: Some(feed),
            };
            let request = Request {
                address: 3,
                request_type: 0x80,
                request: 6,
                maximum: 8,
                value: 0x1234,
                index: 0xabcd,
                length: 8,
                reserved: 0,
            };
            let mut output = MaybeUninit::<[u8; 8]>::uninit();
            assert_eq!(
                fm1_rust_usb_host_control(&context, &request, output.as_mut_ptr().cast()),
                3
            );
            let prefix = core::slice::from_raw_parts(output.as_ptr().cast::<u8>(), 3);
            assert_eq!(prefix, &[17, 34, 51]); // the remaining five bytes stay uninitialized
            assert_eq!(
                addr_of!((*root).setup).read(),
                [0x80, 6, 0x34, 0x12, 0xcd, 0xab, 8, 0]
            );
            assert_eq!(addr_of!((*root).lease).read(), 0);
            assert_eq!(addr_of!((*root).error).read(), 0x55);
            addr_of_mut!((*root).unknown).write(1);
            let mut failed_output = MaybeUninit::<[u8; 8]>::uninit();
            assert_eq!(
                fm1_rust_usb_host_control(&context, &request, failed_output.as_mut_ptr().cast()),
                -3
            );
            assert_eq!(addr_of!((*root).lease).read() & 3, 3);
            assert_eq!(addr_of!((*root).error).read(), 3);
            let source = addr_of!((*root).buffer).read();
            let count = addr_of!((*root).sends).read();
            addr_of_mut!((*root).unknown).write(0);
            addr_of_mut!((*root).csr).write(0);
            assert_eq!(
                fm1_rust_usb_host_control(&context, &request, failed_output.as_mut_ptr().cast()),
                -3
            );
            assert_eq!(addr_of!((*root).buffer).read(), source);
            assert_eq!(addr_of!((*root).sends).read(), count);
        });
    }
}
