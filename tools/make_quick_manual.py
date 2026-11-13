#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""docs/QUICK.md -> build/quick-manual.html (print CSS embedded) -> build/quick-manual.pdf (Edge headless).

  tools/make_quick_manual.py [IN.md] [--html-only] [--measure] [--edge PATH]

--measure also prints each section's height in mm (each on a page of its own), to see what to trim.

The quick manual is an eight-page A4 sheet: QUICK.md is its source, written for print. A `---` rule or a
`<div class="page"></div>` line is a page break; a `<div class="cols">` .. `</div>` pair sets two columns;
a `<!-- footer: ... -->` line is the running footer (version, date, where the full manual is). Standard
library only: the Markdown converter below handles the subset the sheet uses (headings, paragraphs, bold,
italic, code, tables, lists, rules, raw HTML lines). After the PDF is written the page count is printed
(pypdf if it is installed, else counted in the PDF's bytes) with each page's first line, so an overflow
onto a ninth page is seen at once."""
import os
import subprocess
import sys
import time
from pathlib import Path
HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
import rust_manual as _manual

SRC = HERE.parent
EDGE = _manual.COPY["edge"]
WANT_PAGES = _manual.COPY["want_pages"]
IMG_BASE = None
CSS = (_manual.DATA / "quick-manual.css").read_text(encoding="utf-8")


def inline(s):
    return _manual.call("quick-inline", [s])


def table(lines):
    return _manual.call("table", [lines])


def blocks(text):
    return _manual.call("blocks", [text, _manual.config(globals())], globals())


def convert(md):
    return _manual.call("quick-convert", [md, _manual.config(globals())], globals())


def to_pdf(html_path, pdf_path, edge):
    _manual.call("to-pdf", [str(html_path), str(pdf_path), edge], globals())


def measure(pdf_path):
    _manual.call("measure", [str(pdf_path)], globals())


def page_report(pdf_path):
    value = _manual.call("page-report", [str(pdf_path)], globals())
    return value[0], value[1]


def main(argv):
    return _manual.call("quick-main", [argv, _manual.config(globals())], globals())


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
