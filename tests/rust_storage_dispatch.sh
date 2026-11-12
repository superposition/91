#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/storage-dispatch
mkdir -p "$out/gen"
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
python3 tests/reference/storage_dispatch_reference.py "$out"
cc -O2 -ffp-contract=off -std=c99 -Wno-unused-function -D_POSIX_C_SOURCE=200809L -I"$out" -I"$out/gen" -Ifirmware/src -Ifirmware/src/app -Ifirmware/src/dsp "$out/test.c" firmware/src/dsp/amb.c -lm -o "$out/test"
"$out/test" > "$out/results.txt"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
cc -O2 -ffp-contract=off -std=c99 -Wno-unused-function -D_POSIX_C_SOURCE=200809L -DEN_RUST_FORMATS=1 -I"$out" -I"$out/gen" -Ifirmware/src -Ifirmware/src/app -Ifirmware/src/dsp "$out/test.c" firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread -o "$out/test-rust"
"$out/test-rust" > "$out/results-rust.txt"
cmp "$out/results.txt" "$out/results-rust.txt"
cat "$out/results.txt"
