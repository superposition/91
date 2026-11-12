#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""NAIWAN 91's web site (for GitHub Pages or any static host) from a release build:

  python3 tools/make_pages_site.py VERSION OUTDIR      (after tools/release.sh VERSION)

OUTDIR gets index.html (the landing page), install/ (the web installer with the package in it),
emu/ (the browser emulator), editor/ (the parameter and DX7 voice editor), pieces/ and library/ (sharing), manual/ and firmware/perf1-VERSION.fwsc.
  --no-sharing   leaves out pieces/ and library/ and their links (a release whose sharing pages are not ready: 0.16.1)
All links are relative, so it works at any path (https://USER.github.io/91/ and a local server alike)."""
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path
from web_metadata import device_javascript

SRC = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SRC / "web"))
from make_site import product_of, strip_module  # noqa: E402

SHARING = "--no-sharing" not in sys.argv
args = [a for a in sys.argv[1:] if a != "--no-sharing"]
ver = args[0] if args else sys.exit(__doc__)
OUT = Path(args[1] if len(args) > 1 else SRC / "build" / "site")
REPO = json.loads((SRC / "web/product.json").read_text(encoding="utf-8"))["source"]
pkg = SRC / "build" / f"naiwan91-{ver}.fwsc"
raw = pkg.read_bytes()
app = SRC / "build" / "amb.bin"
if app.exists() and pkg.stat().st_mtime < app.stat().st_mtime:
    sys.exit(f"{pkg}: older than build/amb.bin — the package is stale: run ./build.sh --release {ver} first")
product = product_of(raw)
if not re.fullmatch(r"FM-1_7\d{6}", product):
    sys.exit(f"{pkg}: identity {product!r}: make a release build (tools/release.sh {ver})")
if b"FELUCCA-LOADER-1" not in raw:
    sys.exit(f"{pkg}: no update loader in it")
if OUT.exists():
    for old in OUT.iterdir():
        shutil.rmtree(old) if old.is_dir() else old.unlink()
for d in ("install", "firmware", "img", "manual/img"):
    (OUT / d).mkdir(parents=True, exist_ok=True)
(OUT / ".nojekyll").write_text("", encoding="utf-8")
name = f"naiwan91-{ver}.fwsc"
shutil.copy(pkg, OUT / "firmware" / name)
shutil.copytree(SRC / "build" / "emu", OUT / "emu")
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
        u = re.sub(r'<a (?:class="card" )?href="(?:\.\./)?(?:pieces|library)/"[^>]*>.*?</a>\n?', "", t)
        if u != t:
            f.write_text(u, encoding="utf-8", newline="\n")
print(f"site: {OUT}: index, install/, emu/, editor/, manual/, firmware/{name} ({len(raw)} B, {product})")
