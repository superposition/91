#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Entire native authoring gate. Linux CI requires installed mido, NumPy, Pillow,
# a C99 compiler, and pinned Rust1.94.1. Never opens MIDI/audio ports.
# Usage: bash tests/rust_authoring.sh FROZEN_R01_SOURCE
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
python3 -c 'import mido,numpy,PIL'
cargo +1.94.1 build --release --locked -p fm1-cli
export FM1_AUTHORING_CLI="$PWD/target/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_AUTHORING_CLI"
python3 tests/authoring_differential.py "$baseline" "$FM1_AUTHORING_CLI"
python3 tests/authoring_device.py "$baseline" "$FM1_AUTHORING_CLI"
python3 tests/authoring_acceptance.py "$baseline" "$FM1_AUTHORING_CLI"
python3 tests/authoring_cli.py "$baseline" "$FM1_AUTHORING_CLI"
python3 tests/jam_tools_test.py
mkdir -p build/gen build/host
python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
"${CC:-cc}" -O2 -ffp-contract=off -std=c99 -D_POSIX_C_SOURCE=200809L -Wno-unused-function \
    -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen -o build/host/jam_transport_bridge \
    tests/host/jam_transport_bridge.c firmware/src/dsp/amb.c -lm
python3 tests/jam_transport_test.py
python3 tests/authoring_preview.py "$baseline" "$FM1_AUTHORING_CLI"
