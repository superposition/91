#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?independent frozen source path}
out=build/usb-tx
mkdir -p "$out"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
python3 tools/usb_reference.py "$baseline" "$out/frozen"
python3 tools/usb_reference.py "$repo" "$out/current"
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wlib="$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
common='-O2 -ffp-contract=off -ffunction-sections -fdata-sections -Wl,--gc-sections -std=gnu99'
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576'
for ota in 0 1;do
for loader in 0 1;do
    flags="-DFELUCCA_CDC=1 -DFELUCCA_UAC=1 -DFELUCCA_OTA=$ota"
    if [ "$loader" = 1 ];then flags="$flags -DFELUCCA_LOADER=1 -DFELUCCA_USB_PID=2";fi
    name="$ota-$loader"
    cc $common $flags -I"$out/frozen" -I"$baseline/firmware/src" tests/reference/usb_tx_reference.c -lm -o "$out/reference"
    cc $common $flags -I"$out/current" -Ifirmware/src tests/reference/usb_tx_reference.c -lm -o "$out/c0"
    cc $common $flags -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src tests/reference/usb_tx_reference.c "$lib" -lpthread -ldl -lm -o "$out/rust"
    for impl in reference c0 rust;do "$out/$impl" > "$out/$impl.bin";done
    cmp "$out/reference.bin" "$out/c0.bin";cmp "$out/reference.bin" "$out/rust.bin"
    clang $wasmflags $flags -I"$out/current" -Ifirmware/src tests/reference/usb_tx_reference.c web/emu/shim/libc.c -o "$out/c0.wasm"
    clang $wasmflags $flags -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src tests/reference/usb_tx_reference.c web/emu/shim/libc.c "$wlib" -o "$out/rust.wasm"
    node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/c0.wasm" "$out/rust.wasm" > "$out/$name.jsonl"
    cat "$out/$name.jsonl"
done
done
