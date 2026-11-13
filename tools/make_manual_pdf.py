#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""The whole manual as a PDF: docs/manual.html (tools/manual_html.py, with its print stylesheet) printed by Edge headless on Windows
to build/manual.pdf; tools/make_pages_site.py copies it to the site as manual/PERF-1-manual.pdf, next to the quick manual.
  python tools/make_manual_pdf.py [--edge PATH]     (PERF1_EDGE in the environment also names Edge)"""
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE.parent
sys.path.insert(0, str(HERE))
from make_quick_manual import EDGE, to_pdf
import rust_manual as _manual


def main(argv):
    return _manual.call("full-main", [argv, _manual.config(globals())], globals())


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
