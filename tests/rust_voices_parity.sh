#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/voices
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
common='-O2 -ffp-contract=off -std=gnu99 -D_POSIX_C_SOURCE=200809L -DEN_HOST'
cc $common -I"$baseline/firmware/src/dsp" -o "$out/reference" tests/reference/voices_reference.c -lm
cc $common -Ifirmware/src/dsp -o "$out/c0" tests/reference/voices_reference.c -lm
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc $common -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 \
    -Ifirmware/src/dsp -o "$out/rust" tests/reference/voices_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm
for flavor in reference c0 rust; do
    "$out/$flavor" 1000 > "$out/$flavor-fixture.json"
    cmp "$out/$flavor-fixture.json" rust/fm1-engine/tests/voices-native.json
done
cases=$(python3 -c 'print(*range(98),*range(100,138))')
PARITY_JOBS=${PARITY_JOBS:-4} python3 tests/reference/transport_compare.py "$out" $cases
python3 tests/reference/voices_benchmark.py "$out"
echo 'Isolated percussion/plain-pluck state and PCM match frozen C.'
