#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/ui-state
mkdir -p "$out"
python3 tools/ui_abi.py --check
sha256sum -c tests/reference/ui-state.sha256
cargo +1.94.1 test --release --locked -p fm1-app
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -Ifirmware/src"
cc $flags -o "$out/frozen" tests/reference/ui_state_reference.c
cc $flags -DEN_RUST_APP=1 -o "$out/rust" tests/reference/ui_state_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/frozen" > "$out/frozen.bin"
"$out/rust" > "$out/rust.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Ifirmware/src -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/frozen.wasm" tests/reference/ui_state_reference.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_APP=1 -o "$out/rust.wasm" tests/reference/ui_state_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/rust.wasm"
sha256sum "$out/rust.bin"
