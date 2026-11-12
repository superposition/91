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
meta = json.dumps({"version": ver, "product": product, "pkg": "../firmware/" + name, "sha": sha[:16]})
device = device_javascript(SRC, json.loads(meta), strip_module)
page = page.replace("/*DEVICE*/", device).replace("/*LIB*/", lib).replace("/*META*/", meta).replace("<h1>Install NAIWAN 91</h1>", f"<h1>Install NAIWAN 91 {ver}</h1>")
(OUT / "install" / "index.html").write_text(page, encoding="utf-8", newline="\n")

params = SRC / "build" / "params.json"
if params.exists():
    (OUT / "editor").mkdir(exist_ok=True)
    ed = (SRC / "web" / "editor" / "index.html").read_text(encoding="utf-8").replace("/*DEVICE*/", device).replace("/*PARAMS*/", params.read_text(encoding="utf-8"))
    (OUT / "editor" / "index.html").write_text(ed, encoding="utf-8", newline="\n")
    plib = "\n".join(strip_module((SRC / "web" / f).read_text(encoding="utf-8")) for f in ("fm1backup.js", "fm1piece.js", "fm1card.js", "fm1share.js", "fm1idle.js"))
    lib_json = (SRC / "web" / "library" / "library.json").read_text(encoding="utf-8")
    relay = SRC / "web" / "relay" / "relay.json"           # the library relay's address (web/relay/README.md), or none
    relay_json = relay.read_text(encoding="utf-8").strip() if relay.exists() else "{}"
    for d in ("pieces", "library") if SHARING else ():   # sharing pieces and sets, and the library of them: the modules, the parameters' names, the library's codes
        (OUT / d).mkdir(exist_ok=True)
        pg = (SRC / "web" / d / "index.html").read_text(encoding="utf-8").replace("/*DEVICE*/", device).replace("/*PARAMS*/", params.read_text(encoding="utf-8")).replace("/*LIB*/", plib)
        (OUT / d / "index.html").write_text(pg.replace("/*LIBRARY*/", lib_json).replace("/*RELAY*/", relay_json), encoding="utf-8", newline="\n")
manual = (SRC / "docs" / "manual.html").read_text(encoding="utf-8")
manual = re.sub(r'<link rel="stylesheet" href="https://fonts.googleapis.com[^>]+>\n', "", manual)
manual = manual.replace("</head>", '<link rel="stylesheet" href="../fonts/fm1-fonts.css">\n<link rel="stylesheet" href="../site.css">\n<style>.site .home img{display:none}</style>\n</head>', 1)
manual = manual.replace("<main>", '<main><div class="project-note"><p><b>Current Songbook:</b> <a href="../emu/">play the browser instrument</a> or read the <a href="https://github.com/superposition/91/blob/rust/emulator-first/docs/JAM_PACKS.md">Songbook guide</a>. The reference below covers the retained legacy instrument.</p></div>', 1)
for source_pdf, served_pdf in (("manual.pdf", "NAIWAN 91-manual.pdf"), ("quick-manual.pdf", "NAIWAN 91-quick-manual.pdf"), ("beginners-guide.pdf", "NAIWAN 91-beginners-guide.pdf")):
    if not (SRC / "build" / source_pdf).exists():
        manual = re.sub(r'<a\b[^>]*href="' + re.escape(served_pdf) + r'"[^>]*>.*?</a>', "", manual, flags=re.S)
(OUT / "manual" / "index.html").write_text(manual, encoding="utf-8", newline="\n")
(OUT / "manual" / "maps").mkdir(exist_ok=True)         # ready MIDI maps the manual links (0.16.18: the Launch Control XL 3's)
for f in (SRC / "docs" / "maps").glob("*.perfmap"):
    shutil.copy(f, OUT / "manual" / "maps" / f.name)
quick_card = ""
if (SRC / "build" / "quick-manual.pdf").exists():   # the printable quick manual (tools/make_quick_manual.py, made on Windows with Edge)
    shutil.copy(SRC / "build" / "quick-manual.pdf", OUT / "manual" / "NAIWAN 91-quick-manual.pdf")
    quick_card = '<a class="card" href="manual/NAIWAN 91-quick-manual.pdf" download><b>Quick manual (PDF)</b><span>Eight A4 pages to print: every control, page and key map on paper.</span></a>'
if (SRC / "build" / "beginners-guide.pdf").exists():  # the beginner's guide (docs/GUIDE.md, tools/make_quick_manual.py docs/GUIDE.md --out beginners-guide)
    shutil.copy(SRC / "build" / "beginners-guide.pdf", OUT / "manual" / "NAIWAN 91-beginners-guide.pdf")
    quick_card = '<a class="card" href="manual/NAIWAN 91-beginners-guide.pdf" download><b>Beginner\'s guide (PDF)</b><span>Your first hour, step by step, in plain words.</span></a>' + quick_card
if (SRC / "build" / "manual.pdf").exists():         # the whole manual as a PDF (Edge headless printing docs/manual.html: tools/make_manual_pdf.sh)
    shutil.copy(SRC / "build" / "manual.pdf", OUT / "manual" / "NAIWAN 91-manual.pdf")
    quick_card += '<a class="card" href="manual/NAIWAN 91-manual.pdf" download><b>Full manual (PDF)</b><span>The whole manual to download or print.</span></a>'
for img in [*(SRC / "docs" / "img").glob("perf-screen-*.png"), *(SRC / "docs" / "img").glob("guide-*.png")]:
    shutil.copy(img, OUT / "manual" / "img" / img.name)

landing = (SRC / "web" / "index.html").read_text(encoding="utf-8")
values = {"REPO": REPO, "VERSION": ver, "FIRMWARE_NAME": name, "SHA": sha,
          "PACKAGE_KB": str(len(raw) // 1024), "QUICK_CARDS": quick_card}
for key, value in values.items():
    landing = landing.replace("@@" + key + "@@", value)
if re.search(r"@@[A-Z_]+@@", landing):
    sys.exit("unresolved landing-page placeholder")
(OUT / "index.html").write_text(landing, encoding="utf-8", newline="\n")
if not SHARING:                                 # the menus and the landing page without them
    for f in OUT.rglob("*.html"):
        t = f.read_text(encoding="utf-8")
        u = re.sub(r'<a (?:class="card" )?href="(?:\.\./)?(?:pieces|library)/(?:[?#][^"]*)?"[^>]*>.*?</a>\n?', "", t)
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
