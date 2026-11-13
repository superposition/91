#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""The deamonSLINKY mascot (the sprite of the boot card in firmware/src/app/ui.c, its colours) as PNGs for the web pages:
web/img/slinky.png (a logo, 4 px a pixel) and web/img/favicon.png (32 x 32).  python tools/make_logo.py
The pages carry them inline (tools/make_logo.py --inline rewrites the data: URIs in web/amb_installer.html)."""
import io
import json
import os
import sys
from PIL import Image
import rust_tooling as _tools

HERE = os.path.dirname(os.path.abspath(__file__))
UI = os.path.join(HERE, "..", "firmware", "src", "app", "ui.c")
PAL = {key: tuple(value) for key, value in json.loads(
    open(os.path.join(HERE, "data", "logo-palette.json"), encoding="utf-8").read()).items()}


def _image(value):
    width, height, data = value
    return Image.frombytes("RGBA", (width, height), bytes(data)) if data else Image.new("RGBA", (width, height))


def _call(method, args, adapter=None):
    try:
        return _tools.call("asset.logo." + method, args, adapter=adapter)
    except _tools.ToolError as error:
        cls = {"ValueError": ValueError, "IndexError": IndexError,
               "AttributeError": AttributeError, "AssertionError": AssertionError}.get(error.kind)
        if cls is None:
            raise
        raise cls(error.args[0]) from None


def sprite():
    return _call("sprite", [open(UI, encoding="utf-8").read()])


def render(rows, scale, size=None):
    return _image(_call("render", [rows, scale, size, PAL]))


def png(im):
    b = io.BytesIO()
    im.save(b, "PNG", optimize=True)
    return b.getvalue()


class _Adapter:
    def __call__(self, action):
        kind, args = action["kind"], action["args"]
        if kind == "read_text":
            return open(args[0], encoding="utf-8").read()
        if kind == "png":
            return list(png(_image(args)))
        if kind == "mkdir":
            os.makedirs(args[0], exist_ok=True)
        elif kind == "write":
            open(args[0], "wb").write(bytes(args[1]))
        elif kind == "write_text":
            open(args[0], "w", encoding="utf-8", newline="\n").write(args[1])
        elif kind == "print":
            print(args[0])
        else:
            raise RuntimeError("unknown logo adapter action: " + kind)


def main():
    _call("main", [sys.argv[1:], HERE, UI, PAL], _Adapter())


if __name__ == "__main__":
    main()
