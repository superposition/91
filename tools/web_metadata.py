# SPDX-License-Identifier: GPL-3.0-only
"""One product/instrument/release description for the connected web tools."""
from rust_site import transform
import json
from pathlib import Path
from build_codecs import ROOT, source_hash


def runtime_source(root=ROOT):
    def deliver(action):
        kind, args = action["kind"], action["args"]
        if kind == "read_text":
            return Path(args[0]).read_text(encoding="utf-8")
        if kind == "read_json":
            return json.loads(Path(args[0]).read_text(encoding="utf-8"))
        if kind == "read":
            return list(Path(args[0]).read_bytes())
        if kind == "source_hash":
            return source_hash(Path(args[0]))
        raise ValueError("unknown site delivery action: " + kind)
    return transform("site.runtime-source", str(root), adapter=deliver)


def device_javascript(src, release, strip_module):
    data = strip_module((src / "web/device-data.js").read_text(encoding="utf-8"))
    client = strip_module((src / "web/fm1device.js").read_text(encoding="utf-8"))
    codecs = strip_module(runtime_source(src))
    return transform("site.device", data, client, codecs, release)
