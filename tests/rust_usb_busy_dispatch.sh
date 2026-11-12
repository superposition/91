#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:?independent frozen source path}
out=build/usb-busy
mkdir -p "$out/gen"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
python3 tests/reference/usb_busy_dispatch_reference.py "$baseline" "$out/frozen" --frozen-quirk
python3 tests/reference/usb_busy_dispatch_reference.py "$repo" "$out/current"
common="-O2 -ffp-contract=off -std=c99 -Wno-unused-function -D_POSIX_C_SOURCE=200809L -I$out/gen -Ifirmware/src -Ifirmware/src/app -Ifirmware/src/dsp"
cc $common -I"$out/frozen" "$out/frozen/test.c" firmware/src/dsp/amb.c -lm -o "$out/frozen/test"
cc $common -I"$out/current" "$out/current/test.c" firmware/src/dsp/amb.c -lm -o "$out/current/test"
"$out/frozen/test" > "$out/frozen/results.txt"
"$out/current/test" > "$out/current/results.txt"
cmp "$out/frozen/results.txt" "$out/current/results.txt"
cat "$out/current/results.txt"
echo 'Frozen ready-length loss witnessed; current retained payload/length survives incoming F0 and Busy dispatch retries.'
