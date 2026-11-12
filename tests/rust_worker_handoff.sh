#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/worker-handoff
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/host/worker_handoff_test.c -lpthread -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/host/worker_handoff_test.c -lpthread -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_WORKER_TEST=1 -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 \
    -Ifirmware/src/dsp tests/host/worker_handoff_test.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/rust"
: > "$out/results.jsonl"
for songbook in 0 1; do
for balance in 1 2; do
    "$out/reference" 0 "$out/reference.trace" "$songbook" "$balance" > "$out/reference-$songbook-$balance.json"
    "$out/c0" 0 "$out/c0.trace" "$songbook" "$balance" > "$out/c0-$songbook-$balance.json"
    cmp "$out/reference.trace" "$out/c0.trace"
    for mode in 0 1 2 3 10 11; do
        "$out/rust" "$mode" "$out/rust.trace" "$songbook" "$balance" > "$out/rust-$songbook-$balance-$mode.json"
        cmp "$out/reference.trace" "$out/rust.trace"
        cat "$out/rust-$songbook-$balance-$mode.json" | tee -a "$out/results.jsonl"
    done
done
for mode in 4 5 6 7 8 9; do
    "$out/rust" "$mode" "$out/fault.trace" "$songbook" 2 > "$out/fault-$songbook-$mode.json"
    cat "$out/fault-$songbook-$mode.json" | tee -a "$out/results.jsonl"
done
done
echo 'Frozen serial PCM/MIDI/UI order matches every valid worker mode; confirmed loss, quarantine and stale callbacks pass.'
