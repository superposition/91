#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/effects-completion
mkdir -p "$out"
python3 tools/effects_abi.py --check
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/effects_completion_reference.c -lpthread -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/effects_completion_reference.c -lpthread -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 -DEN_RUST_HARMONY=1 -Ifirmware/src/dsp tests/reference/effects_completion_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm -o "$out/rust"
: > "$out/results.jsonl"
for mode in 0 1 2 3 4;do
    for impl in reference c0 rust;do "$out/$impl" "$mode" "$out/$impl.trace" > "$out/$impl-$mode.json";done
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference-$mode.json" "$out/c0-$mode.json"
    cmp "$out/reference-$mode.json" "$out/rust-$mode.json"
    python3 - "$out/reference-$mode.json" <<'PY'
import json,sys
actual=json.load(open(sys.argv[1]))
golden=json.load(open('rust/fm1-engine/tests/effects-completion-golden.json'))
assert actual==golden[str(actual['mode'])],actual
PY
    cat "$out/rust-$mode.json" | tee -a "$out/results.jsonl"
done
echo 'Frozen/fresh C and Rust settings, grid, integer fades/bounds and partial resets match.'
