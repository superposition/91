#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Requires Python with Pillow and a native C compiler; no app/audio/device/USB.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/assets/linux
export FM1_TOOLING_CLI="$PWD/build/assets/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
"${PYTHON:-python3}" tests/assets_differential.py "$baseline" "$FM1_TOOLING_CLI"
