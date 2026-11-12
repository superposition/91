#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Usage: bootstrap_core.sh MRUSTC_CHECKOUT rust-src-1.90.0.tar.xz FRESH_OUTPUT
# No network access, allocator, fabricated atomic implementation or system core.
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
compiler=$(CDPATH= cd -- "${1:?pinned and patched mrustc checkout}" && pwd)
archive=$(realpath "${2:?Rust 1.90.0 source archive}")
out=${3:?fresh absolute output directory}
case "$out" in /*) ;; *) echo 'Output must be absolute' >&2; exit 1;; esac
test ! -e "$out"
tc=${JIELI_TOOLCHAIN:?vendor toolchain root}
python3 "$repo/rust/toolchain/verify_mrustc.py" "$compiler"
printf '%s  %s\n' cde088d57064d151b2236f4619aea4a8207e0709eb3035ddc6617d609ab7d453 "$archive" | sha256sum -c -
make -C "$compiler" -j "${FM1_JOBS:-4}" bin/mrustc
mkdir -p "$out/source"
tar -xJf "$archive" -C "$out/source"
core="$out/source/rust-src-1.90.0/rust-src/lib/rustlib/src/rust/library/core/src/lib.rs"
test -f "$core"
export MRUSTC_TARGET_VER=1.90
"$compiler/bin/mrustc" "$core" --crate-name core --crate-type rlib --edition 2021 \
    --target "$repo/rust/toolchain/pi32v2-mrustc.toml" --cfg bootstrap --cfg no_fp_fmt_parse \
    -O -C panic=abort -o "$out/libcore.rlib" -C "emit-build-command=$out/core-build-command.txt" > "$out/core.log" 2>&1
"$tc/pi32v2/bin/clang" -target pi32v2 -mcpu=r3 -mfprev1 -ffp-contract=off -O2 -flto \
    -ffunction-sections -fdata-sections -std=gnu11 -I"$repo/rust/toolchain/freestanding" \
    -c "$out/libcore.rlib.c" -o "$out/libcore.lto.o" > "$out/core-c.log" 2>&1
sha256sum "$out/libcore.rlib.hir" "$out/libcore.rlib.c" "$out/libcore.lto.o" > "$out/core.sha256"
printf 'Pinned core metadata, generated C and vendor LTO object: %s\n' "$out"
