#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Fingerprint native/WASM source inputs before and after a comparison build."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def snapshot():
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for directory in ("firmware", "rust", "host", "web/emu"):
        paths += [p for p in (ROOT / directory).rglob("*") if p.is_file() and
                  (p.suffix in (".c", ".h", ".inc", ".rs") or p.name == "Cargo.toml")]
    files = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in sorted(set(paths))}
    digest = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
    return {"source_sha256": digest, "files": files}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", type=Path)
    args = parser.parse_args()
    result = snapshot()
    if args.check:
        before = json.loads(args.check.read_text())
        changed = sorted(name for name in set(before["files"]) | set(result["files"])
                         if before["files"].get(name) != result["files"].get(name))
        if changed:
            raise SystemExit("source changed during comparison build: " + ", ".join(changed))
        print(json.dumps({"source_sha256": result["source_sha256"], "unchanged_files": len(result["files"])}))
    else:
        print(json.dumps(result, indent=2, sort_keys=True))
