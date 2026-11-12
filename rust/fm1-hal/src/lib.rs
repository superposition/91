// SPDX-License-Identifier: GPL-3.0-only
#![no_std]
#![forbid(unsafe_code)]
//! Bounded FM-1 startup decisions and register sequences. Device/ISA access is
//! supplied by the reviewed backend; volatile is not an ownership/atomicity rule.
pub mod boot;
pub mod dma;
pub mod drivers;
pub mod flash;
pub mod io;
pub mod midi;
pub mod usb_control;
pub mod usb_descriptors;
pub mod usb_transport;
