#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +nightly-2026-08-28 miri test --locked -p fm1-sim --features c-app --bin fm1-sim foreign_uninitialized_destination_gets_only_verified_store_bytes
