#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/runner-policy
mkdir -p "$out/gen"
sha256sum -c tests/reference/runner-policy.sha256
python3 tools/gen_font.py "$out/gen/felucca_font.h" >"$out/font.log"
python3 tests/reference/runner_widgets_extract.py . "$out/gen/runner_widgets.inc"
cargo +1.94.1 test --release --locked -p fm1-app
cargo +1.94.1 clippy --locked -p fm1-app -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=c99 -Ifirmware/src -I$out/gen"
cc $flags -o "$out/c0" tests/reference/runner_policy_reference.c -lm
cc $flags -DEN_RUST_APP=1 -o "$out/rust" tests/reference/runner_policy_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
cc $flags "-DAPP_RUNNER_SOURCE=\"$repo/tests/reference/runner_frozen.inc\"" -o "$out/frozen" tests/reference/runner_policy_reference.c -lm
"$out/c0" >"$out/c0.bin"
"$out/rust" >"$out/rust.bin"
"$out/frozen" >"$out/frozen.bin"
cmp "$out/c0.bin" "$out/rust.bin"
cmp "$out/frozen.bin" "$out/rust.bin"
if [ "$#" -gt 0 ];then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") >"$out/r01-verification.log"
    cc $flags "-DAPP_RUNNER_SOURCE=\"$baseline/firmware/src/app/runner.c\"" -o "$out/r01" tests/reference/runner_policy_reference.c -lm
    "$out/r01" >"$out/r01.bin"
    cmp "$out/r01.bin" "$out/rust.bin"
fi
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown --target-dir "$out/wasm"
wasmflags="--target=wasm32 -O2 -ffp-contract=off -std=gnu99 -nostdlib -ffreestanding -w -Iweb/emu/shim -Ifirmware/src -I$out/gen -Wl,--no-entry -Wl,--export-memory -Wl,--initial-memory=8388608 -Wl,-z,stack-size=1048576"
clang $wasmflags -o "$out/c0.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c
clang $wasmflags -DEN_RUST_APP=1 -o "$out/rust.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
clang $wasmflags "-DAPP_RUNNER_SOURCE=\"$repo/tests/reference/runner_frozen.inc\"" -o "$out/frozen.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c
node tests/host/project_wasm_test.mjs "$out/frozen.bin" "$out/frozen.wasm" "$out/c0.wasm" "$out/rust.wasm" > "$out/wasm-results.jsonl"
sha256sum "$out/rust.bin"
drawflags="$flags -DRUNNER_DRAW_TEST=1"
cc $drawflags -o "$out/draw-c0" tests/reference/runner_policy_reference.c -lm
cc $drawflags -DEN_RUST_APP=1 -o "$out/draw-rust" tests/reference/runner_policy_reference.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
frozenrunner="-DAPP_RUNNER_SOURCE=\"$repo/tests/reference/runner_frozen.inc\""
frozenwidgets="-DAPP_RUNNER_WIDGETS=\"$repo/tests/reference/runner_widgets_frozen.inc\""
cc $drawflags "$frozenrunner" "$frozenwidgets" -o "$out/draw-frozen" tests/reference/runner_policy_reference.c -lm
"$out/draw-c0" >"$out/draw-c0.bin"
"$out/draw-rust" >"$out/draw-rust.bin"
"$out/draw-frozen" >"$out/draw-frozen.bin"
cmp "$out/draw-frozen.bin" "$out/draw-rust.bin"
cmp "$out/draw-c0.bin" "$out/draw-rust.bin"
clang $wasmflags -DRUNNER_DRAW_TEST=1 -o "$out/draw-c0.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c
clang $wasmflags -DRUNNER_DRAW_TEST=1 -DEN_RUST_APP=1 -o "$out/draw-rust.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c "$out/wasm/wasm32-unknown-unknown/release/libfm1_jam_bridge.a"
clang $wasmflags -DRUNNER_DRAW_TEST=1 "$frozenrunner" "$frozenwidgets" -o "$out/draw-frozen.wasm" tests/reference/runner_policy_reference.c web/emu/shim/libc.c
node tests/host/project_wasm_test.mjs "$out/draw-frozen.bin" "$out/draw-frozen.wasm" "$out/draw-c0.wasm" "$out/draw-rust.wasm" >> "$out/wasm-results.jsonl"
cat "$out/wasm-results.jsonl"
for profile in 0 1;do
    cc $flags -DEN_RUST_APP=$profile -o "$out/dma-$profile" tests/host/runner_dma_test.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
    "$out/dma-$profile"
done
