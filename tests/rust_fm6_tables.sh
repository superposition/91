#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-$repo}
out=build/fm6-tables
mkdir -p "$out"
if [ "$baseline" != "$repo" ]; then (cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log";fi
sha256sum -c tests/reference/fm6.sha256
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/fm6_tables_reference.c -lm -o "$out/reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/fm6_tables_reference.c -lm -o "$out/c0"
"$out/reference" > "$out/reference.json"
"$out/c0" > "$out/c0.json"
cmp "$out/reference.json" "$out/c0.json"
cmp "$out/reference.json" rust/fm1-engine/tests/fm6-tables.json
python3 tools/import_fm6_tables.py "$out/reference.json" --check
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -I"$baseline/firmware/src/dsp" tests/reference/fm6_components_reference.c -lm -o "$out/components-reference"
cc -O2 -ffp-contract=off -std=gnu99 -DEN_HOST -Ifirmware/src/dsp tests/reference/fm6_components_reference.c -lm -o "$out/components-c0"
"$out/components-reference" > "$out/components-reference.json"
"$out/components-c0" > "$out/components-c0.json"
cmp "$out/components-reference.json" "$out/components-c0.json"
cmp "$out/components-reference.json" rust/fm1-engine/tests/fm6-components.json
cargo test --locked -p fm1-engine --test fm6
echo 'Immutable FM tables/component goldens, fresh C0, generated source and safe Rust match.'
