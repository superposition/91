// SPDX-License-Identifier: GPL-3.0-only
#![forbid(unsafe_code)]
//! Deterministic simulator policy, independent of the transitional app ABI.
pub mod clock;
pub mod platform;
pub mod scenario;
pub mod sinks;
