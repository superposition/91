#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
mkdir -p build/mcu-hal
python3 tests/reference/core1_hal_reference.py build/mcu-hal/reference.c
${CC:-cc} -O2 -std=c99 -o build/mcu-hal/reference build/mcu-hal/reference.c
build/mcu-hal/reference
