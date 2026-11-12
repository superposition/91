#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Archived AMB-style API regression harness. Never shipped by the site builder.
#   web/app/build.sh --legacy-test -> build/legacy-app-test/
set -eu
if [ "$#" -ne 1 ] || [ "$1" != "--legacy-test" ]; then
    echo 'Archived API test harness. Build the player with web/emu/build_clang.sh; build the site with tools/make_pages_site.py.' >&2
    echo 'For historical API regression tests only: sh web/app/build.sh --legacy-test' >&2
    exit 2
fi
cd "$(dirname "$0")/../.."
mkdir -p build/legacy-app-test
emcc -O2 -ffp-contract=off -std=gnu99 -Wall -Wextra -Wno-unused-function \
    -DEN_HOST -Ifirmware/src/dsp \
    --no-entry -sSTANDALONE_WASM -sSTACK_SIZE=262144 -sINITIAL_MEMORY=8388608 -sFILESYSTEM=0 \
    -o build/legacy-app-test/amb.wasm web/app/amb_api.c firmware/src/dsp/amb.c
cp web/app/index.html web/app/worklet.js build/legacy-app-test/
echo "archived API test: build/legacy-app-test ($(wc -c < build/legacy-app-test/amb.wasm) B wasm)"
