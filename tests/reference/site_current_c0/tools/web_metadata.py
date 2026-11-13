# SPDX-License-Identifier: GPL-3.0-only
"""One product/instrument/release description for the connected web tools."""
import json
from build_codecs import runtime_source


def device_javascript(src, release, strip_module):
    data = strip_module((src / "web/device-data.js").read_text(encoding="utf-8"))
    client = strip_module((src / "web/fm1device.js").read_text(encoding="utf-8"))
    codecs = strip_module(runtime_source(src))
    return codecs + "\n" + data + "\nconst DEVICE_RELEASE = " + json.dumps(release).replace("<", "\\u003c") + ";\n" + client
