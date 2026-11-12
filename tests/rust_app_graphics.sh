#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/app-graphics
mkdir -p "$out/gen"
sha256sum -c tests/reference/app-graphics.sha256
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
python3 tests/reference/app_extract_alpha.py . "$out/gen/app_alpha.inc"
cargo +1.94.1 test --release --locked -p fm1-app
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=c99 -Ifirmware/src -I$out/gen"
cc $flags -o "$out/c0" tests/reference/app_graphics_reference.c
cc $flags -DEN_RUST_APP=1 -o "$out/rust" tests/reference/app_graphics_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
cc $flags "-DAPP_GFX_SOURCE=\"$repo/tests/reference/app_gfx_frozen.inc\"" "-DAPP_ALPHA_SOURCE=\"$repo/tests/reference/app_alpha_frozen.inc\"" -o "$out/frozen" tests/reference/app_graphics_reference.c
"$out/c0" > "$out/c0.bin"
"$out/rust" > "$out/rust.bin"
"$out/frozen" > "$out/frozen.bin"
cmp "$out/c0.bin" "$out/rust.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
cc $flags -o "$out/dma" tests/host/app_dma_test.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/dma"
if [ "$#" -gt 0 ];then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/r01-verification.log"
    mkdir -p "$out/frozen-gen"
    (cd "$baseline" && python3 tools/gen_font.py "$repo/$out/frozen-gen/felucca_font.h") > "$out/frozen-font.log"
    cmp "$out/gen/felucca_font.h" "$out/frozen-gen/felucca_font.h"
    python3 tests/reference/app_extract_alpha.py "$baseline" "$out/frozen-gen/app_alpha.inc"
    cc -O2 -std=c99 -I"$out/frozen-gen" "-DAPP_GFX_SOURCE=\"$baseline/firmware/src/gfx.c\"" -o "$out/r01" tests/reference/app_graphics_reference.c
    "$out/r01" > "$out/r01.bin"
    cmp "$out/r01.bin" "$out/rust.bin"
fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Ifirmware/src -I$out/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/c0.wasm" tests/reference/app_graphics_reference.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_APP=1 -o "$out/rust.wasm" tests/reference/app_graphics_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
clang $wasmflags "-DAPP_GFX_SOURCE=\"$repo/tests/reference/app_gfx_frozen.inc\"" "-DAPP_ALPHA_SOURCE=\"$repo/tests/reference/app_alpha_frozen.inc\"" -o "$out/frozen.wasm" tests/reference/app_graphics_reference.c web/emu/shim/libc.c
node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm"
sha256sum "$out/c0.bin" "$out/rust.bin"
