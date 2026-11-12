// SPDX-License-Identifier: GPL-3.0-only
//! Allocation-free interaction and presentation over caller-owned state/storage.
#![no_std]
#![forbid(unsafe_code)]
pub mod controller;
pub mod controls;
pub mod graphics;
#[cfg(feature = "host-platform")]
pub mod host_platform;
pub mod runner;
pub mod runner_draw;
pub mod text;
pub mod ui;
pub mod ui_state;
pub mod web_audio;
