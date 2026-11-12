#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/hal-parity
mkdir -p "$out"
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
python3 tools/hal_reference.py "$baseline" "$out/frozen-hal"
python3 tools/hal_reference.py "$repo" "$out/fresh-hal"
cargo build --release --locked -p fm1-jam-bridge --features hal-test --target-dir "$out/rust-build"
lib="$out/rust-build/release/libfm1_jam_bridge.a"
common='-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections'
g++ $common -I"$out/frozen-hal" tests/reference/hal_reference.cpp -o "$out/reference"
g++ $common -I"$out/fresh-hal" tests/reference/hal_reference.cpp -o "$out/c0"
g++ $common -DEN_RUST_HAL=1 -I"$out/fresh-hal" -Ifirmware/hal tests/reference/hal_reference.cpp "$lib" -lpthread -ldl -lm -o "$out/bridge"
for impl in reference c0 bridge;do "$out/$impl" "$out/$impl.trace" > "$out/$impl.jsonl";done
cargo run --release --locked -p fm1-hal --example hal_oracle --target-dir "$out/rust-build" -- "$out/rust.trace" > "$out/rust.jsonl"
for impl in c0 bridge rust;do cmp "$out/reference.trace" "$out/$impl.trace";cmp "$out/reference.jsonl" "$out/$impl.jsonl";done
cmp "$out/reference.jsonl" rust/fm1-hal/tests/registers-golden.jsonl
for impl in reference c0 bridge;do
    case "$impl" in reference) hdr="$out/frozen-hal";opts='';;c0) hdr="$out/fresh-hal";opts='';;bridge) hdr="$out/fresh-hal";opts='-DEN_RUST_HAL=1';;esac
    cc $common $opts -I"$hdr" -Ifirmware/hal tests/reference/hal_readable_reference.c "$lib" -lpthread -ldl -lm -o "$out/readable-$impl"
    "$out/readable-$impl" > "$out/readable-$impl.bin"
done
cmp "$out/readable-reference.bin" "$out/readable-c0.bin";cmp "$out/readable-reference.bin" "$out/readable-bridge.bin"
for core in 0 1;do
    cc $common -DEN_CORE2=$core -I"$out/frozen-hal" -Ifirmware/hal tests/reference/hal_boot_reference.c -o "$out/boot-reference-$core"
    cc $common -DEN_CORE2=$core -I"$out/fresh-hal" -Ifirmware/hal tests/reference/hal_boot_reference.c -o "$out/boot-c0-$core"
    cc $common -DEN_CORE2=$core -DEN_RUST_HAL=1 -I"$out/fresh-hal" -Ifirmware/hal tests/reference/hal_boot_reference.c "$lib" -lpthread -ldl -lm -o "$out/boot-rust-$core"
    for impl in reference c0 rust;do "$out/boot-$impl-$core" "$out/boot-$impl-$core.trace" > "$out/boot-$impl-$core.json";done
    for impl in c0 rust;do cmp "$out/boot-reference-$core.trace" "$out/boot-$impl-$core.trace";cmp "$out/boot-reference-$core.json" "$out/boot-$impl-$core.json";done
    "$out/boot-rust-$core" "$out/boot-unknown-$core.trace" unknown > "$out/boot-unknown-$core.json"
    cat "$out/boot-rust-$core.json" "$out/boot-unknown-$core.json"
done
cc -O2 -Ifirmware/hal tests/host/hal_irq_state_test.c -o "$out/irq"
"$out/irq"
g++ -O2 -I"$out/fresh-hal" -Ifirmware/hal tests/reference/hal_display_test.cpp -o "$out/display"
"$out/display"
cc -O2 -ffp-contract=off -std=c99 -Ifirmware/hal tests/host/hal_audio_fault_test.c "$lib" -lpthread -ldl -lm -o "$out/audio-fault"
"$out/audio-fault"
echo 'Frozen/fresh C and safe Rust/actual bridge320register traces, startup regions/noinit, saved IRQ and retained DMA fault contracts pass.'
