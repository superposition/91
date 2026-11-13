#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
sh tests/rust_ui_values_snapshot.sh
UI_APP_CORPUS=values sh tests/rust_ui_input.sh "$@"
