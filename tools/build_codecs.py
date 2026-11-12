#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build the bounded browser codec module and its self-contained JS loader."""
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def source_hash(root=ROOT):
    files = [root / "Cargo.toml", root / "Cargo.lock", root / "web/codec-runtime.mjs"]
    for crate in ("fm1-types", "fm1-formats", "fm1-codecs-wasm"):
        files += sorted((root / "rust" / crate).rglob("*.rs"))
        files += [root / "rust" / crate / "Cargo.toml"]
    h = hashlib.sha256()
    for file in sorted(files):
        h.update(file.relative_to(root).as_posix().encode() + b"\0" + file.read_bytes())
    return h.hexdigest()


def runtime_source(root=ROOT):
    wasm = root / "build/codecs/fm1-codecs.wasm"
    manifest = json.loads((wasm.parent / "manifest.json").read_text(encoding="utf-8"))
    if manifest.get("source") != source_hash(root) or manifest.get("sha256") != hashlib.sha256(wasm.read_bytes()).hexdigest():
        raise ValueError("browser codecs are stale; run python3 tools/build_codecs.py")
    text = (root / "web/codec-runtime.mjs").read_text(encoding="utf-8")
    start = text.index("// NODE_ONLY_BEGIN")
    end = text.index("// NODE_ONLY_END") + len("// NODE_ONLY_END")
    text = text[:start] + text[end:]
    return text.replace("/*CODECS_BASE64*/", base64.b64encode(wasm.read_bytes()).decode("ascii"))


def build():
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.94.1")
    subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "fm1-codecs-wasm"], cwd=ROOT, env=env, check=True)
    target = Path(env.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target.is_absolute():
        target = ROOT / target
    data = (target / "wasm32-unknown-unknown/release/fm1_codecs_wasm.wasm").read_bytes()
    out = ROOT / "build/codecs"
    out.mkdir(parents=True, exist_ok=True)
    (out / "fm1-codecs.wasm").write_bytes(data)
    (out / "manifest.json").write_text(json.dumps({"abi": 1, "rust": "1.94.1", "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "source": source_hash()}, indent=2) + "\n")
    (out / "runtime.mjs").write_text(runtime_source(), encoding="utf-8", newline="\n")


if __name__ == "__main__":
    build()
