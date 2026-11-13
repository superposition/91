#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
out=build/device-wire
mkdir -p "$out" build/gen
baseline=${1:-}
if [ -z "$baseline" ]; then
    python3 tools/frozen_source.py build/r01-source
    baseline="$repo/build/r01-source"
fi
baseline=$(CDPATH= cd -- "$baseline" && pwd)
(cd "$baseline" && sha256sum -c "$repo/tests/reference/source.sha256") > "$out/r01-verification.log"
fixture="$baseline/firmware/src/dsp/song_factory.inc"
python3 tests/reference/factory_fixture.py "$fixture" > "$out/factory-fixture.json"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
lib="$out/native/release/libfm1_jam_bridge.a"
cc -O2 -std=c99 -Wno-unused-function -o "$out/c0" tests/reference/device_wire_reference.c
cc -O2 -std=c99 -Wno-unused-function -DEN_RUST_FORMATS=1 -o "$out/rust" tests/reference/device_wire_reference.c "$lib" -lm -ldl -lpthread
"$out/c0" > "$out/c0.bin"
"$out/rust" > "$out/rust.bin"
cmp "$out/c0.bin" "$out/rust.bin"
if [ "$#" -gt 0 ]; then
    cc -O2 -std=c99 -Wno-unused-function "-DDEVICE_OTA_SOURCE=\"$baseline/firmware/src/ota.c\"" -o "$out/frozen" tests/reference/device_wire_reference.c
    "$out/frozen" > "$out/frozen.bin"
    cmp "$out/frozen.bin" "$out/rust.bin"
fi
python3 tools/gen_font.py build/gen/felucca_font.h > "$out/font.log"
flags='-O2 -ffp-contract=off -std=c99 -D_POSIX_C_SOURCE=200809L -Wno-unused-function -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen'
# This historical USB regression retains its original song and assertions.
# Current JMP3 transfer/save coverage runs separately in rust_authoring.sh.
cc $flags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -o "$out/jam-c0" tests/host/jam_usb_test.c firmware/src/dsp/amb.c -lm
cc $flags "-DEN_TEST_FACTORY_HEADER=\"$fixture\"" -DEN_RUST_FORMATS=1 -o "$out/jam-rust" tests/host/jam_usb_test.c firmware/src/dsp/amb.c "$lib" -lm -ldl -lpthread
env -u AMB_LEGACY_BOOT "$out/jam-c0" > "$out/jam-c0.log"
env -u AMB_LEGACY_BOOT "$out/jam-rust" > "$out/jam-rust.log"
cmp "$out/jam-c0.log" "$out/jam-rust.log"
if [ "$#" -gt 1 ]; then
    cc -O2 -o "$out/ota-c0" tests/ota_test.c
    cc -O2 -DEN_RUST_FORMATS=1 -o "$out/ota-rust" tests/ota_test.c "$lib" -lm -ldl -lpthread
    "$out/ota-c0" "$2" > "$out/ota-c0.log"
    "$out/ota-rust" "$2" > "$out/ota-rust.log"
    cmp "$out/ota-c0.log" "$out/ota-rust.log"
fi
sha256sum "$out/c0.bin" "$out/rust.bin"
cat "$out/jam-rust.log"
