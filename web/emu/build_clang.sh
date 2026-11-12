#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# The emulator without Emscripten: plain clang (wasm32 target) and wasm-ld, with the few libc pieces
# the code uses supplied by web/emu/shim. Same output as web/emu/build.sh.
#   web/emu/build_clang.sh  ->  build/emu/{index.html, worklet.js, amb.wasm}
set -e
cd "$(dirname "$0")/../.."
mkdir -p build/gen build/emu
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
RUST_FLAGS=""
RUST_LIB=""
if [ "${EN_RUST_JAM:-1}" = 1 ]; then
    cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir build/rust-emu
    RUST_FLAGS="-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_VOICES=1"
    if [ "${EN_RUST_FM6:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FM6=1"; fi
    if [ "${EN_RUST_FACTORY:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FACTORY=1"; fi
    if [ "${EN_RUST_FORMATS:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FORMATS=1"; fi
    if [ "${EN_RUST_STORAGE:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_STORAGE=1"; fi
    RUST_LIB="build/rust-emu/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
fi
clang --target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w \
    -DEN_HOST -DEN_WEB $RUST_FLAGS "-DEN_VERSION=\"$(printf %s "${EN_VERSION:-DEV}" | tr a-z A-Z)\"" \
    -Iweb/emu/shim -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen \
    -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,-z,stack-size=1048576 \
    -o build/emu/amb.wasm web/emu/amb_web.c firmware/src/dsp/amb.c web/emu/shim/libc.c $RUST_LIB
sh web/emu/package.sh
echo "emu: build/emu ($(wc -c < build/emu/amb.wasm) B wasm)"
