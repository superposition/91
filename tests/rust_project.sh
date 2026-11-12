#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/persistence
mkdir -p "$out" build/gen
python3 tools/gen_font.py build/gen/felucca_font.h > "$out/project-font.log"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags='-O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen'
cc $flags -o "$out/project-c0" tests/reference/project_reference.c firmware/src/dsp/amb.c -lm
cc $flags -DEN_RUST_STORAGE=1 -DEN_RUST_FORMATS=1 -o "$out/project-rust" tests/reference/project_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/project-c0" > "$out/project-c0.bin"
"$out/project-rust" > "$out/project-rust.bin"
cmp "$out/project-c0.bin" "$out/project-rust.bin"
grep '  build/persistence/project-c0.bin$' tests/reference/persistence.sha256 | sha256sum -c -
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/project-r01-verification.log"
    (cd "$baseline" && cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$repo/build/gen" -o "$repo/$out/project-frozen" "$repo/tests/reference/project_reference.c" firmware/src/dsp/amb.c -lm)
    "$out/project-frozen" > "$out/project-frozen.bin"
    cmp "$out/project-frozen.bin" "$out/project-rust.bin"
fi
sha256sum "$out/project-c0.bin" "$out/project-rust.bin"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -I. -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,-z,stack-size=1048576'
clang $wasmflags -o "$out/project-c0.wasm" tests/reference/project_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_STORAGE=1 -DEN_RUST_FORMATS=1 -o "$out/project-rust.wasm" tests/reference/project_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/project_wasm_test.mjs "$out/project-c0.bin" "$out/project-c0.wasm" "$out/project-rust.wasm"
