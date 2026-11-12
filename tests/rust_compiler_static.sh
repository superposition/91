#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Usage: bash tests/rust_compiler_static.sh MRUSTC CORE_OUTPUT ZIG [OUTPUT]
set -eu
cd "$(dirname "$0")/.."
repo=$PWD
compiler=$(realpath "${1:?patched compiler binary}")
core=$(realpath "${2:?fresh generated core}")
zig=$(realpath "${3:?pinned Zig compiler}")
out=${4:-build/compiler-static-regression}
mkdir -p "$out"
out=$(realpath "$out")
tc=${JIELI_TOOLCHAIN:?vendor toolchain root}
export MRUSTC_TARGET_VER=1.90
cp rust/toolchain/static_exports.rs "$out/static_exports.rs"
printf '#![no_std]\n#[path = "static_exports.rs"] mod probe;\n' > "$out/crate.rs"
"$compiler" "$out/crate.rs" --crate-name static_exports --crate-type rlib --edition 2021 \
    --target "$repo/rust/toolchain/pi32v2-mrustc.toml" -L "$core" --extern "core=$core/libcore.rlib" \
    -O -C panic=abort -o "$out/libstatic_exports.rlib" -C "emit-build-command=$out/command.txt" > "$out/emit.log" 2>&1
cat > "$out/probe.c" <<'C'
#include <stdint.h>
#include <assert.h>
extern const uint32_t fm1_export_value, FM1_NOMANGLE_VALUE;
extern uint32_t fm1_export_cell, fm1_export_cell_step(void);
int main(void) {
    assert(fm1_export_value == 0x11223344u);
    assert(FM1_NOMANGLE_VALUE == 0x55667788u);
    assert(fm1_export_cell == 17);
    assert(fm1_export_cell_step() == 18);
    assert(fm1_export_cell == 18);
}
C
"$zig" cc -target x86-linux-musl -O2 -std=gnu11 -ffunction-sections -fdata-sections \
    -Wl,--gc-sections -static "$out/probe.c" "$out/libstatic_exports.rlib.c" -o "$out/generated"
"$out/generated"
rustc +1.94.1 --edition 2021 --test rust/toolchain/static_exports.rs -o "$out/rustc"
"$out/rustc"
"$tc/pi32v2/bin/clang" -target pi32v2 -mcpu=r3 -mfprev1 -ffp-contract=off -O2 \
    -ffunction-sections -fdata-sections -std=gnu11 -I"$repo/rust/toolchain/freestanding" \
    -c "$out/libstatic_exports.rlib.c" -o "$out/vendor.o" > "$out/vendor.log" 2>&1
"$tc/common/bin/objdump" -t "$out/vendor.o" > "$out/symbols.txt"
python3 - "$out/symbols.txt" <<'PY'
from pathlib import Path
import sys
text = Path(sys.argv[1]).read_text()
for name in ['fm1_export_value', 'FM1_NOMANGLE_VALUE', 'fm1_export_cell']:
    line = next(line for line in text.splitlines() if line.endswith(' ' + name))
    assert ('.data' if name == 'fm1_export_cell' else '.rodata') in line, line
    print(line)
PY
sha256sum "$compiler" "$core/libcore.rlib.c" "$out/libstatic_exports.rlib.c" "$out/vendor.o" > "$out/identities.sha256"
echo 'Export spelling, immutable rodata and writable UnsafeCell match.'
