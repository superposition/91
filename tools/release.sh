#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# A release build of NAIWAN 91: parameter data, manual, tests, firmware package and browser emulator.
#   tools/release.sh VERSION [--no-tests]        e.g. tools/release.sh 0.4.4
# Then:  python3 tools/make_pages_site.py VERSION build/site   (the installer, emulator, editor and manual as a static site)
# See BUILDING.md for the toolchain (JIELI_TOOLCHAIN, AC79_SDK), clang with the wasm target and node.
set -e
# Legacy comparison data must never leak into a production release.
unset FM1_TEST_FACTORY_HEADER
cd "$(dirname "$0")/.."
V="${1:?usage: tools/release.sh VERSION [--no-tests]}"
python3 tools/build_codecs.py
python3 tools/make_params.py
python3 tools/manual_html.py
if [ "${2:-}" != "--no-tests" ]; then
    tests/run_tests.sh
fi
./build.sh --release "$V"
# Keep the builder's exit status: a pipe to tail would hide a failed emulator build.
EN_VERSION="$V" web/emu/build_clang.sh
ls -la "build/naiwan91-$V.fwsc"
