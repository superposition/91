#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Python stdlib + Rust1.94.1. All PDF/Edge capabilities are deterministic doubles.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/manual/linux
export FM1_TOOLING_CLI="$PWD/build/manual/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
"${PYTHON:-python3}" tests/manual_differential.py "$baseline" "$FM1_TOOLING_CLI"
"${PYTHON:-python3}" tests/manual_pdf_policy.py "$baseline" "$FM1_TOOLING_CLI"
