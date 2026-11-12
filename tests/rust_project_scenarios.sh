#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
out=build/persistence
mkdir -p "$out" build/gen
python3 tools/gen_font.py build/gen/felucca_font.h > "$out/scenario-font.log"
cargo +1.94.1 build --release --locked -p fm1-jam-bridge --target-dir "$out/native"
common='-O2 -ffp-contract=off -std=c99 -DEN_HOST -DEN_HOST_FIXED_CPU=0 -D_POSIX_C_SOURCE=200809L -Ifirmware/src -Ifirmware/src/dsp -Ibuild/gen'
# Isolate persistence/codec changes: full DSP integration is covered by the
# separate strict WASM PCM suite. Only the wall-clock CPU label is fixed here.
flags='-DEN_RUST_FORMATS=1 -DEN_RUST_STORAGE=1'
cc $common -o "$out/scenario-c0" host/amb_host.c firmware/src/dsp/amb.c -lm
cc $common $flags -o "$out/scenario-rust" host/amb_host.c firmware/src/dsp/amb.c "$out/native/release/libfm1_jam_bridge.a" -lm -ldl -lpthread
for scenario in store genrestore restore restore_old restore_save restore_shared setlist start resume setmem settings pluckmig basssteps midimap setmap seamless; do
    for flavor in c0 rust; do
        mkdir -p "$out/scenarios/$flavor/$scenario"
        rc=0
        AMB_LEGACY_BOOT=1 "$out/scenario-$flavor" "tests/scenarios/$scenario.amb" "$out/scenarios/$flavor/$scenario" > "$out/scenarios/$flavor/$scenario.log" 2>&1 || rc=$?
        echo "$rc" > "$out/scenarios/$flavor/$scenario.rc"
        sed -e '/^host: .*render .*host CPU)/d' -e 's@scenarios/c0/@scenarios/FLAVOR/@g' -e 's@scenarios/rust/@scenarios/FLAVOR/@g' "$out/scenarios/$flavor/$scenario.log" > "$out/scenarios/$flavor/$scenario.normalized"
    done
    cmp "$out/scenarios/c0/$scenario.rc" "$out/scenarios/rust/$scenario.rc"
    cmp "$out/scenarios/c0/$scenario.normalized" "$out/scenarios/rust/$scenario.normalized"
    echo "$scenario: C0/Rust trace and exit status match (exit$(cat "$out/scenarios/c0/$scenario.rc"))"
done
python3 - <<'PY'
from pathlib import Path
import json
out=Path('build/persistence/scenarios')
existing=json.loads(Path('tests/reference/project-scenario-baseline.json').read_text())['existingFailures']
for path in (out/'c0').glob('*.rc'):
    name=path.stem
    rc=int(path.read_text())
    failures=[line for line in path.with_suffix('.log').read_text().splitlines() if line.startswith('FAIL')]
    assert rc==(1 if name in existing else 0),(name,rc)
    assert failures==existing.get(name,[]),(name,failures)
counts={'.wav':0,'.png':0}
for path in (out/'c0').rglob('*'):
    if path.is_file() and path.suffix in ('.png','.wav'):
        other=out/'rust'/path.relative_to(out/'c0')
        assert other.read_bytes()==path.read_bytes(), str(path)
        counts[path.suffix]+=1
print(f"{counts['.png']} PNG and {counts['.wav']} WAV artifacts are byte-identical; 14 assertion-clean scenarios, 2 known inherited factory-expectation failures unchanged")
PY
