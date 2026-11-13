#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Linux: stdlib Python3.10+, C99 compiler, Node and pinned Rust. No device/server.
set -eu
cd "$(dirname "$0")/.."
baseline=$(realpath "${1:?immutable R01 source directory}")
export RUSTUP_TOOLCHAIN=1.94.1
cargo +1.94.1 build --release --locked -p fm1-cli --target-dir build/site/linux
export FM1_TOOLING_CLI="$PWD/build/site/linux/release/fm1-rs"
export FM1_CODEC_CLI="$FM1_TOOLING_CLI"
mkdir -p build/site/provider
"${CC:-cc}" -O1 -std=c99 -DEN_HOST -Ifirmware/src -Ifirmware/src/dsp \
    -o build/site/provider/gparams tests/host/gparams.c firmware/src/dsp/amb.c -lm
build/site/provider/gparams >build/site/provider/params.json
build/site/provider/gparams pieces >build/site/provider/names.json
"${CC:-cc}" -O1 -std=c99 -DEN_HOST -I"$baseline/firmware/src" -I"$baseline/firmware/src/dsp" \
    -o build/site/provider/frozen-gparams "$baseline/tests/host/gparams.c" "$baseline/firmware/src/dsp/amb.c" -lm
build/site/provider/frozen-gparams >build/site/provider/frozen-params.json
build/site/provider/frozen-gparams pieces >build/site/provider/frozen-names.json
python3 tools/make_params.py --check
python3 tests/site_metadata_differential.py "$baseline" "$FM1_TOOLING_CLI"
python3 tests/site_build_differential.py "$FM1_TOOLING_CLI"
