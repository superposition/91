#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/ui-buttons
mkdir -p "$out/gen"
sha256sum -c tests/reference/ui-buttons.sha256
python3 tests/reference/ui_frame_extract.py . "$out/gen/ui_button_current.inc" button_current
python3 tests/reference/ui_frame_extract.py . "$out/gen/ui_button_port.inc" button_port
cp tests/reference/ui_button_frozen.inc "$out/gen/ui_button_frozen.inc"
cargo +1.94.1 fmt --all -- --check
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 test --locked -p fm1-jam-bridge ui::tests::callbacks_reenter_ui_only_between_scoped_borrows
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -DEN_HOST=1 -I$out/gen -Ifirmware/src"
current='-DUI_BUTTON_SOURCE="ui_button_current.inc"'
cc $flags -o "$out/frozen" tests/reference/ui_buttons_reference.c
cc $flags "$current" -o "$out/c0" tests/reference/ui_buttons_reference.c
cc $flags "$current" -DEN_RUST_APP=1 -o "$out/rust" tests/reference/ui_buttons_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/frozen" >"$out/frozen.bin"
"$out/c0" >"$out/c0.bin"
"$out/rust" >"$out/rust.bin"
cmp "$out/frozen.bin" "$out/c0.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
if [ "$#" -gt 0 ];then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") >"$out/r01-verification.log"
    python3 tests/reference/ui_frame_extract.py "$baseline" "$out/gen/ui_button_frozen.inc" button_frozen
    cmp tests/reference/ui_button_frozen.inc "$out/gen/ui_button_frozen.inc"
fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -DEN_HOST=1 -Iweb/emu/shim -Ifirmware/src -I$out/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/frozen.wasm" tests/reference/ui_buttons_reference.c web/emu/shim/libc.c
clang $wasmflags "$current" -o "$out/c0.wasm" tests/reference/ui_buttons_reference.c web/emu/shim/libc.c
clang $wasmflags "$current" -DEN_RUST_APP=1 -o "$out/rust.wasm" tests/reference/ui_buttons_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm"
sha256sum "$out/rust.bin"
