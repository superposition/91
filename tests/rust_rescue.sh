#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# No libusb, MIDI package, device, driver, network download or privileged user
# is needed. Cargo fetches only the exact locked host dependencies if absent.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/rescue/linux
export FM1_TOOLING_CLI="$PWD/build/rescue/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
python3 tests/rescue_differential.py "$baseline" "$FM1_TOOLING_CLI"
python3 tests/rescue_bulk.py "$baseline" "$FM1_TOOLING_CLI"
python3 tests/rescue_capture.py "$baseline" "$FM1_TOOLING_CLI"
# The shared adapter's recoverable-error extension must preserve #22's first
# checkpoint and its fake-device/public CLI contracts.
python3 tests/tooling_differential.py "$baseline" "$FM1_TOOLING_CLI"
python3 tests/install_test.py
python3 tools/fm1log.py --selftest
