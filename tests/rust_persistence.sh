#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/persistence
mkdir -p "$out"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags='-O2 -std=c99 -Ifirmware/src'
cc $flags -o "$out/c0" tests/reference/persistence_reference.c
cc $flags -DEN_RUST_STORAGE=1 -o "$out/rust" tests/reference/persistence_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/c0" > "$out/c0.bin"
"$out/rust" > "$out/rust.bin"
cmp "$out/c0.bin" "$out/rust.bin"
grep '  build/persistence/c0.bin$' tests/reference/persistence.sha256 | sha256sum -c -
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/r01-verification.log"
    cc $flags "-DSTORAGE_SOURCE=\"$baseline/firmware/src/storage.c\"" -o "$out/frozen" tests/reference/persistence_reference.c
    "$out/frozen" > "$out/frozen.bin"
    cmp "$out/frozen.bin" "$out/rust.bin"
fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Ifirmware/src -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576'
clang $wasmflags -o "$out/c0.wasm" tests/reference/persistence_reference.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_STORAGE=1 -o "$out/rust.wasm" tests/reference/persistence_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/persistence_wasm_test.mjs "$out/c0.wasm" "$out/rust.wasm"
sha256sum "$out/c0.bin" "$out/rust.bin"
