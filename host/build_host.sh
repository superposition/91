#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Build the Rust native simulator into the existing build/host/amb_host path.
# EN_RUST_HOST=0 retains the original C0 runner; the app ABI is transitional.
set -e
cd "$(dirname "$0")/.."
if [ "${EN_RUST_HOST:-1}" != 0 ]; then
    export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.94.1}"
    cargo build --release --locked -p fm1-sim --features rust-engine --target-dir build/rust-native-host
    mkdir -p build/host
    cp build/rust-native-host/release/fm1-sim build/host/amb_host
    echo "host: build/host/amb_host (Rust simulator; transitional application ABI)"
    exit 0
fi
CC="${CC:-cc}"
mkdir -p build/gen build/host
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
$CC -O2 -ffp-contract=off -std=c99 -Wall -Wextra -Wno-unused-function -Wno-unused-parameter \
    -DEN_HOST -D_POSIX_C_SOURCE=200809L -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen -o build/host/amb_host host/amb_host.c \
    firmware/src/dsp/amb.c -lm
echo "host: build/host/amb_host"
