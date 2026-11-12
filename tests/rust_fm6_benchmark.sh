#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/fm6-benchmark
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir build/fm6-parity/rust-build
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/fm6_benchmark.c -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/fm6_benchmark.c -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp tests/reference/fm6_benchmark.c build/fm6-parity/rust-build/release/libfm1_jam_bridge.a -lpthread -ldl -lm -o "$out/rust"
python3 tests/reference/fm6_benchmark.py "$out"
