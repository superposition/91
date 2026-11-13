#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?immutable R01 source is required for the independent full-app input corpus}
baseline=$(CDPATH= cd -- "$baseline" && pwd)
corpus=${UI_APP_CORPUS:-input}
case "$corpus" in input|performance|automatic);;*)echo 'unknown UI corpus' >&2;exit 2;;esac
out=build/ui-$corpus
driver=tests/reference/ui_${corpus}_reference.c
mkdir -p "$out/gen"
(cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") >"$out/r01-verification.log"
python3 tools/ui_input_abi.py --check
python3 tools/ui_performance_abi.py --check
python3 tests/reference/ui_input_host.py . "$out/ui_input_current.inc"
python3 tests/reference/ui_input_host.py "$baseline" "$out/ui_input_frozen.inc"
python3 tools/gen_font.py "$out/gen/felucca_font.h" >"$out/font.log"
cargo +1.94.1 fmt --all -- --check
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 test --locked -p fm1-jam-bridge ui::tests::
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -DEN_HOST=1 -DEN_WEB=1 -DEN_VERSION=\"0.3.0\" -D_POSIX_C_SOURCE=200809L -I$out -I$out/gen -Ifirmware/src -Ifirmware/src/dsp"
rustflags="-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 -DEN_RUST_HARMONY=1 -DEN_RUST_STORAGE=1 -DEN_RUST_APP=1"
cc $flags '-DUI_INPUT_HOST="ui_input_frozen.inc"' "$driver" "$baseline/firmware/src/dsp/amb.c" -lm -o "$out/frozen"
cc $flags "$driver" firmware/src/dsp/amb.c -lm -o "$out/c0"
cc $flags $rustflags "$driver" firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread -o "$out/rust"
"$out/frozen" >"$out/frozen.bin"
"$out/c0" >"$out/c0.bin"
"$out/rust" >"$out/rust.bin"
cmp "$out/frozen.bin" "$out/c0.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
sha256sum -c tests/reference/ui-$corpus.sha256
if [ "${UI_INPUT_NATIVE_ONLY:-0}" = 1 ];then exit 0;fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -DEN_HOST=1 -DEN_WEB=1 -DEN_VERSION=\"0.3.0\" -Iweb/emu/shim -Ifirmware/src -Ifirmware/src/dsp -I$out -I$out/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=33554432 -Wl,-z,stack-size=1048576"
clang $wasmflags '-DUI_INPUT_HOST="ui_input_frozen.inc"' "$driver" "$baseline/firmware/src/dsp/amb.c" web/emu/shim/libc.c -o "$out/frozen.wasm"
clang $wasmflags "$driver" firmware/src/dsp/amb.c web/emu/shim/libc.c -o "$out/c0.wasm"
clang $wasmflags $rustflags "$driver" firmware/src/dsp/amb.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a" -o "$out/rust.wasm"
if [ "$corpus" = performance ];then
    node tests/host/ui_performance_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm"
else
    node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm"
fi
