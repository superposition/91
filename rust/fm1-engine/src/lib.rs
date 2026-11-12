// SPDX-License-Identifier: GPL-3.0-only
//! Portable, allocation-free musical decisions, factory data and DSP kernels.
#![no_std]
#![forbid(unsafe_code)]

pub mod effects;
#[path = "factory_logic.rs"]
pub mod factory;
pub mod fm6;
pub mod melodic;
pub mod planner;
pub mod scheduling_tables;
pub mod sequencer;
pub mod transport;
pub mod voices;
