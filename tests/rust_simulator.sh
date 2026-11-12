#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/simulator
mkdir -p "$out/gen"
if [ "$baseline" != "$repo" ];then
    (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
python3 tools/gen_font.py "$out/gen/felucca_font.h" >/dev/null
common='-O2 -ffp-contract=off -std=c99 -DEN_HOST -D_POSIX_C_SOURCE=200809L -Wno-unused-function'
for flavor in reference c0;do
    source=$repo
    if [ "$flavor" = reference ];then source=$baseline;fi
    python3 tests/reference/simulator_oracle.py "$source" "$out/$flavor.c"
    cc $common -I"$out/gen" -I"$source/firmware/src" -I"$source/firmware/src/dsp" "$out/$flavor.c" "$source/firmware/src/dsp/amb.c" -lm -o "$out/$flavor"
done
FM1_SIM_SOURCE="$baseline" cargo build --locked --release -p fm1-sim --features c-app --target-dir "$out/frozen-target"
cp "$out/frozen-target/release/fm1-sim" "$out/frozen-runner"
cargo build --locked --release -p fm1-sim --features c-app --target-dir "$out/c0-target"
cp "$out/c0-target/release/fm1-sim" "$out/c0-runner"
cargo build --locked --release -p fm1-sim --features rust-engine
cp target/release/fm1-sim "$out/rust"
python3 tests/reference/simulator_compare.py "$out"
python3 tests/reference/simulator_repeat.py "$out"
