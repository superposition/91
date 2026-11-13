#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/ui-values-snapshot
mkdir -p "$out/gen"
python3 tests/reference/ui_values_snapshot_extract.py "$out/gen/ui_values_snapshot.inc"
flags="-O2 -std=gnu99 -I$out/gen -Ifirmware/hal"
cc $flags tests/reference/ui_values_snapshot_test.c -o "$out/protected"
cc $flags -DEN_HOST=1 tests/reference/ui_values_snapshot_test.c -o "$out/unmasked"
"$out/protected" >"$out/protected.json"
"$out/unmasked" >"$out/unmasked.json"
cat "$out/protected.json" "$out/unmasked.json"
