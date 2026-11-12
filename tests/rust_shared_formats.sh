#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Optional arg: immutable R01 source for independent C/JS/Python reproduction.
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$repo"
out="$repo/build/formats"
mkdir -p "$out/gen"
sha256sum -c tests/reference/formats.sha256
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/r01-verification.log"
    (cd "$baseline" && python3 tools/gen_font.py "$out/gen/felucca_font.h") > "$out/font.log"
    python3 tests/reference/formats_usb_reference.py "$baseline" "$out/gen/formats_dx.inc"
    (cd "$baseline" && cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$out/gen" -o "$out/reference" "$repo/tests/reference/formats_reference.c" firmware/src/dsp/amb.c -lm)
    "$out/reference" > "$out/c-reference.json"
    node tests/reference/formats_reference.mjs "$baseline" > "$out/js-reference.json"
    python3 tests/reference/formats_reference.py "$baseline" > "$out/python-reference.json"
    for kind in c js python; do cmp "$out/$kind-reference.json" "rust/fm1-formats/tests/formats/$kind-reference.json"; done
fi
cargo +1.94.1 test --locked --release -p fm1-formats -p fm1-jam-bridge
cargo +1.94.1 clippy --locked -p fm1-formats -p fm1-jam-bridge --all-targets -- -D warnings
cargo +1.94.1 check --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown
[ -f "$out/gen/felucca_font.h" ] || python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
python3 tests/reference/formats_usb_reference.py . "$out/gen/formats_dx.inc"
python3 tests/reference/formats_usb_reference.py . "$out/gen/formats_dx_current.inc"
cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$out/gen" -o "$out/current-c0" tests/reference/formats_reference.c firmware/src/dsp/amb.c -lm
"$out/current-c0" > "$out/current-c0.json"
cmp "$out/current-c0.json" rust/fm1-formats/tests/formats/c-reference.json
cargo +1.94.1 build --locked --release -p fm1-jam-bridge --target-dir "$out/native"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_RUST_FORMATS=1 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$out/gen" -o "$out/rust-reference" tests/reference/formats_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/rust-reference" > "$out/rust-reference.json" 2> "$out/native-proof.json"
cmp "$out/rust-reference.json" rust/fm1-formats/tests/formats/c-reference.json
cc -O2 -std=gnu99 -DEN_RUST_FORMATS=1 -o "$out/dxproto-rust" tests/host/dxproto_test.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/dxproto-rust"
cat "$out/native-proof.json"
