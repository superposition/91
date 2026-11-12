#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Build both implementations from this checkout; frozen R01 PCM fingerprints
# additionally catch a simultaneous change to the C reference and Rust build.
set -eu
cd "$(dirname "$0")/.."
out=build/emu-parity
mkdir -p "$out"
export EN_VERSION=0.3.0
EN_RUST_JAM=0 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/reference.wasm"
EN_RUST_JAM=1 sh web/emu/build_clang.sh
cp build/emu/amb.wasm "$out/rust.wasm"
node tests/host/emu_test.mjs "$out/rust.wasm"
node tests/host/emu_http_test.mjs "$out/rust.wasm"
node tests/host/emu_rust_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/factory.json"
cat "$out/factory.json"
node tests/host/emu_rust_test.mjs "$out/reference.wasm" "$out/rust.wasm" rust/fm1-engine/tests/packs/harris-citypop.jmp > "$out/saved-v1.json"
cat "$out/saved-v1.json"
node tests/host/emu_session_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/session.json"
cat "$out/session.json"
node tests/host/emu_handoff_test.mjs "$out/rust.wasm" > "$out/handoff.json"
cat "$out/handoff.json"
node tests/host/emu_sequencer_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/sequencer.json"
cat "$out/sequencer.json"
node tests/host/emu_factory_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/factory-lines.json"
cat "$out/factory-lines.json"
node tests/host/emu_fm6_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/fm6.json"
cat "$out/fm6.json"
node tests/host/emu_worker_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/worker.json"
cat "$out/worker.json"
node tests/host/emu_dj_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/dj.json"
cat "$out/dj.json"
node tests/host/emu_effects_test.mjs "$out/reference.wasm" "$out/rust.wasm" > "$out/effects.json"
cat "$out/effects.json"
sha256sum "$out/reference.wasm" "$out/rust.wasm" > "$out/sha256.txt"
