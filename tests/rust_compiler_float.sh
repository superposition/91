#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Positive regression gate; the original compiler intentionally fails it.
# Usage: bash tests/rust_compiler_float.sh MRUSTC CORE_OUTPUT ZIG [OUTPUT]
set -eu
cd "$(dirname "$0")/.."
repo=$PWD
compiler=$(realpath "${1:?patched compiler binary}")
core=$(realpath "${2:?generated core output directory}")
zig=$(realpath "${3:?pinned Zig compiler}")
out=${4:-build/compiler-float-regression}
mkdir -p "$out"
out=$(realpath "$out")
export MRUSTC_TARGET_VER=1.90
native=(-target x86-linux-musl -msse2 -mfpmath=sse -ffp-contract=off -fexcess-precision=standard -O2 -std=gnu11 -w -ffunction-sections -fdata-sections)
# Natural i386 layout: no packing and no disabled assertions.
"$zig" cc "${native[@]}" -c "$core/libcore.rlib.c" -o "$out/core.o"
for probe in float_vectors remainder_constants float_comparisons;do
    source="tests/reference/compiler_$probe.rs"
    "$compiler" "$source" --crate-name "$probe" --crate-type rlib --edition 2021 \
        --target "$repo/rust/toolchain/pi32v2-mrustc.toml" -L "$core" --extern "core=$core/libcore.rlib" \
        -O -C panic=abort -o "$out/lib$probe.rlib" -C "emit-build-command=$out/$probe-command.txt" > "$out/$probe-emit.log" 2>&1
    "$zig" cc "${native[@]}" -Wl,--gc-sections -static "tests/reference/compiler_$probe.c" \
        "$out/lib$probe.rlib.c" "$out/core.o" -lm -o "$out/$probe-generated"
    "$out/$probe-generated" > "$out/$probe-generated.jsonl"
    rustc +1.94.1 "$source" --crate-type lib --emit=obj -C opt-level=2 -C panic=abort -o "$out/$probe-rustc.o"
    "${CC:-cc}" -ffp-contract=off "tests/reference/compiler_$probe.c" "$out/$probe-rustc.o" -lm -o "$out/$probe-rustc"
    "$out/$probe-rustc" > "$out/$probe-rustc.jsonl"
    cmp "$out/$probe-generated.jsonl" "$out/$probe-rustc.jsonl"
done
sha256sum "$compiler" "$core/libcore.rlib.c" "$out"/*.jsonl > "$out/identities.sha256"
echo 'Generated-C49 bit-pattern cases and28 numeric comparison predicates match rustc.'
