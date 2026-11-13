#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""The DY voices' table from DX7 bank files: python3 -I tools/gen_fm6_dy.py tools/fm6_dy.txt BANKS [--rom DIR]

Reads the manifest (slot | name | bank file | voice | source), takes each voice's 128-byte packed record out of its bank (32-voice VMEM dumps, found
anywhere in the file) and writes firmware/src/dsp/fm6_dy.h (FM6_DY[][128], FM6_DY_NAME[]), build/fm6_dy.bin (the same records, for tests/host/dycal)
and build/PERF-1-test-voices.syx (the U lines, a 32-voice bank for the installer's "Load DX7 voices"). With --rom DIR every voice is compared with
the dumps in DIR's rom*.syx (Yamaha's ROM banks): one whose operator bytes match a ROM voice is refused, since those are not ours to take.
The bank files are data, not code: run with -I."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.dirname(HERE)
# -I omits the script directory. Only this trusted sibling module path is added;
# no working-directory or PYTHONPATH lookup is needed for bank data.
if HERE not in sys.path:
    sys.path.insert(0, HERE)
import rust_tooling as _tools
INIT = bytes(json.loads(open(os.path.join(HERE, "data", "dy-init.json"), encoding="utf-8").read()))


def _call(method, args, adapter=None):
    try:
        return _tools.call("asset.dy." + method, args, adapter=adapter)
    except _tools.ToolError as error:
        cls = {"ValueError": ValueError, "IndexError": IndexError,
               "SystemExit": SystemExit, "AssertionError": AssertionError}.get(error.kind)
        if cls is None:
            raise
        raise cls(error.args[0]) from None


def dumps(data):
    return [[bytes(record) for record in bank] for bank in _call("dumps", [list(data)])]


def name_of(r):
    return _call("name", [list(r)])


class _Adapter:
    lines = None
    def __call__(self, action):
        kind, args = action["kind"], action["args"]
        if kind == "read":
            return list(open(args[0], "rb").read())
        if kind == "listdir":
            return os.listdir(args[0])
        if kind == "open_lines":
            self.lines = open(args[0], encoding="utf-8")
        elif kind == "line":
            return self.lines.readline() or None
        elif kind == "close_lines":
            self.lines.close()
            self.lines = None
        elif kind == "print":
            print(args[0])
        elif kind == "write":
            open(args[0], "wb").write(bytes(args[1]))
        elif kind == "write_text":
            open(args[0], "w", encoding="utf-8", newline="\n").write(args[1])
        elif kind == "mkdir":
            os.makedirs(args[0], exist_ok=True)
        else:
            raise RuntimeError("unknown DY adapter action: " + kind)


def main():
    adapter = _Adapter()
    try:
        _call("main", [sys.argv[1:], SRC, __doc__], adapter)
    finally:
        if adapter.lines is not None:
            adapter.lines.close()


if __name__ == "__main__":
    main()
