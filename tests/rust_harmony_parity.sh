#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/harmony-parity
mkdir -p "$out"
python3 tools/harmony_abi.py --check
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/harmony_reference.c -lpthread -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/harmony_reference.c -lpthread -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 -DEN_RUST_HARMONY=1 -Ifirmware/src/dsp tests/reference/harmony_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/rust"
# The harmonic boundary is independently selectable; other ports remain C0.
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_HARMONY=1 -Ifirmware/src/dsp tests/reference/harmony_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/harmony-only"
: > "$out/results.jsonl"
for mode in ${HARMONY_CASES:-0 1 2 3 4 5};do
    for impl in reference c0 rust harmony-only;do "$out/$impl" "$mode" "$out/$impl.trace" > "$out/$impl-$mode.json";done
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference.trace" "$out/harmony-only.trace"
    cmp "$out/reference-$mode.json" "$out/c0-$mode.json"
    cmp "$out/reference-$mode.json" "$out/rust-$mode.json"
    cmp "$out/reference-$mode.json" "$out/harmony-only-$mode.json"
    python3 - "$out/reference-$mode.json" <<'PY'
import json,sys
actual=json.load(open(sys.argv[1]))
expected=json.load(open('rust/fm1-engine/tests/harmony-native-golden.json'))[str(actual['mode'])]
assert all(actual.get(k)==v for k,v in expected.items()),actual
PY
    cat "$out/rust-$mode.json" | tee -a "$out/results.jsonl"
done
echo 'Frozen/fresh C and Rust harmonic buffers, callbacks, RNG, registers, graph/slot/arc transitions and mood match.'
