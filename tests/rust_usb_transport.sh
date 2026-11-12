#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?independent frozen source path}
mode=${2:-full}
case "$mode" in
    full) out=build/usb-transport;fixture=tests/reference/usb_transport_reference.c;;
    edge) out=build/usb-transport-edge;fixture=tests/reference/usb_transport_edge_reference.c;;
    *) echo 'expected full or edge fixture' >&2;exit 2;;
esac
mkdir -p "$out"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
# The sole intentional correction has its own unmodified-frozen witness test.
python3 tools/usb_reference.py "$baseline" "$out/frozen" --preserve-ready-length
python3 tools/usb_reference.py "$repo" "$out/current"
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wlib="$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
common='-O2 -ffp-contract=off -ffunction-sections -fdata-sections -Wl,--gc-sections -std=gnu99'
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576'
for dx in 0 1;do
for ota in 0 1;do
    flags="-DFELUCCA_CDC=1 -DFELUCCA_UAC=1 -DUSB_DX=$dx -DFELUCCA_OTA=$ota"
    name="$dx-$ota"
    cc $common $flags -I"$out/frozen" -I"$baseline/firmware/src" "$fixture" -lm -o "$out/reference"
    cc $common $flags -I"$out/current" -Ifirmware/src "$fixture" -lm -o "$out/c0"
    cc $common $flags -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src "$fixture" "$lib" -lpthread -ldl -lm -o "$out/rust"
    for impl in reference c0 rust;do "$out/$impl" > "$out/$impl.bin";done
    cmp "$out/reference.bin" "$out/c0.bin";cmp "$out/reference.bin" "$out/rust.bin"
    clang $wasmflags $flags -I"$out/current" -Ifirmware/src "$fixture" web/emu/shim/libc.c -o "$out/c0.wasm"
    clang $wasmflags $flags -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src "$fixture" web/emu/shim/libc.c "$wlib" -o "$out/rust.wasm"
    node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/c0.wasm" "$out/rust.wasm" > "$out/$name.jsonl"
    cat "$out/$name.jsonl"
done
done
