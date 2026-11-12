#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
"""Render the bitmap fonts to a C header (assets/fonts/):
  XS / S / B / M / L: Terminus 12, 16, 16 bold, 24 bold, 32 bold (BDF, SIL OFL 1.1); see SIZES. TTF fonts also work through render()
(anti-aliased, tabular figures via the OpenType `tnum` feature).

Glyph format: per glyph an advance width, a bitmap width and an offset; the
bitmap starts FONT_PAD pixels left of the pen position (room for side
bearings); rows top to bottom, 2 pixels per byte (high nibble first),
alpha 0..15.
"""
import json
import os
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import rust_tooling as _tools

FONTS = Path(__file__).resolve().parents[1] / "assets" / "fonts"
FONTSETS = {key: [tuple(row) for row in rows] for key, rows in
            json.loads((Path(__file__).parent / "data/fontsets.json").read_text(encoding="utf-8")).items()}
FONTSET = os.environ.get("X0X_FONTSET", "chakra")
FONTDIRS = [Path(p) for p in os.environ.get("X0X_FONT_DIRS", "").split(":") if p]
SIZES = FONTSETS[FONTSET]
PAD = 2


class _Adapter:
    """Pillow/file/console effects only; Rust chooses geometry and packed bytes."""
    def __call__(self, action):
        kind, args = action["kind"], action["args"]
        if kind == "exists":
            return Path(args[0]).exists()
        if kind == "read_bdf":
            return Path(args[0]).read_text(errors="replace")
        if kind == "print":
            print(args[0])
        elif kind == "open_ttf":
            self.font = ImageFont.truetype(args[0], args[1])
        elif kind == "axes":
            return [dict(axis, name=axis["name"] if isinstance(axis["name"], str)
                         else axis["name"].decode()) for axis in self.font.get_variation_axes()]
        elif kind == "set_axes":
            self.font.set_variation_by_axes(args[0])
        elif kind == "metrics":
            asc, desc = self.font.getmetrics()
            return dict(asc=asc, desc=desc, bbox_top=self.font.getbbox("A8|(", features=args[0])[1])
        elif kind == "length":
            return self.font.getlength(chr(args[0]), features=args[1])
        elif kind == "glyph":
            code, width, full, top, height, features = args
            image = Image.new("L", (width, full), 0)
            ImageDraw.Draw(image).text((PAD, 0), chr(code), font=self.font, fill=255, features=features)
            return list(image.crop((0, top, width, top + height)).tobytes())
        elif kind == "write_header":
            Path(args[0]).write_text(args[1])
        else:
            raise RuntimeError("unknown font adapter action: " + kind)


def _call(method, args):
    try:
        return _tools.call("asset.font." + method, args, adapter=_Adapter())
    except _tools.ToolError as error:
        kinds = {"AssertionError": AssertionError, "ValueError": ValueError,
                 "IndexError": IndexError, "StopIteration": StopIteration,
                 "SystemExit": SystemExit}
        if error.kind == "KeyError":
            raise KeyError(int(error.args[0])) from None
        cls = kinds.get(error.kind)
        if cls is None:
            raise
        raise cls(*([error.args[0]] if error.args[0] else [])) from None


def upscale(px_, w, h, scale):
    """Integer nearest-neighbour enlargement; preserve scale-one identity."""
    if scale == 1:
        return px_
    return _call("upscale", [px_, w, h, scale])


def _font(value):
    height, glyphs = value
    return height, [tuple(glyph) for glyph in glyphs]


def render_bdf(path, scale, first, last):
    return _font(_call("bdf", [str(path), scale, first, last]))


def font_path(name):
    return Path(_call("path", [name, str(FONTS), list(map(str, FONTDIRS))]))


def render(ttf, px, scale, pixel, first, last):
    return _font(_call("render", [ttf, px, scale, pixel, first, last,
                                  str(FONTS), list(map(str, FONTDIRS))]))


def main(out):
    _call("generate", [FONTSET, SIZES, str(out), str(FONTS), list(map(str, FONTDIRS))])


if __name__ == "__main__":
    main(sys.argv[1])
