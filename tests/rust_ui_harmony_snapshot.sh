#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/ui-harmony-snapshot
mkdir -p "$out/gen"
python3 tests/reference/harmony_readonly_extract.py "$out/gen/harmony_readonly_wrappers.inc"
flags="-O2 -std=gnu99 -DEN_HOST=1 -I$out/gen -Ifirmware/src/dsp -Ifirmware/hal"
cc $flags -DFM1_HARM_SNAPSHOT_TEST=1 tests/reference/harmony_readonly_test.c -o "$out/protected"
cc $flags -DFM1_HARM_NEGATIVE_CONTROL=1 tests/reference/harmony_readonly_test.c -o "$out/unmasked"
"$out/protected" >"$out/protected.json"
"$out/unmasked" >"$out/unmasked.json"
cat "$out/protected.json" "$out/unmasked.json"
