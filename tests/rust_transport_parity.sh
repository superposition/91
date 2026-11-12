#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Compare the real native instrument's timing/MIDI/PCM traces. Optionally pass
# the frozen R01 source directory; its complete manifest is verified first.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/transport-parity
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST \
    -I"$baseline/firmware/src/dsp" -o "$out/reference" tests/reference/transport_reference.c -lm
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST \
    -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 \
    -Ifirmware/src/dsp -o "$out/rust" tests/reference/transport_reference.c \
    "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm
"$out/reference" 8 > "$out/tempo.json"
cmp "$out/tempo.json" rust/fm1-engine/tests/transport-tempo.json
"$out/rust" 8 > "$out/tempo-rust.json"
cmp "$out/tempo.json" "$out/tempo-rust.json"
for scenario in 0 1 2 3 4 5 6 7; do
    "$out/reference" "$scenario" "$out/reference-$scenario.trace" > "$out/reference-$scenario.json"
    "$out/rust" "$scenario" "$out/rust-$scenario.trace" > "$out/rust-$scenario.json"
    cmp "$out/reference-$scenario.trace" "$out/rust-$scenario.trace"
    cmp "$out/reference-$scenario.json" "$out/rust-$scenario.json"
    cat "$out/rust-$scenario.json"
done
echo 'Native transport phase/chunk/control/MIDI/PCM traces match.'
