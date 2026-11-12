#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/hal-store
mkdir -p "$out" build/gen
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
python3 tools/gen_font.py build/gen/felucca_font.h > "$out/font.log"
cargo build --release --locked -p fm1-jam-bridge --features hal --target-dir "$out/rust-build"
lib="$out/rust-build/release/libfm1_jam_bridge.a"
common='-O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -Ifirmware/hal -Ibuild/gen'
cc $common tests/host/hal_flash_idle_test.c -o "$out/idle"
"$out/idle"
cc $common tests/host/hal_store_busy_test.c -o "$out/busy-c0"
cc $common -DEN_RUST_STORAGE=1 tests/host/hal_store_busy_test.c "$lib" -lpthread -ldl -lm -o "$out/busy-rust"
cc $common tests/host/hal_project_busy_test.c firmware/src/dsp/amb.c -lm -o "$out/project-c0"
cc $common -DEN_RUST_STORAGE=1 -DEN_RUST_FORMATS=1 tests/host/hal_project_busy_test.c firmware/src/dsp/amb.c "$lib" -lpthread -ldl -lm -o "$out/project-rust"
for impl in busy-c0 busy-rust project-c0 project-rust;do "$out/$impl";done
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/effects_reference.c -lm -o "$out/reference"
cc $common -DEN_HOST tests/reference/effects_reference.c -lm -o "$out/c0"
cc $common -DEN_HOST -DEN_RUST_HAL=1 -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 tests/reference/hal_flash_workers.c "$lib" -lpthread -ldl -lm -o "$out/workers"
cc $common -DEN_HOST -DEN_WORKER_TEST=1 -DEN_RUST_HAL=1 -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 tests/host/hal_flash_quarantine_test.c "$lib" -lpthread -ldl -lm -o "$out/quarantine"
"$out/quarantine"
for mode in ${HAL_FLASH_CASES:-3 8};do
    for impl in reference c0 workers;do "$out/$impl" "$mode" "$out/$impl.trace" > "$out/$impl-$mode.json";done
    cmp "$out/reference.trace" "$out/c0.trace";cmp "$out/reference.trace" "$out/workers.trace"
    cmp "$out/reference-$mode.json" "$out/c0-$mode.json";cmp "$out/reference-$mode.json" "$out/workers-$mode.json"
    cat "$out/workers-$mode.json"
done
echo 'Transient Busy and nested whole-worker flash admission preserve frozen PCM/state and all saved data.'
