#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/hal-audio
mkdir -p "$out"
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --features hal-test --target-dir "$out/rust-build"
lib="$out/rust-build/release/libfm1_jam_bridge.a"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/hal -I"$baseline/firmware/src/dsp" tests/reference/hal_audio_reference.c -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/hal -Ifirmware/src/dsp tests/reference/hal_audio_reference.c -lm -o "$out/c0"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_HAL=1 -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 -DEN_RUST_HARMONY=1 -Ifirmware/hal -Ifirmware/src/dsp tests/reference/hal_audio_reference.c "$lib" -lpthread -ldl -lm -o "$out/rust"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_RUST_HAL=1 -Ifirmware/hal -Ifirmware/src/dsp tests/reference/hal_audio_reference.c "$lib" -lpthread -ldl -lm -o "$out/c-handoff"
: > "$out/results.jsonl"
for words in 512 1024;do
for scenario in ${HAL_AUDIO_CASES:-3 8};do
    for impl in reference c0 rust c-handoff;do "$out/$impl" "$scenario" "$out/$impl.trace" "$words" > "$out/$impl.json";done
    for impl in c0 rust c-handoff;do cmp "$out/reference.trace" "$out/$impl.trace";cmp "$out/reference.json" "$out/$impl.json";done
    printf 'half_words=%s ' "$words" | tee -a "$out/results.jsonl"
    cat "$out/rust.json" | tee -a "$out/results.jsonl"
done
done
echo 'Registered raw DMA conversion, gap then hush and USB plus full instrument PCM/rings/RNG match frozen/fresh C; C-only DSP handoff also matches.'
