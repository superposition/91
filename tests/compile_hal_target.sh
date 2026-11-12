#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Compile only. This does not link, package, execute or access the device.
set -eu
cd "$(dirname "$0")/.."
out=build/hal-target
mkdir -p "$out"
: "${JIELI_TOOLCHAIN:?point at the directory containing pi32v2 and common}"
flags=(-target pi32v2 -mcpu=r3 -mfprev1 -ffp-contract=off -Os -ffunction-sections -fdata-sections -fno-builtin -Wall -Wno-unused-function -Ifirmware/hal -Ifirmware/src -Ibuild/gen -Irust/toolchain/freestanding -DEN_CORE2=1 -DEN_DIAG=1 -DEN_RUST_MCU_ATOMICS=1)
for name in JAM COMMANDS HANDOFF TRANSPORT SCHEDULER MELODIC EFFECTS FACTORY VOICES FM6 FORMATS WORKER STORAGE HAL;do flags+=("-DEN_RUST_$name=1");done
cc="$JIELI_TOOLCHAIN/pi32v2/bin/clang"
"$cc" "${flags[@]}" -c firmware/src/main.c -o "$out/main.o" > "$out/main.log" 2>&1
"$cc" "${flags[@]}" -O2 -c firmware/src/dsp/amb.c -o "$out/engine.o" > "$out/engine.log" 2>&1
"$JIELI_TOOLCHAIN/common/bin/objdump" -t "$out/main.o" > "$out/main-symbols.txt"
echo 'HAL1 platform/engine compile with the vendor PI32v2 compiler; image link and hardware are unqualified.'
