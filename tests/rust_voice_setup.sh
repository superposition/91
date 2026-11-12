#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/voice-setup
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
common='-O2 -ffp-contract=off -std=gnu99 -D_POSIX_C_SOURCE=200809L -DEN_HOST'
flags='-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1'
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
for kind in data setup; do
    cc $common -I"$baseline/firmware/src/dsp" -o "$out/$kind-reference" "tests/reference/voice_${kind}_reference.c" -lm
    cc $common -Ifirmware/src/dsp -o "$out/$kind-c0" "tests/reference/voice_${kind}_reference.c" -lm
    cc $common $flags -Ifirmware/src/dsp -o "$out/$kind-rust" "tests/reference/voice_${kind}_reference.c" "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm
    for flavor in reference c0 rust; do
        "$out/$kind-$flavor" 1000 > "$out/$kind-$flavor-fixture.json"
        cmp "$out/$kind-$flavor-fixture.json" "rust/fm1-engine/tests/voice-$kind-native.json"
    done
done
cp "$out/setup-reference" "$out/reference"
cp "$out/setup-rust" "$out/rust"
cases=$(python3 -c 'print(*range(91),100,101,102)')
PARITY_JOBS=${PARITY_JOBS:-4} python3 tests/reference/transport_compare.py "$out" $cases
echo 'All voice recipe bits, slow controls, key allocation, release, cleanup and MIDI traces match frozen C.'
