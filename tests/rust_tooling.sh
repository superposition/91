#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Linux host-only gate. Python3.10+, pinned Rust1.94.1; no third-party Python
# dependencies, MIDI import, ports, audio I/O, firmware build or hardware needed.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/tooling/linux
export FM1_TOOLING_CLI="$PWD/build/tooling/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
python3 tests/tooling_differential.py "$baseline" "$FM1_TOOLING_CLI"
python3 tests/install_test.py
python3 tools/fm1log.py --selftest
