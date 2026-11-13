#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-}
if [ -z "$baseline" ]; then
    python3 tools/frozen_source.py build/r01-source
    baseline="$repo/build/r01-source"
fi
baseline=$(cd "$baseline" && pwd)
if [ "$baseline" = "$repo" ]; then
    echo "rust_simulator: use the verified frozen reference, not the current source tree" >&2
    exit 2
fi
out=build/simulator
mkdir -p "$out/gen"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fixture="$baseline/firmware/src/dsp/song_factory.inc"
python3 tests/reference/factory_fixture.py "$fixture" > "$out/factory-fixture.json"
unset FM1_TEST_FACTORY_HEADER FM1_SIM_SOURCE
python3 tests/reference/build_inputs.py > "$out/build-inputs.json"
python3 tools/gen_font.py "$out/gen/felucca_font.h" >/dev/null
common='-O2 -ffp-contract=off -std=c99 -DEN_HOST -D_POSIX_C_SOURCE=200809L -Wno-unused-function'
for flavor in reference c0;do
    source=$repo
    if [ "$flavor" = reference ];then source=$baseline;fi
    python3 tests/reference/simulator_oracle.py "$source" "$out/$flavor.c"
    set --
    if [ "$flavor" = c0 ]; then set -- "-DEN_TEST_FACTORY_HEADER=\"$fixture\""; fi
    cc $common "$@" -I"$out/gen" -I"$source/firmware/src" -I"$source/firmware/src/dsp" "$out/$flavor.c" "$source/firmware/src/dsp/amb.c" -lm -o "$out/$flavor"
done
FM1_SIM_SOURCE="$baseline" cargo build --locked --release -p fm1-sim --features c-app --target-dir "$out/frozen-target"
cp "$out/frozen-target/release/fm1-sim" "$out/frozen-runner"
FM1_TEST_FACTORY_HEADER="$fixture" cargo build --locked --release -p fm1-sim --features c-app --target-dir "$out/c0-target"
cp "$out/c0-target/release/fm1-sim" "$out/c0-runner"
FM1_TEST_FACTORY_HEADER="$fixture" cargo build --locked --release -p fm1-sim --features rust-engine
cp target/release/fm1-sim "$out/rust"
python3 tests/reference/build_inputs.py --check "$out/build-inputs.json" > "$out/legacy-build-verification.json"
python3 tests/reference/simulator_compare.py "$out"
python3 tests/reference/simulator_repeat.py "$out"
# Current factory music has separate, strict C/Rust cases. It is never compared
# against a historical-song fingerprint or substituted into the frozen corpus.
cc $common -I"$out/gen" -Ifirmware/src -Ifirmware/src/dsp "$out/c0.c" firmware/src/dsp/amb.c -lm -o "$out/current-c0"
cargo build --locked --release -p fm1-sim --features c-app --target-dir "$out/c0-target"
cp "$out/c0-target/release/fm1-sim" "$out/current-c0-runner"
cargo build --locked --release -p fm1-sim --features rust-engine
cp target/release/fm1-sim "$out/current-rust"
python3 tests/reference/build_inputs.py --check "$out/build-inputs.json" > "$out/current-build-verification.json"
python3 tests/reference/simulator_compare.py "$out" --current
