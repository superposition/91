// SPDX-License-Identifier: GPL-3.0-only
//! Portable, allocation-free musical engine. DSP migration is tracked separately.
#![no_std]
#![forbid(unsafe_code)]

#[path = "factory_logic.rs"]
pub mod factory;
pub mod planner;
pub mod sequencer;
pub mod transport;
