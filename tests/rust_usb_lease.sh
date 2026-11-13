#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?independent frozen source path}
out=build/usb-lease
if [ "${2:-correction}" = healthy ];then out=build/usb-lease-healthy;fi
mkdir -p "$out"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
python3 tools/usb_reference.py "$baseline" "$out/frozen"
python3 tools/usb_reference.py "$repo" "$out/current"
cargo build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wlib="$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
extra=
if [ "${2:-correction}" = healthy ];then extra=-DUSB_LEASE_HEALTHY=1;fi
common='-O2 -DNDEBUG -ffp-contract=off -ffunction-sections -fdata-sections -Wl,--gc-sections -std=gnu99 -DPERF_USB_HOST=1 -DFELUCCA_OTA=1 -DFELUCCA_CDC=1 -DFELUCCA_UAC=1'
cc $common $extra -I"$out/frozen" -I"$baseline/firmware/src" tests/reference/usb_lease_reference.c -lm -o "$out/reference"
cc $common $extra -I"$out/current" -Ifirmware/src tests/reference/usb_lease_reference.c -lm -o "$out/c0"
cc $common $extra -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src tests/reference/usb_lease_reference.c "$lib" -lpthread -ldl -lm -o "$out/rust"
for impl in reference c0 rust;do "$out/$impl" > "$out/$impl.bin";done
cmp "$out/reference.bin" "$out/c0.bin"
if [ "${2:-correction}" = healthy ];then cmp "$out/reference.bin" "$out/rust.bin";fi
wasmflags='--target=wasm32 -O2 -DNDEBUG -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576 -DPERF_USB_HOST=1 -DFELUCCA_OTA=1 -DFELUCCA_CDC=1 -DFELUCCA_UAC=1'
clang $wasmflags $extra -I"$out/current" -Ifirmware/src tests/reference/usb_lease_reference.c web/emu/shim/libc.c -o "$out/c0.wasm"
clang $wasmflags $extra -DEN_RUST_USB=1 -I"$out/current" -Ifirmware/src tests/reference/usb_lease_reference.c web/emu/shim/libc.c "$wlib" -o "$out/rust.wasm"
node tests/host/project_wasm_test.mjs "$out/reference.bin" "$out/c0.wasm" > "$out/c0.jsonl"
node tests/host/project_wasm_test.mjs "$out/rust.bin" "$out/rust.wasm" > "$out/rust.jsonl"
cat "$out/c0.jsonl" "$out/rust.jsonl"
