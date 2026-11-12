#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# The original fixtures are test evidence; no expanded fixture enters production.
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$repo"
out=build/formats
mkdir -p "$out/gen"
sha256sum -c tests/reference/browser-codecs.sha256
python3 tools/build_codecs.py
cargo +1.94.1 test --locked --release -p fm1-formats -p fm1-codecs-wasm
cargo +1.94.1 clippy --locked -p fm1-formats -p fm1-codecs-wasm -p fm1-jam-bridge --all-targets -- -D warnings
node tests/host/codecs_wasm_test.mjs
node tests/web_backup_test.js
node tests/web_piece_test.js
node tests/web_syx_test.js
node tests/web_ota_test.mjs
python3 tools/make_params.py --check
node tests/web_metadata_test.mjs
python3 tools/gen_font.py "$out/gen/felucca_font.h" > "$out/font.log"
flags="-O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I$out/gen"
cc $flags -o "$out/constructors-c0" tests/reference/constructors_reference.c firmware/src/dsp/amb.c -lm
"$out/constructors-c0" > "$out/constructors-c0.json"
cargo +1.94.1 build --locked --release -p fm1-jam-bridge --target-dir "$out/native"
cc $flags -DEN_RUST_FORMATS=1 -o "$out/constructors-rust" tests/reference/constructors_reference.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
"$out/constructors-rust" > "$out/constructors-rust.json"
cmp "$out/constructors-c0.json" "$out/constructors-rust.json"
if [ "$#" -gt 0 ]; then
    baseline=$(CDPATH= cd -- "$1" && pwd)
    (cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/constructors-source-verification.log"
    (cd "$baseline" && cc -O2 -ffp-contract=off -std=gnu99 -I. -Ifirmware/src -Ifirmware/src/dsp -I"$repo/$out/gen" -o "$repo/$out/constructors-frozen" "$repo/tests/reference/constructors_reference.c" firmware/src/dsp/amb.c -lm)
    "$out/constructors-frozen" > "$out/constructors.json"
    cmp "$out/constructors.json" "$out/constructors-rust.json"
    python3 tests/reference/codec_vectors.py
    sha256sum -c tests/reference/browser-codecs.sha256
fi
