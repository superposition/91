// SPDX-License-Identifier: GPL-3.0-only
//! A separate synchronous codec module; the instrument's24-export ABI is unchanged.
#![cfg_attr(target_arch = "wasm32", no_std)]
#[cfg(target_arch = "wasm32")]
mod abi;
pub mod api;
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
