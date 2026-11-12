#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Foreground DJ borrows plus the existing worker harness; no physical proof.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/dj-workers
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
common='-O2 -ffp-contract=off -std=gnu99 -DEN_HOST'
cc $common -I"$baseline/firmware/src/dsp" tests/reference/dj_workers.c "$baseline/firmware/src/dsp/amb.c" -lpthread -lm -o "$out/reference"
cc $common -Ifirmware/src/dsp tests/reference/dj_workers.c firmware/src/dsp/amb.c -lpthread -lm -o "$out/c0"
cc $common -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp tests/reference/dj_workers.c firmware/src/dsp/amb.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/rust"
: > "$out/results.txt"
for pre in 0 100; do
for kind in 0 1 2; do
for balance in 1 2; do
for mode in 0 1 2 3; do
    for flavor in reference c0 rust; do
        "$out/$flavor" "$mode" "$out/$flavor.trace" "$kind" "$balance" 1 0 "$pre" 100 100 > "$out/$flavor.txt"
    done
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference.txt" "$out/c0.txt"
    cmp "$out/reference.txt" "$out/rust.txt"
    printf 'predelay=%s type=%s balance=%s ' "$pre" "$kind" "$balance" >> "$out/results.txt"
    cat "$out/rust.txt" | tee -a "$out/results.txt"
done
done
done
done
echo 'Frozen-C/current-C0/Rust active DJ audio matches in48serial/deferred/threaded/busy worker configurations.'
