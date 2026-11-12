#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/ui-frame
mkdir -p "$out/gen"
sha256sum -c tests/reference/ui-frame.sha256
python3 tests/reference/ui_frame_extract.py . "$out/gen/ui_frame_port.inc" port
cp tests/reference/ui_frame_frozen.inc "$out/gen/ui_frame_frozen.inc"
cargo +1.94.1 fmt --all -- --check
cargo +1.94.1 test --release --locked -p fm1-app
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -DEN_HOST=1 -DEN_RUST_HANDOFF=1 -I$out/gen -Ifirmware/src"
cc $flags -o "$out/frozen" tests/reference/ui_frame_reference.c
cc $flags -DEN_RUST_APP=1 -o "$out/rust" tests/reference/ui_frame_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/frozen" >"$out/frozen.bin"
"$out/rust" >"$out/rust.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
if [ "$#" -gt 0 ];then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") >"$out/r01-verification.log"
    python3 tests/reference/ui_frame_extract.py "$baseline" "$out/gen/ui_frame_frozen.inc" frozen
    cmp tests/reference/ui_frame_frozen.inc "$out/gen/ui_frame_frozen.inc"
fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -DEN_HOST=1 -DEN_RUST_HANDOFF=1 -Iweb/emu/shim -Ifirmware/src -I$out/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/frozen.wasm" tests/reference/ui_frame_reference.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_APP=1 -o "$out/rust.wasm" tests/reference/ui_frame_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/rust.wasm"
sha256sum "$out/rust.bin"
