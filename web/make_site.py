#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
"""Compatibility imports for the retired Felucca site builder.

The supported NAIWAN build is:
  python3 tools/make_pages_site.py VERSION build/site
The old command-line layout is retired and never writes an output directory.
"""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from web_site_support import BLK, BLOCKS, KEEP, product_of, strip_module  # noqa: F401,E402


def main(*_args):
    raise SystemExit("web/make_site.py is retired. Use: python3 tools/make_pages_site.py VERSION build/site")


if __name__ == "__main__":
    main()
