#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Both FIFO roles and both MarkI paths, through the actual job owner/scheduler.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/fm6-workers
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir build/fm6-parity/rust-build
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/host/core2_test.c "$baseline/firmware/src/dsp/amb.c" -lpthread -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/host/core2_test.c firmware/src/dsp/amb.c -lpthread -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp tests/host/core2_test.c firmware/src/dsp/amb.c build/fm6-parity/rust-build/release/libfm1_jam_bridge.a -lpthread -ldl -lm -o "$out/rust"
: > "$out/results.txt"
for songbook in 0 2; do
for float in 0 1; do
for balance in 1 2; do
for mode in 0 1 2 3; do
    "$out/reference" "$mode" "$out/reference.trace" 0 "$balance" "$float" "$songbook" 100 100 100 > "$out/reference.txt"
    "$out/c0" "$mode" "$out/c0.trace" 0 "$balance" "$float" "$songbook" 100 100 100 > "$out/c0.txt"
    "$out/rust" "$mode" "$out/rust.trace" 0 "$balance" "$float" "$songbook" 100 100 100 > "$out/rust.txt"
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference.txt" "$out/c0.txt"
    cmp "$out/reference.txt" "$out/rust.txt"
    printf 'songbook=%s float=%s balance=%s ' "$songbook" "$float" "$balance" >> "$out/results.txt"
    cat "$out/rust.txt" | tee -a "$out/results.txt"
done
done
done
done
echo 'Frozen-C/fresh-C0/Rust legacy and two-role FM worker PCM match; host completion only.'
