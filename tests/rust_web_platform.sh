#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/web-platform
mkdir -p "$out/gen"
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --features web --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -D_POSIX_C_SOURCE=200809L -I$out/gen -Ifirmware/src -Ifirmware/src/dsp -w"
cc $flags tests/reference/web_platform_reference.c firmware/src/dsp/amb.c -lm -o "$out/c0"
cc $flags -DEN_RUST_WEB=1 tests/reference/web_platform_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread -o "$out/rust"
"$out/c0" > "$out/c0.bin"
"$out/rust" > "$out/rust.bin"
cmp "$out/c0.bin" "$out/rust.bin"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --features web --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -Iweb/emu/shim -I$out/gen -Ifirmware/src -Ifirmware/src/dsp -w -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,--max-memory=33554432 -Wl,-z,stack-size=1048576"
clang $wasmflags tests/reference/web_platform_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c -o "$out/c0.wasm"
clang $wasmflags -DEN_RUST_WEB=1 tests/reference/web_platform_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a" -o "$out/rust.wasm"
node tests/host/project_wasm_test.mjs "$out/c0.bin" "$out/c0.wasm" "$out/rust.wasm"
sha256sum "$out/c0.bin" "$out/rust.bin"
