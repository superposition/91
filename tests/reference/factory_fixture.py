#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Verify the independent R01 Songbook table used only by legacy test builds."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TABLE = "firmware/src/dsp/song_factory.inc"


def verify(path):
    expected = dict(line.split(maxsplit=1)[::-1] for line in
                    (ROOT / "tests/reference/source.sha256").read_text().splitlines())
    path = Path(path).resolve(strict=True)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != expected[TABLE]:
        raise ValueError("test factory differs from the frozen R01 table: " + str(path))
    return {"fixture": "frozen-r01-factory", "path": str(path), "sha256": digest}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("header", type=Path)
    parser.add_argument("--path", action="store_true")
    args = parser.parse_args()
    result = verify(args.header)
    print(result["path"] if args.path else json.dumps(result))
