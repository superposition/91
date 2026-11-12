#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/fm6-parity
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/fm6_runtime_reference.c -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/fm6_runtime_reference.c -lm -o "$out/c0"
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp tests/reference/fm6_runtime_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/rust"
: > "$out/results.jsonl"
for scenario in ${FM6_CASES:-0 1 2 3 4 5 6 7}; do
    "$out/reference" "$scenario" "$out/reference.trace" > "$out/reference-$scenario.json"
    "$out/c0" "$scenario" "$out/c0.trace" > "$out/c0-$scenario.json"
    "$out/rust" "$scenario" "$out/rust.trace" > "$out/rust-$scenario.json"
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference-$scenario.json" "$out/c0-$scenario.json"
    cmp "$out/reference-$scenario.json" "$out/rust-$scenario.json"
    cat "$out/rust-$scenario.json" | tee -a "$out/results.jsonl"
done
echo 'Frozen-C/fresh-C0/Rust FM routing, pool, state and raw PCM match.'
