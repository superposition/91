#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Host pthread/deferred/busy worker evidence only; no physical deadlines claimed.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/effects-workers
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
cargo build --release --locked -p fm1-jam-bridge --target-dir build/effects-parity/rust-build
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" -o "$out/reference" tests/host/core2_test.c "$baseline/firmware/src/dsp/amb.c" -lpthread -lm
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp -o "$out/c0" tests/host/core2_test.c firmware/src/dsp/amb.c -lpthread -lm
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp -o "$out/rust" tests/host/core2_test.c firmware/src/dsp/amb.c build/effects-parity/rust-build/release/libfm1_jam_bridge.a -lpthread -ldl -lm
: > "$out/results.txt"
for pre in 0 100; do
for kind in 0 1 2; do
    for balance in 1 2; do
        for mode in 0 1 2 3; do
            "$out/reference" "$mode" "$out/reference.trace" "$kind" "$balance" 1 0 "$pre" 100 100 > "$out/reference.txt"
            "$out/c0" "$mode" "$out/c0.trace" "$kind" "$balance" 1 0 "$pre" 100 100 > "$out/c0.txt"
            "$out/rust" "$mode" "$out/rust.trace" "$kind" "$balance" 1 0 "$pre" 100 100 > "$out/rust.txt"
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
# The legacy harness has a synthetic "lost" case which abandons a live pthread;
# do not turn that unsafe host simulation into a claimed target retirement proof.
echo 'Frozen-C/fresh-C0/Rust serial, deferred, pthread and busy-worker FX output matches.'
