#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""The web editor's data (build/params.json), from the firmware itself so it cannot drift from it:

  * every parameter (name, range, default, the names of its values): tests/host/gparams.c compiled against the engine
  * the unit's own pages and what their four knobs are: firmware/src/app/ui.c (VIEW_NAME, VIEW_KNOB, KNOB_LABEL)
  * the DX7 algorithms' routing: firmware/src/dsp/fm6_core.inc (FM6_ALG)

    python3 tools/make_params.py            (needs cc; run it where the release is built)
"""
import json
import os
import subprocess
import sys
from pathlib import Path
from rust_site import transform

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, ".."))
AMB_H = os.path.join(ROOT, "firmware", "src", "dsp", "amb.h")
UI_C = os.path.join(ROOT, "firmware", "src", "app", "ui.c")
FM6 = os.path.join(ROOT, "firmware", "src", "dsp", "fm6_core.inc")
OUT = os.path.join(ROOT, "build", "params.json")

LANES = ["Kick", "Snare", "Clap", "Hat", "Open hat", "Perc"]
LAYERS = ["Pad", "Bass", "Acid", "Pluck", "Drums"]


def strip_comments(t):
    return transform("metadata.strip-comments", t)


def param_enum():
    with open(AMB_H, encoding="utf-8") as source:
        return transform("metadata.enum", source.read())


def ui_tables(P):
    with open(UI_C, encoding="utf-8") as source:
        return transform("metadata.ui", source.read(), P)


def algorithms():
    with open(FM6, encoding="utf-8") as source:
        return transform("metadata.algorithms", source.read())


def main():
    cc = os.environ.get("CC", "cc")
    exe = os.path.join(ROOT, "build", "host", "gparams")
    os.makedirs(os.path.dirname(exe), exist_ok=True)
    subprocess.check_call([cc, "-O1", "-std=c99", "-DEN_HOST", "-I" + os.path.join(ROOT, "firmware", "src"), "-I" + os.path.join(ROOT, "firmware", "src", "dsp"), "-o", exe,
                           os.path.join(ROOT, "tests", "host", "gparams.c"), os.path.join(ROOT, "firmware", "src", "dsp", "amb.c"), "-lm"])
    params = json.loads(subprocess.check_output([exe]).decode("utf-8"))
    P = param_enum()
    # C currently supplies display/value names; Rust checks every numeric spec
    # against the maintained instrument table and supplies canonical FM6 routing.
    result = transform("metadata.assemble", params, P, ui_tables(P), algorithms(),
                       json.loads(subprocess.check_output([exe, "pieces"]).decode("utf-8")),
                       json.loads((Path(ROOT)/"web/product.json").read_text(encoding="utf-8")), True)
    with open(OUT, "w", encoding="utf-8") as output:
        output.write(result["params"])
    generated = result["device"]
    target = Path(ROOT)/"web/device-data.js"
    if "--check" in sys.argv:
        if not target.exists() or target.read_text(encoding="utf-8") != generated:
            sys.exit("web/device-data.js is stale: run python3 tools/make_params.py")
    else:
        target.write_text(generated, encoding="utf-8")
    sys.stdout.write("params.json: %d parameters, %d tabs, %d sections, %d not on any tab, %d algorithms (%d B)\n" % tuple(result["summary"] + [os.path.getsize(OUT)]))


if __name__ == "__main__":
    main()
