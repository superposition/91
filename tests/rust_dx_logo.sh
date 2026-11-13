#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Native generator and both bank-reader consumers. Python needs Pillow + mido.
# No C application, browser, USB/MIDI handle, audio device or download is used.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/dx-logo/linux
export FM1_TOOLING_CLI="$PWD/build/dx-logo/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
export FM1_AUTHORING_CLI="$FM1_TOOLING_CLI"
"${PYTHON:-python3}" tests/dx_logo_differential.py "$baseline" "$FM1_TOOLING_CLI"
"${PYTHON:-python3}" tests/authoring_differential.py "$baseline" "$FM1_TOOLING_CLI" build/dx-logo/authoring
"${PYTHON:-python3}" tests/assets_differential.py "$baseline" "$FM1_TOOLING_CLI"
