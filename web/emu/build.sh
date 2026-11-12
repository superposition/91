#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# AMB-1 in the browser: the host simulator compiled to WebAssembly (Emscripten), plus its page.
#   [EN_VERSION=0.1] web/emu/build.sh  ->  build/emu/{index.html, worklet.js, amb.wasm}
# Same sources and flags as host/build_host.sh (-ffp-contract=off, like the device).
set -e
cd "$(dirname "$0")/../.."
mkdir -p build/gen build/emu
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
RUST_FLAGS=""
RUST_LIB=""
if [ "${EN_RUST_JAM:-1}" = 1 ]; then
    cargo build --release --locked -p fm1-jam-bridge --no-default-features --features web --target wasm32-unknown-unknown --target-dir build/rust-emu
    RUST_FLAGS="-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_VOICES=1"
    if [ "${EN_RUST_HARMONY:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_HARMONY=1"; fi
    if [ "${EN_RUST_WORKER:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_WORKER=1"; fi
    if [ "${EN_RUST_FM6:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FM6=1"; fi
    if [ "${EN_RUST_FACTORY:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FACTORY=1"; fi
    if [ "${EN_RUST_FORMATS:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_FORMATS=1"; fi
    if [ "${EN_RUST_STORAGE:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_STORAGE=1"; fi
    if [ "${EN_RUST_APP:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_APP=1"; fi
    if [ "${EN_RUST_WEB:-1}" = 1 ]; then RUST_FLAGS="$RUST_FLAGS -DEN_RUST_WEB=1"; fi
    RUST_LIB="build/rust-emu/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
fi
emcc -O2 -ffp-contract=off -std=gnu99 -Wall -Wno-unused-function -Wno-unused-parameter -Wno-unused-variable \
    -DEN_HOST -DEN_WEB $RUST_FLAGS "-DEN_VERSION=\"$(printf %s "${EN_VERSION:-DEV}" | tr a-z A-Z)\"" -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen \
    --no-entry -sSTANDALONE_WASM -sSTACK_SIZE=1048576 -sINITIAL_MEMORY=33554432 -sMAXIMUM_MEMORY=33554432 -sALLOW_MEMORY_GROWTH=0 -sFILESYSTEM=0 \
    -o build/emu/amb.wasm web/emu/amb_web.c firmware/src/dsp/amb.c $RUST_LIB
sh web/emu/package.sh
echo "emu: build/emu ($(wc -c < build/emu/amb.wasm) B wasm)"
