#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-only
# Host-only gate. Python3.8+, pinned Rust1.94.1; no third-party Python
# dependencies, MIDI import, ports, audio I/O, firmware build or hardware needed.
set -eu
cd "$(dirname "$0")/.."
exec python3 tests/rust_tooling.py "${1:?immutable R01 source directory}"
