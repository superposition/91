#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""docs/MANUAL.md -> docs/manual.html (NAIWAN 91's page: the mascot's colours, the slinky logo and favicon
embedded from web/img). The Markdown is the source; run this after editing it.

  tools/manual_html.py [IN.md [OUT.html]] [--fragment]

--fragment writes the page without its <!doctype>/<html>/<head>/<body> wrapper (for hosts that
add their own); the default is a complete document that opens in any browser.

Handles what the manual uses: headings, paragraphs, **bold**, `code`, lists, tables, fenced
code, rules, and the contents list (linked to the sections by number)."""
import os
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
import rust_manual as _manual

SRC = HERE.parent
PART_COLOURS = {}
CSS = (_manual.DATA / "manual.css").read_text(encoding="utf-8")
NAME = _manual.COPY["name"]
WEB_IMG = SRC / "web" / "img"
REPO = os.environ.get("REPO_URL", "https://github.com/superposition/91")
IMG_BASE = SRC / "docs"
EMBED = False


def inline(t):
    return _manual.call("inline", [t, PART_COLOURS])


def slug(title):
    return _manual.call("slug", [title])


def data_uri(path):
    return _manual.call("uri", [str(path)], globals())


def figure(caption, src):
    return _manual.call("figure", [caption, src, _manual.config(globals())], globals())


def convert(md):
    return _manual.call("convert", [md, _manual.config(globals())], globals())


def main():
    _manual.call("main", [sys.argv[1:], _manual.config(globals())], globals())


if __name__ == "__main__":
    main()
