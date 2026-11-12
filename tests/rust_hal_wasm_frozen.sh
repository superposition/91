#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
set -eu
cd "$(dirname "$0")/.."
reference=${1:-../baseline-2026-11-11/source/build/emu/amb.wasm}
rust=${2:-build/emu-parity/rust.wasm}
python3 - "$reference" <<'PY'
import hashlib,sys
from pathlib import Path
assert hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest()=='a3d08947713bda76b64f76ed39b55aa26bdd182a65bf88fbd1efe0cbc12f8036'
PY
for name in emu_rust_test emu_session_test emu_sequencer_test emu_factory_test emu_project_test emu_fm6_test emu_worker_test emu_dj_test emu_effects_test;do
    node "tests/host/$name.mjs" "$reference" "$rust"
done
node tests/host/emu_rust_test.mjs "$reference" "$rust" rust/fm1-engine/tests/packs/harris-citypop.jmp
