#!/usr/bin/env python3
"""The pieces' factory drum patterns, as grids: writes firmware/src/dsp/pats.h (included by amb.c).
Each piece has six patterns of six lanes (kick, snare, clap, closed hat, open hat, perc), 16 steps:
x a hit, . a rest. Roughly: 1 an intro, 2-4 the groove building, 5 a variation, 6 a tool (thin or
broken), so turning Pattern from 1 to 6 is an arrangement.
Edit the grids in tools/data/patterns.json; Rust validates and packs them."""
import json
import os
from pathlib import Path
import rust_tooling as _tools

LANES = ("K", "S", "C", "H", "O", "P")
_ = "................"
PIECES = [(name, what, [tuple(pattern) for pattern in patterns])
          for name, what, patterns in json.loads(
              (Path(__file__).parent / "data/patterns.json").read_text(encoding="utf-8"))]


def bits(s):
    try:
        return _tools.call("asset.bits", [s])
    except _tools.ToolError as error:
        if error.kind == "AssertionError":
            raise AssertionError(s) from None
        raise


try:
    _header = _tools.call("asset.patterns", [PIECES])
except _tools.ToolError as error:
    if error.kind == "AssertionError":
        raise AssertionError(error.args[0]) from None
    raise
path = os.path.join(os.path.dirname(__file__), "..", "firmware", "src", "dsp", "pats.h")
open(path, "w").write(_header)
print("wrote", os.path.normpath(path))
