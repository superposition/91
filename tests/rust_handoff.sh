#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Real Songbook receive/activation/readback and UI-copy paths in C and Rust.
set -eu
cd "$(dirname "$0")/.."
out=build/handoff
mkdir -p "$out" build/gen
[ -f build/gen/felucca_font.h ] || python3 tools/gen_font.py build/gen/felucca_font.h >/dev/null
cargo build --release --locked -p fm1-jam-bridge --target-dir build/rust-handoff
common='-O2 -ffp-contract=off -std=c99 -D_POSIX_C_SOURCE=200809L -Wno-unused-function -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen'
# The host compiler is separate from the WASM clang wrapper.
${CC:-cc} $common -o "$out/c" tests/host/jam_usb_test.c firmware/src/dsp/amb.c -lm
${CC:-cc} $common -DEN_RUST_JAM=1 -DEN_RUST_COMMANDS=1 -DEN_RUST_HANDOFF=1 -DEN_RUST_TRANSPORT=1 -DEN_RUST_SCHEDULER=1 -DEN_RUST_MELODIC=1 -DEN_RUST_EFFECTS=1 -DEN_RUST_FACTORY=1 \
    -o "$out/rust" tests/host/jam_usb_test.c firmware/src/dsp/amb.c \
    build/rust-handoff/release/libfm1_jam_bridge.a -lm -ldl -lpthread
env -u AMB_LEGACY_BOOT "$out/c" > "$out/c.log"
env -u AMB_LEGACY_BOOT "$out/rust" > "$out/rust.log"
# Hash every byte of the common protocol responses, not just decoded statuses.
test "$(head -n 1 "$out/c.log")" = "$(head -n 1 "$out/rust.log")"
cat "$out/c.log" "$out/rust.log"
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    python3 tests/reference/handoff_reference.py "$baseline" "$out/frozen.c"
    ${CC:-cc} $common -o "$out/frozen" "$out/frozen.c" "$baseline/firmware/src/dsp/amb.c" -lm
    env -u AMB_LEGACY_BOOT "$out/frozen" > "$out/frozen.log"
    test "$(head -n 1 "$out/frozen.log")" = "$(head -n 1 "$out/rust.log")"
    cat "$out/frozen.log"
fi
