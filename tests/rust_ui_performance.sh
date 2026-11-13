#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
UI_APP_CORPUS=performance sh tests/rust_ui_input.sh "$@"
out=build/ui-performance
mkdir -p "$out/gen"
python3 tests/reference/ui_frame_extract.py "${1:?immutable R01 source required}" "$out/gen/ui_tap_frozen.inc" tap
cc -O0 -fsanitize=undefined -fno-sanitize-recover=undefined -I"$out/gen" tests/reference/ui_tap_undefined.c -o "$out/tap-undefined"
cc -O0 -I"$out/gen" tests/reference/ui_tap_undefined.c -o "$out/tap-signal"
ulimit -c 0
python3 tests/reference/ui_tap_undefined.py "$out/tap-undefined" "$out/tap-undefined.json" "$out/tap-signal"
