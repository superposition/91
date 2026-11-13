#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Link one selected scalar entry using the actual seven vendor-LTO libraries.
# This is an inspection ELF, never a bootable image or a device qualification.
set -eu
cd "$(dirname "$0")/.."
core=$(realpath "${1:?generated core output}")
bridge=$(realpath "${2:?generated seven-crate output}")
out=${3:-build/usb-endpoint-leaf}
: "${JIELI_TOOLCHAIN:?vendor toolchain root}"
mkdir -p "$out"
cat > "$out/leaf.ld" <<'LINKER'
ENTRY(fm1_rust_usb_endpoint)
SECTIONS {
  . = 0x02001000;
  .text : { *(.text*) *(.rodata*) }
  .data : { *(.data*) }
  .bss : { *(.bss*) *(COMMON) }
}
LINKER
objects=()
for package in hal types formats engine rt app jam_bridge;do
    objects+=("$bridge/libfm1_$package.rlib.o")
done
"$JIELI_TOOLCHAIN/pi32v2/bin/ld" --gc-sections -e fm1_rust_usb_endpoint -T "$out/leaf.ld" \
    "${objects[@]}" "$core/libcore.lto.o" -o "$out/leaf.elf" > "$out/link.log" 2>&1
"$JIELI_TOOLCHAIN/common/bin/objdump" -d "$out/leaf.elf" > "$out/leaf.dis"
"$JIELI_TOOLCHAIN/common/bin/objdump" -t "$out/leaf.elf" > "$out/leaf.symbols"
"$JIELI_TOOLCHAIN/common/bin/objdump" -h "$out/leaf.elf" > "$out/leaf.sections"
sha256sum "$out/leaf.elf" "$out/leaf.dis" "$out/leaf.symbols" "$out/leaf.sections" > "$out/identities.sha256"
echo 'Selected USB endpoint leaf linked with vendor LTO; inspect its prologue and call edges separately from C/full-image stacks.'
