#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Frozen corpus + current selectable C0 + maintained Rust + actual C ABI.
# Optional first arg: immutable R01 extraction, verified against all440 hashes.
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$repo"
out="$repo/build/factory"
mkdir -p "$out"
sha256sum -c tests/reference/factory.sha256
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/r01-verification.log"
    (cd "$baseline" && cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp -o "$out/r01-export" "$repo/tests/reference/factory_reference.c" -lm)
    "$out/r01-export" > "$out/r01.json"
    "$out/r01-export" --tables > "$out/r01-compact.json"
    cmp rust/fm1-engine/tests/factory/reference.json "$out/r01.json"
    cmp rust/fm1-engine/tests/factory/compact.json "$out/r01-compact.json"
    "$out/r01-export" --lifecycle-check 2> "$out/r01-reset-proof.json"
    (cd "$baseline" && cc -O2 -std=gnu99 -Ifirmware/src/dsp -o "$out/r01-boundaries" "$repo/tests/reference/factory_boundaries.c")
    "$out/r01-boundaries" > "$out/r01-boundaries.json"
    cmp rust/fm1-engine/tests/factory/boundaries.json "$out/r01-boundaries.json"
fi
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_FACTORY=0 -Ifirmware/src/dsp -o "$out/c0-export" tests/reference/factory_reference.c -lm
"$out/c0-export" > "$out/current-c0.json"
cmp rust/fm1-engine/tests/factory/reference.json "$out/current-c0.json"
"$out/c0-export" --bridge-check 2> "$out/c0-command-proof.json"
"$out/c0-export" --lifecycle-check 2> "$out/c0-reset-proof.json"
cargo +1.94.1 test --locked -p fm1-types -p fm1-engine -p fm1-jam-bridge
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_FACTORY=1 -Ifirmware/src/dsp -o "$out/rust-export" tests/reference/factory_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/rust-export" > "$out/rust-bridge.json"
cmp rust/fm1-engine/tests/factory/reference.json "$out/rust-bridge.json"
"$out/rust-export" --bridge-check 2> "$out/bridge-proof.json"
"$out/rust-export" --lifecycle-check 2> "$out/rust-reset-proof.json"
cat "$out/bridge-proof.json"
