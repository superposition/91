#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Regenerate independent golden data without overwriting maintained fixtures.
# Usage: sh tests/reference/generate.sh /absolute/R01/source [output-directory]
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
baseline=$(CDPATH= cd -- "${1:?provide the extracted R01 source directory}" && pwd)
mkdir -p "${2:-$repo/build/reference}"
out=$(CDPATH= cd -- "${2:-$repo/build/reference}" && pwd)
cd "$baseline"
sha256sum -c "$repo/tests/reference/source.sha256" > "$out/source-verification.log"
mkdir -p "$out/gen"
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp \
    -o "$out/planner-reference" "$repo/tests/reference/planner_reference.c" -lm
"$out/planner-reference" packs/afterhours.jmp packs/harris-citypop.jmp \
    packs/neon-turnaround.jmp packs/velvet-changes.jmp > "$out/planner.json"
cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$out/gen" \
    -o "$out/session-reference" "$repo/tests/reference/session_reference.c" firmware/src/dsp/amb.c -lm
"$out/session-reference" > "$out/session.json"
cmp "$out/planner.json" "$repo/rust/fm1-engine/tests/reference.json"
cmp "$out/session.json" "$repo/rust/fm1-formats/tests/session-reference.json"
echo "Frozen C planner/session oracles reproduced exactly: $out"
