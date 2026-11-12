#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Reuse pinned mrustc/core bootstrap, emit a fresh set of PI32v2 probe objects.
# Usage: emit_probe.sh TOOLCHAIN_PROOF_DIRECTORY FRESH_OUTPUT_DIRECTORY SOURCE_REVISION
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
proof=$(CDPATH= cd -- "${1:?toolchain proof directory}" && pwd)
out=${2:?fresh absolute output directory}
revision=${3:?source revision including local patch provenance}
test ! -e "$out"
mkdir -p "$out"
cd "$proof"
export MRUSTC_TARGET_VER=1.90
printf '%s\n' "$revision" > "$out/source-revision.txt"
target="$repo/rust/toolchain/pi32v2-mrustc.toml"
for package in types formats engine; do
    args=()
    if [ "$package" != types ]; then args+=(--extern "fm1_types=$out/libfm1_types.rlib"); fi
    if [ "$package" = engine ]; then args+=(--extern "fm1_formats=$out/libfm1_formats.rlib"); fi
    mrustc/bin/mrustc "$repo/rust/fm1-$package/src/lib.rs" --crate-name "fm1_$package" --crate-type rlib --edition 2021 \
        --target "$target" -L "$out" -L output-pi32v2 --extern core=output-pi32v2/libcore.rlib "${args[@]}" \
        -O -C panic=abort -o "$out/libfm1_$package.rlib" -C "emit-build-command=$out/$package-command.txt" \
        > "$out/$package.log" 2>&1
done
mrustc/bin/mrustc "$repo/rust/toolchain/probe.rs" --crate-name fm1_probe --crate-type rlib --edition 2021 \
    --target "$target" -L "$out" -L output-pi32v2 --extern core=output-pi32v2/libcore.rlib \
    --extern "fm1_types=$out/libfm1_types.rlib" --extern "fm1_formats=$out/libfm1_formats.rlib" --extern "fm1_engine=$out/libfm1_engine.rlib" \
    -O -C panic=abort -o "$out/libfm1_probe.rlib" -C "emit-build-command=$out/probe-command.txt" > "$out/probe.log" 2>&1
tc=${JIELI_TOOLCHAIN:?set the vendor toolchain root}
freestanding=${FM1_FREESTANDING:?set the freestanding C include directory}
for package in types formats engine probe; do
    "$tc/pi32v2/bin/clang" -target pi32v2 -mcpu=r3 -mfprev1 -ffp-contract=off -O2 \
        -ffunction-sections -fdata-sections -std=gnu11 -I"$freestanding" \
        -c "$out/libfm1_$package.rlib.c" -o "$out/libfm1_$package.rlib.o" > "$out/$package-c.log" 2>&1
done
