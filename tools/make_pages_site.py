#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""NAIWAN 91's web site (for GitHub Pages or any static host) from a release build:

  python3 tools/make_pages_site.py VERSION OUTDIR      (after tools/release.sh VERSION)

OUTDIR gets index.html (the landing page), install/ (the web installer with the package in it),
emu/ (the browser emulator), editor/ (the parameter and DX7 voice editor), pieces/ and library/ (sharing), manual/ and firmware/naiwan91-VERSION.fwsc.
Outputs must live below this repository's build/ directory. Existing generated
sites are verified before replacement; --adopt-existing explicitly migrates a
verified older pre-manifest site. Unknown files and local changes are preserved.
  --no-sharing   leaves out pieces/ and library/ and their links
All links are relative, so it works at any path (https://USER.github.io/91/ and a local server alike)."""
import argparse
import hashlib
import json
import re
from rust_site import transform
import shutil
import sys
from pathlib import Path
from web_metadata import device_javascript

SRC = Path(__file__).resolve().parents[1]
from web_site_support import SiteOutput, compatibility_redirect, product_of, release_identity, strip_module

parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
parser.add_argument("version")
parser.add_argument("output", nargs="?", type=Path, default=SRC / "build" / "site")
parser.add_argument("--no-sharing", action="store_true")
parser.add_argument("--adopt-existing", action="store_true", help="migrate a verified older generated site without a build manifest")
args = parser.parse_args()
ver = args.version
try:
    expected_product = release_identity(ver)
except ValueError as error:
    parser.error(str(error))
SHARING = not args.no_sharing
REPO = json.loads((SRC / "web/product.json").read_text(encoding="utf-8"))["source"]
pkg = SRC / "build" / f"naiwan91-{ver}.fwsc"
raw = pkg.read_bytes()
app = SRC / "build" / "amb.bin"
if app.exists() and pkg.stat().st_mtime < app.stat().st_mtime:
    sys.exit(f"{pkg}: older than build/amb.bin — the package is stale: run ./build.sh --release {ver} first")
try:
    product = product_of(raw)
except ValueError as error:
    sys.exit(f"{pkg}: {error}")
if not re.fullmatch(r"FM-1_7\d{6}", product):
    sys.exit(f"{pkg}: identity {product!r}: make a release build (tools/release.sh {ver})")
if product != expected_product:
    sys.exit(f"{pkg}: package identity {product!r} does not match release {ver} ({expected_product})")
if b"FELUCCA-LOADER-1" not in raw:
    sys.exit(f"{pkg}: no update loader in it")
for required in ("build/params.json", "build/emu/amb.wasm", "build/emu/index.html", "build/emu/worklet.js", "build/emu/src/app.mjs", "build/codecs/manifest.json", "build/codecs/fm1-codecs.wasm", "docs/manual.html"):
    if not (SRC / required).is_file():
        sys.exit(f"missing site input: {required}; build the browser bundle and parameter/manual data first")
try:
    site_output = SiteOutput(SRC, args.output, adopt_existing=args.adopt_existing)
except ValueError as error:
    sys.exit(str(error))
OUT = site_output.staging
for d in ("install", "firmware", "img", "manual/img"):
    (OUT / d).mkdir(parents=True, exist_ok=True)
(OUT / ".nojekyll").write_text("", encoding="utf-8")
name = f"naiwan91-{ver}.fwsc"
shutil.copy(pkg, OUT / "firmware" / name)
shutil.copytree(SRC / "build" / "emu", OUT / "emu", ignore=shutil.ignore_patterns("fonts", "img", "site.css"))
# The standalone folder owns its assets; the full site shares these at its root.
shutil.copy(SRC / "web" / "emu" / "index.html", OUT / "emu" / "index.html")
shutil.copytree(SRC / "web" / "fonts", OUT / "fonts")   # the site's own typeface
shutil.copy(SRC / "web" / "site.css", OUT / "site.css")
shutil.copy(SRC / "web" / "img" / "naiwan91-mark.svg", OUT / "img" / "naiwan91-mark.svg")
for capture in (SRC / "docs" / "img").glob("naiwan91-*.png"):
    shutil.copy(capture, OUT / "img" / capture.name)

page = (SRC / "web" / "amb_installer.html").read_text(encoding="utf-8")
lib = strip_module((SRC / "web" / "fm1pkg.js").read_text(encoding="utf-8")) + "\n" + strip_module((SRC / "web" / "fm1ota.js").read_text(encoding="utf-8")) + "\n" + strip_module((SRC / "web" / "fm1backup.js").read_text(encoding="utf-8")) + "\n" + strip_module((SRC / "web" / "fm1idle.js").read_text(encoding="utf-8"))
sha = hashlib.sha256(raw).hexdigest()
meta = transform("site.json", {"version": ver, "product": product, "pkg": "../firmware/" + name, "sha": sha[:16]}, True, None, False, False)
device = device_javascript(SRC, json.loads(meta), strip_module)
page = transform("site.replace", page, [["/*DEVICE*/", device], ["/*LIB*/", lib], ["/*META*/", meta], ["<h1>Install NAIWAN 91</h1>", f"<h1>Install NAIWAN 91 {ver}</h1>"]])
(OUT / "install" / "index.html").write_text(page, encoding="utf-8", newline="\n")

params = SRC / "build" / "params.json"
if params.exists():
    (OUT / "editor").mkdir(exist_ok=True)
    ed = transform("site.replace", (SRC / "web" / "editor" / "index.html").read_text(encoding="utf-8"), [["/*DEVICE*/", device], ["/*PARAMS*/", params.read_text(encoding="utf-8")]])
    (OUT / "editor" / "index.html").write_text(ed, encoding="utf-8", newline="\n")
    plib = "\n".join(strip_module((SRC / "web" / f).read_text(encoding="utf-8")) for f in ("fm1backup.js", "fm1piece.js", "fm1card.js", "fm1share.js", "fm1idle.js"))
    lib_json = (SRC / "web" / "library" / "library.json").read_text(encoding="utf-8")
    relay = SRC / "web" / "relay" / "relay.json"           # the library relay's address (web/relay/README.md), or none
    relay_json = relay.read_text(encoding="utf-8").strip() if relay.exists() else "{}"
    for d in ("pieces", "library") if SHARING else ():   # sharing pieces and sets, and the library of them: the modules, the parameters' names, the library's codes
        (OUT / d).mkdir(exist_ok=True)
        pg = transform("site.replace", (SRC / "web" / d / "index.html").read_text(encoding="utf-8"), [["/*DEVICE*/", device], ["/*PARAMS*/", params.read_text(encoding="utf-8")], ["/*LIB*/", plib]])
        (OUT / d / "index.html").write_text(transform("site.replace", pg, [["/*LIBRARY*/", lib_json], ["/*RELAY*/", relay_json]]), encoding="utf-8", newline="\n")
manual = (SRC / "docs" / "manual.html").read_text(encoding="utf-8")
pdfs = [(SRC / "build" / name).exists() for name in ("manual.pdf", "quick-manual.pdf", "beginners-guide.pdf")]
manual = transform("site.manual", manual, pdfs)
(OUT / "manual" / "index.html").write_text(manual, encoding="utf-8", newline="\n")
(OUT / "manual" / "maps").mkdir(exist_ok=True)         # ready MIDI maps the manual links (0.16.18: the Launch Control XL 3's)
for f in (SRC / "docs" / "maps").glob("*.perfmap"):
    shutil.copy(f, OUT / "manual" / "maps" / f.name)
card_pdfs = [False, False, False]
if (SRC / "build" / "quick-manual.pdf").exists():   # the printable quick manual (tools/make_quick_manual.py, made on Windows with Edge)
    shutil.copy(SRC / "build" / "quick-manual.pdf", OUT / "manual" / "NAIWAN 91-quick-manual.pdf")
    card_pdfs[1] = True
if (SRC / "build" / "beginners-guide.pdf").exists():  # the beginner's guide (docs/GUIDE.md, tools/make_quick_manual.py docs/GUIDE.md --out beginners-guide)
    shutil.copy(SRC / "build" / "beginners-guide.pdf", OUT / "manual" / "NAIWAN 91-beginners-guide.pdf")
    card_pdfs[2] = True
if (SRC / "build" / "manual.pdf").exists():         # the whole manual as a PDF (Edge headless printing docs/manual.html: tools/make_manual_pdf.sh)
    shutil.copy(SRC / "build" / "manual.pdf", OUT / "manual" / "NAIWAN 91-manual.pdf")
    card_pdfs[0] = True
quick_card = transform("site.cards", card_pdfs)
for img in [*(SRC / "docs" / "img").glob("perf-screen-*.png"), *(SRC / "docs" / "img").glob("guide-*.png")]:
    shutil.copy(img, OUT / "manual" / "img" / img.name)

landing = (SRC / "web" / "index.html").read_text(encoding="utf-8")
values = {"REPO": REPO, "VERSION": ver, "FIRMWARE_NAME": name, "SHA": sha,
          "PACKAGE_KB": str(len(raw) // 1024), "QUICK_CARDS": quick_card}
landing = transform("site.landing", landing, values)
(OUT / "index.html").write_text(landing, encoding="utf-8", newline="\n")
if not SHARING:                                 # the menus and the landing page without them
    for f in OUT.rglob("*.html"):
        t = f.read_text(encoding="utf-8")
        u = transform("site.sharing", t)
        if u != t:
            f.write_text(u, encoding="utf-8", newline="\n")
for route, target, label in (("installer", "../../install/", "Install"), ("editor", "../../editor/", "Editor")):
    folder = OUT / "webapp" / route
    folder.mkdir(parents=True)
    (folder / "index.html").write_text(compatibility_redirect(target, label), encoding="utf-8")
try:
    site_output.publish({"version": ver, "product": product, "firmware": name, "firmware_sha256": sha})
except ValueError as error:
    sys.exit(str(error))
if site_output.retained_backup is not None:
    print(f"preserved pre-manifest site and any local edits: {site_output.retained_backup}", file=sys.stderr)
print(f"site: {site_output.destination}: index, install/, emu/, editor/, manual/, firmware/{name} ({len(raw)} B, {product})")
