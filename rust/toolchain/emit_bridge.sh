#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Usage: emit_bridge.sh MRUSTC_CHECKOUT CORE_OUTPUT FRESH_OUTPUT SOURCE_REVISION [plain|hal]
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
compiler=$(CDPATH= cd -- "${1:?pinned and patched mrustc checkout}" && pwd)
core=$(CDPATH= cd -- "${2:?core bootstrap output}" && pwd)
out=${3:?fresh absolute output directory}
revision=${4:?source revision and patch provenance}
mode=${5:-plain}
case "$mode" in plain) features=pi32v2;;hal) features=pi32v2,hal;;*) echo 'Mode must be plain or hal' >&2;exit 1;;esac
case "$out" in /*) ;; *) echo 'Output must be absolute' >&2; exit 1;; esac
test ! -e "$out"
tc=${JIELI_TOOLCHAIN:?vendor toolchain root}
python3 "$repo/rust/toolchain/verify_mrustc.py" "$compiler"
test -s "$core/libcore.rlib.hir"
test -s "$core/libcore.lto.o"
cargo +1.94.1 check --release --locked --manifest-path "$repo/Cargo.toml" \
    -p fm1-jam-bridge --no-default-features --features "$features" --target wasm32-unknown-unknown
mkdir -p "$out"
printf '%s\n' "$revision" > "$out/source-revision.txt"
printf '%s\n' "$mode" > "$out/hal-mode.txt"
export MRUSTC_TARGET_VER=1.90
for package in hal types formats engine rt app jam-bridge; do
    crate="fm1_${package//-/_}"
    args=()
    if [ "$package" != types ] && [ "$package" != hal ]; then args+=(--extern "fm1_types=$out/libfm1_types.rlib"); fi
    if [ "$package" = engine ] || [ "$package" = app ] || [ "$package" = jam-bridge ]; then args+=(--extern "fm1_formats=$out/libfm1_formats.rlib"); fi
    if [ "$package" = app ] || [ "$package" = jam-bridge ]; then args+=(--extern "fm1_engine=$out/libfm1_engine.rlib"); fi
    if [ "$package" = jam-bridge ]; then
        args+=(--extern "fm1_rt=$out/libfm1_rt.rlib" --extern "fm1_app=$out/libfm1_app.rlib" --extern "fm1_hal=$out/libfm1_hal.rlib")
        if [ "$mode" = hal ];then args+=(--cfg 'feature="hal"');fi
    fi
    if [ "$package" = rt ] || [ "$package" = jam-bridge ]; then args+=(--cfg 'feature="pi32v2"'); fi
    "$compiler/bin/mrustc" "$repo/rust/fm1-$package/src/lib.rs" --crate-name "$crate" --crate-type rlib --edition 2021 \
        --target "$repo/rust/toolchain/pi32v2-mrustc.toml" -L "$out" -L "$core" --extern "core=$core/libcore.rlib" "${args[@]}" \
        -O -C panic=abort -o "$out/lib$crate.rlib" -C "emit-build-command=$out/$package-command.txt" > "$out/$package.log" 2>&1
    "$tc/pi32v2/bin/clang" -target pi32v2 -mcpu=r3 -mfprev1 -ffp-contract=off -O2 -flto \
        -ffunction-sections -fdata-sections -std=gnu11 -I"$repo/rust/toolchain/freestanding" \
        -c "$out/lib$crate.rlib.c" -o "$out/lib$crate.rlib.o" > "$out/$package-c.log" 2>&1
done
sha256sum "$out"/*.rlib.hir "$out"/*.rlib.c "$out"/*.rlib.o > "$out/bridge.sha256"
printf 'All seven Rust libraries emitted and vendor LTO compiled (%s): %s\n' "$mode" "$out"
