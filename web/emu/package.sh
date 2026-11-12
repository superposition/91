#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Bundle the same local assets for standalone and full-site emulator previews.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/emu/fonts build/emu/img
cp web/emu/index.html web/emu/worklet.js web/site.css build/emu/
mkdir -p build/emu/src
cp web/emu/src/*.mjs build/emu/src/
cp web/fonts/* build/emu/fonts/
cp web/img/naiwan91-mark.svg build/emu/img/
sed -i 's|href="../site.css"|href="site.css"|g; s|href="../fonts/|href="fonts/|g; s|href="../img/naiwan91-mark.svg"|href="img/naiwan91-mark.svg"|g' build/emu/index.html
