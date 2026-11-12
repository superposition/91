#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Real compressor/limiter plus audible native render with ring/state/RNG traces.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/effects-parity
case "${2:-}" in
    extra-genres|impulses) out="$out-$2" ;;
    '') ;;
    *) echo 'unknown effects test group' >&2; exit 2 ;;
esac
mkdir -p "$out"
python3 tools/effects_abi.py --check
if [ "$baseline" != "$repo" ]; then
    (cd "$baseline"; sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" -o "$out/reference" tests/reference/effects_reference.c -lm
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp -o "$out/c0" tests/reference/effects_reference.c -lm
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/rust-build"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FORMATS=1 \
    -Ifirmware/src/dsp -o "$out/rust" tests/reference/effects_reference.c "$out/rust-build/release/libfm1_jam_bridge.a" -lpthread -ldl -lm
: > "$out/results.jsonl"
cases=${EFFECTS_CASES:-0 1 2 3 4 5 6 7 8 20 30 31}
if [ "${2:-}" = extra-genres ]; then cases='9 10 11 12 13 14 15 16 17 18 19'; fi
if [ "${2:-}" = impulses ]; then cases='20 30 31'; fi
for scenario in $cases; do
    "$out/reference" "$scenario" "$out/reference.trace" > "$out/reference-$scenario.json"
    "$out/c0" "$scenario" "$out/c0.trace" > "$out/c0-$scenario.json"
    "$out/rust" "$scenario" "$out/rust.trace" > "$out/rust-$scenario.json"
    cmp "$out/reference.trace" "$out/c0.trace"
    cmp "$out/reference.trace" "$out/rust.trace"
    cmp "$out/reference-$scenario.json" "$out/c0-$scenario.json"
    cmp "$out/reference-$scenario.json" "$out/rust-$scenario.json"
    python3 - "$out/reference-$scenario.json" rust/fm1-engine/tests/effects-native-golden.json <<'PY'
import json, sys
actual=json.load(open(sys.argv[1]))
golden=json.load(open(sys.argv[2])).get(str(actual['mode']))
if golden is not None and actual != golden:
    raise SystemExit(f"frozen FX fingerprint changed: {actual}")
PY
    cat "$out/rust-$scenario.json" | tee -a "$out/results.jsonl"
done
echo 'Native frozen-C, fresh-C0 and Rust FX PCM/USB/state/ring/RNG traces match.'
