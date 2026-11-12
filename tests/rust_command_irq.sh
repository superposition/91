#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/command-irq
mkdir -p "$out"
python3 tests/reference/command_irq_reference.py "$out/reference.c"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --features pi32v2 --target-dir "$out/rust-build"
${CC:-cc} -O2 -std=c99 -o "$out/reference" "$out/reference.c" "$out/rust-build/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/reference"
