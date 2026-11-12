#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?independent frozen source path}
out=build/usb-queues
mkdir -p "$out"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
python3 tests/reference/usb_queue_reference.py "$baseline" "$out/frozen"
python3 tests/reference/usb_queue_reference.py "$repo" "$out/current"
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wlib="$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
common='-O2 -ffp-contract=off -ffunction-sections -fdata-sections -Wl,--gc-sections -std=gnu99'
cc $common -I"$out/frozen" -Ifirmware/src -Ifirmware/hal tests/reference/usb_queue_reference.c -lm -o "$out/reference"
cc $common -I"$out/current" -Ifirmware/src -Ifirmware/hal tests/reference/usb_queue_reference.c -lm -o "$out/c0"
"$out/reference" > "$out/reference.bin";"$out/c0" > "$out/c0.bin";cmp "$out/reference.bin" "$out/c0.bin"
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576'
clang $wasmflags -I"$out/current" -Ifirmware/src -Ifirmware/hal tests/reference/usb_queue_reference.c web/emu/shim/libc.c -o "$out/c0.wasm"
node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/c0.wasm" > "$out/c0.jsonl"
cat "$out/c0.jsonl"
for hal in 0 1;do
    cc $common -DEN_RUST_HAL=$hal -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src -Ifirmware/hal tests/reference/usb_queue_reference.c "$lib" -lpthread -ldl -lm -o "$out/rust"
    "$out/rust" > "$out/rust.bin";cmp "$out/reference.bin" "$out/rust.bin"
    clang $wasmflags -DEN_RUST_HAL=$hal -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src -Ifirmware/hal tests/reference/usb_queue_reference.c web/emu/shim/libc.c "$wlib" -o "$out/rust.wasm"
    node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/rust.wasm" > "$out/hal-$hal.jsonl"
    cat "$out/hal-$hal.jsonl"
done
