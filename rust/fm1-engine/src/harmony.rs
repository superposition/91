// SPDX-License-Identifier: GPL-3.0-only
//! Legacy degree/chord policy, distinct from the borrowed Songbook planner.
//! Candidate order, integer remainders and RNG boundaries are musical behavior.
pub mod basic;
#[rustfmt::skip]
pub mod abi;
pub mod legacy;
mod tables;
