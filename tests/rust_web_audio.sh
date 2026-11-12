#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$PWD
baseline=${1:-$repo}
out=build/web-audio
mkdir -p "$out/gen"
if [ "$baseline" != "$repo" ];then
    (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fi
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
cargo +1.94.1 test --release --locked -p fm1-app
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --features fm1-jam-bridge/web --all-targets -- -D warnings
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --features web --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -DEN_HOST -DEN_WEB -D_POSIX_C_SOURCE=200809L -I$out/gen -Ifirmware/src -Ifirmware/src/dsp"
cc $flags -o "$out/c0" tests/reference/web_audio_reference.c firmware/src/dsp/amb.c -lm
cc $flags -DEN_RUST_WEB=1 -o "$out/rust" tests/reference/web_audio_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
cc $flags "-DWEB_AUDIO_SOURCE=\"$baseline/web/emu/amb_web.c\"" -o "$out/frozen" tests/reference/web_audio_reference.c "$baseline/firmware/src/dsp/amb.c" -lm
for flavor in c0 rust frozen;do "$out/$flavor" > "$out/$flavor.bin";done
cmp "$out/c0.bin" "$out/rust.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
sha256sum "$out"/*.bin
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --features web --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -DEN_HOST -DEN_WEB -Iweb/emu/shim -Ifirmware/src -Ifirmware/src/dsp -I$out/gen -Wl,--no-entry -Wl,--export=main -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,--max-memory=33554432 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/c0.wasm" tests/reference/web_audio_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_WEB=1 -o "$out/rust.wasm" tests/reference/web_audio_reference.c firmware/src/dsp/amb.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
clang $wasmflags "-DWEB_AUDIO_SOURCE=\"$baseline/web/emu/amb_web.c\"" -o "$out/frozen.wasm" tests/reference/web_audio_reference.c "$baseline/firmware/src/dsp/amb.c" web/emu/shim/libc.c
node tests/host/web_audio_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm" > "$out/wasm-results.jsonl"
cat "$out/wasm-results.jsonl"
echo '4096 ring/output cases and all 65536 RGB565 background colors match frozen/current C on native and WASM.'
