#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# The established clock traces plus every genre's real mutation/fill/FX path.
set -eu
cd "$(dirname "$0")/.."
export SEQUENCER_CASES='16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34'
exec sh tests/rust_transport_parity.sh "$@"
