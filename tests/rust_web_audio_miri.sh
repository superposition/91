#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Separate interpreter gate; production/native/WASM compiler remains Rust1.94.1.
set -eu
cd "$(dirname "$0")/.."
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features web web_audio::tests::retained_raw_views_remain_valid_between_exclusive_updates
