#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
UI_APP_CORPUS=automatic sh tests/rust_ui_input.sh "$@"
