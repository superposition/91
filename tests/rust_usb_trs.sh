#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/usb-trs
mkdir -p "$out"
if [ "$baseline" != "$repo" ];then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
common='-O2 -ffp-contract=off -ffunction-sections -fdata-sections -Wl,--gc-sections -std=gnu99'
cc $common -I"$baseline/firmware/src" tests/reference/usb_trs_reference.c -lm -o "$out/reference"
cc $common -Ifirmware/src tests/reference/usb_trs_reference.c -lm -o "$out/c0"
cc $common -DEN_RUST_USB=1 -Ifirmware/src tests/reference/usb_trs_reference.c "$lib" -lpthread -ldl -lm -o "$out/rust"
for impl in reference c0 rust;do "$out/$impl" > "$out/$impl.bin";done
cmp "$out/reference.bin" "$out/c0.bin";cmp "$out/reference.bin" "$out/rust.bin"
cc $common -Ifirmware/src tests/host/trs_test.c -lm -o "$out/legacy-c0"
cc $common -DEN_RUST_USB=1 -Ifirmware/src tests/host/trs_test.c "$lib" -lpthread -ldl -lm -o "$out/legacy-rust"
"$out/legacy-c0" > "$out/legacy-c0.txt";"$out/legacy-rust" > "$out/legacy-rust.txt"
cmp "$out/legacy-c0.txt" "$out/legacy-rust.txt";cat "$out/legacy-rust.txt"
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576'
clang $wasmflags -Ifirmware/src tests/reference/usb_trs_reference.c web/emu/shim/libc.c -o "$out/c0.wasm"
clang $wasmflags -DEN_RUST_USB=1 -Ifirmware/src tests/reference/usb_trs_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a" -o "$out/rust.wasm"
node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/c0.wasm" "$out/rust.wasm"
sha256sum "$out/reference.bin" "$out/c0.bin" "$out/rust.bin"
