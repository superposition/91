#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
export SEQUENCER_CASES='16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50'
export PARITY_JOBS=${PARITY_JOBS:-4}
sh tests/rust_transport_parity.sh "$@"
python3 tests/reference/melodic_coverage.py build/transport-parity
