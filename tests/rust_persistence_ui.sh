#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/persistence
mkdir -p "$out" build/gen
# This historical session witness stores factory packs. Keep its original music
# while current JMP3 mode/Space persistence is covered by jam_transport_test.py.
baseline=${1:-"$repo/build/r01-source"}
if [ ! -f "$baseline/firmware/src/dsp/song_factory.inc" ]; then
    python3 tools/frozen_source.py "$baseline"
fi
baseline=$(CDPATH= cd -- "$baseline" && pwd)
(cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/ui-r01-verification.log"
fixture="$baseline/firmware/src/dsp/song_factory.inc"
python3 tests/reference/factory_fixture.py "$fixture" > "$out/ui-factory-fixture.json"
python3 tools/gen_font.py build/gen/felucca_font.h > "$out/ui-font.log"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags='-O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen'
cc $flags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -o "$out/ui-c0" tests/reference/persistence_ui_reference.c firmware/src/dsp/amb.c -lm
cc $flags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -DEN_RUST_STORAGE=1 -DEN_RUST_FORMATS=1 -o "$out/ui-rust" tests/reference/persistence_ui_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/ui-c0" > "$out/ui-c0.bin"
"$out/ui-rust" > "$out/ui-rust.bin"
cmp "$out/ui-c0.bin" "$out/ui-rust.bin"
grep '  build/persistence/ui-c0.bin$' tests/reference/persistence.sha256 | sha256sum -c -
(cd "$baseline" && cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$repo/build/gen" -o "$repo/$out/ui-frozen" "$repo/tests/reference/persistence_ui_reference.c" firmware/src/dsp/amb.c -lm)
"$out/ui-frozen" > "$out/ui-frozen.bin"
cmp "$out/ui-frozen.bin" "$out/ui-rust.bin"
sha256sum "$out/ui-c0.bin" "$out/ui-rust.bin"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags='--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -I. -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,-z,stack-size=1048576'
clang $wasmflags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -o "$out/ui-c0.wasm" tests/reference/persistence_ui_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c
clang $wasmflags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -DEN_RUST_STORAGE=1 -DEN_RUST_FORMATS=1 -o "$out/ui-rust.wasm" tests/reference/persistence_ui_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/project_wasm_test.mjs "$out/ui-c0.bin" "$out/ui-c0.wasm" "$out/ui-rust.wasm"
