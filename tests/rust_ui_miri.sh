#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# The interpreter gate is separate from pinned1.94.1 production compilation.
set -eu
cd "$(dirname "$0")/.."
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +nightly-2026-08-28 miri test --locked -p fm1-jam-bridge ui::tests::
