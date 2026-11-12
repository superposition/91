// SPDX-License-Identifier: GPL-3.0-only
//! Portable, allocation-free musical engine. DSP migration is tracked separately.
#![no_std]
#![forbid(unsafe_code)]

pub mod melodic;
pub mod planner;
pub mod scheduling_tables;
pub mod sequencer;
pub mod transport;
