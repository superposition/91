#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge --features web retained_host_roots_and_uninitialized_outputs_survive_reentrant_app_callbacks
