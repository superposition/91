#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/ui-busy
mkdir -p "$out/gen"
python3 tests/reference/ui_busy_host.py . "$out/ui_busy_host.c"
python3 tools/gen_font.py "$out/gen/felucca_font.h" >/dev/null
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
flags="-O2 -ffp-contract=off -std=gnu99 -DEN_HOST=1 -DEN_WEB=1 -D_POSIX_C_SOURCE=200809L -I$out -I$out/gen -Ifirmware/src -Ifirmware/src/dsp"
cc $flags tests/host/ui_store_busy_test.c firmware/src/dsp/amb.c -lm -o "$out/c0"
"$out/c0"
rustflags="-DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 -DEN_RUST_VOICES=1 -DEN_RUST_FM6=1 -DEN_RUST_FORMATS=1 -DEN_RUST_WORKER=1 -DEN_RUST_HARMONY=1 -DEN_RUST_STORAGE=1 -DEN_RUST_APP=1"
cc $flags $rustflags tests/host/ui_store_busy_test.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread -o "$out/rust"
"$out/rust"
python3 tests/host/ui_flush_pending_test.py
