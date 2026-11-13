#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Keep frozen factory compatibility separate from current JMP3 factory parity.
set -eu
cd "$(dirname "$0")/.."
repo=$(pwd)
baseline=${1:-}
if [ -z "$baseline" ]; then
    python3 tools/frozen_source.py build/r01-source
    baseline="$repo/build/r01-source"
fi
baseline=$(cd "$baseline" && pwd)
out=build/emu-parity
mkdir -p "$out"
(cd "$baseline";sha256sum -c "$repo/tests/reference/source.sha256") > "$out/source-verification.log"
fixture="$baseline/firmware/src/dsp/song_factory.inc"
python3 tests/reference/factory_fixture.py "$fixture" > "$out/factory-fixture.json"
unset FM1_TEST_FACTORY_HEADER
python3 tests/reference/build_inputs.py > "$out/build-inputs.json"
export EN_VERSION=0.3.0
FM1_TEST_FACTORY_HEADER="$fixture" EN_RUST_JAM=0 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/legacy-reference.wasm"
FM1_TEST_FACTORY_HEADER="$fixture" EN_RUST_JAM=1 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/legacy-rust.wasm"
python3 tests/reference/build_inputs.py --check "$out/build-inputs.json" > "$out/legacy-build-verification.json"
node tests/host/emu_rust_test.mjs "$out/legacy-reference.wasm" "$out/legacy-rust.wasm" > "$out/legacy-factory.json"
cat "$out/legacy-factory.json"
node tests/host/emu_rust_test.mjs "$out/legacy-reference.wasm" "$out/legacy-rust.wasm" rust/fm1-engine/tests/packs/harris-citypop.jmp > "$out/legacy-saved-v1.json"
cat "$out/legacy-saved-v1.json"
# Ordinary builds below use the current table and leave build/emu production-shaped.
EN_RUST_JAM=0 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/reference.wasm"
EN_RUST_JAM=1 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/rust.wasm"
python3 tests/reference/build_inputs.py --check "$out/build-inputs.json" > "$out/current-build-verification.json"
node tests/host/emu_test.mjs "$out/rust.wasm"
if [ "${EN_RUST_APP:-1}" = 1 ];then
    node tests/host/emu_app_test.mjs "$out/rust.wasm"
    node tests/host/emu_runner_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/runner.json"
    cat "$out/runner.json"
    node tests/host/emu_controls_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/controls.json"
    cat "$out/controls.json"
fi
node tests/host/emu_http_test.mjs "$out/rust.wasm"
node tests/host/emu_synth_jam_test.mjs "$out/rust.wasm" > "$out/synth-jam.json"
cat "$out/synth-jam.json"
node tests/host/emu_rust_test.mjs "$out/reference.wasm" "$out/rust.wasm" --current-factory > "$out/factory.json"
cat "$out/factory.json"
node tests/host/emu_rust_test.mjs "$out/reference.wasm" "$out/rust.wasm" rust/fm1-engine/tests/packs/harris-citypop.jmp --current-factory > "$out/saved-v1.json"
cat "$out/saved-v1.json"
node tests/host/emu_session_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/session.json"
cat "$out/session.json"
node tests/host/emu_handoff_test.mjs "$out/rust.wasm" > "$out/handoff.json"
cat "$out/handoff.json"
node tests/host/emu_sequencer_test.mjs "$out/reference.wasm" "$out/rust.wasm" --harmony > "$out/sequencer.json"
cat "$out/sequencer.json"
node tests/host/emu_factory_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/factory-lines.json"
cat "$out/factory-lines.json"
node tests/host/emu_project_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/project.json"
cat "$out/project.json"
node tests/host/emu_fm6_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/fm6.json"
cat "$out/fm6.json"
node tests/host/emu_worker_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/worker.json"
cat "$out/worker.json"
node tests/host/emu_dj_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/dj.json"
cat "$out/dj.json"
node tests/host/emu_effects_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/effects.json"
cat "$out/effects.json"
sha256sum "$out/reference.wasm" "$out/rust.wasm" > "$out/sha256.txt"
