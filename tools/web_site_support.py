#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Package/module helpers derived from Leo Kuroshita (@kurogedelic), Hügelton Instruments.
"""Maintained static-site helpers and bounded generated-output replacement."""
import atexit
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import tempfile

BLOCKS, BLK, KEEP = 20, 0x30, 0x2F
MANIFEST = "site-build.json"
GENERATOR = "naiwan91-static-site"


def strip_module(source):
    """Inline the repository's single-line ESM imports/exports, unchanged."""
    source = re.sub(r"^export\s+", "", source, flags=re.M)
    return re.sub(r"^import .*?;\n", "", source, flags=re.M)


def product_of(raw):
    """Read the package marker identity using the fm1pkg.js wire algorithm."""
    if len(raw) < BLOCKS * BLK:
        raise ValueError("firmware package is shorter than its 20 identity blocks")
    return "".join(chr((marker - i - 1) & 255)
                   for i in range(BLOCKS) if (marker := raw[i * BLK + KEEP]) != 0x7D)


def release_identity(version):
    """The release identity contract used by tools/build.py: FM-1_7XXYYZZ."""
    match = re.fullmatch(r"(\d{1,2})\.(\d{1,2})(?:\.(\d{1,2}))?(-[A-Za-z0-9]+)?", version)
    if not match:
        raise ValueError("release version must be X.Y[.Z][-suffix], up to two digits each")
    return "FM-1_7%02d%02d%02d" % (int(match[1]), int(match[2]), int(match[3] or 0))


def output_path(source_root, requested):
    """Resolve an output strictly under build, outside every build input."""
    source_root = Path(source_root).resolve()
    build = source_root / "build"
    if build.resolve() != build:
        raise ValueError("build directory must not be a symlink or junction")
    normalized = Path(os.path.abspath(requested))
    resolved = normalized.resolve()
    if normalized != resolved:
        raise ValueError("site output must not pass through a symlink or junction")
    try:
        relative = resolved.relative_to(build)
    except ValueError:
        raise ValueError("site output must be a directory below this repository's build/") from None
    if not relative.parts:
        raise ValueError("site output cannot be build/ itself")
    if relative.parts[0] in {"emu", "gen", "loader", "host", "hardware", "emu-parity"} or relative.parts[0].startswith("rust-"):
        raise ValueError("site output overlaps a build input or verification directory")
    for parent in resolved.parents:
        if parent == build:
            break
        if (parent / MANIFEST).is_file():
            raise ValueError("site output cannot be nested inside another generated site")
    return resolved


def inventory(root):
    """Hash files without following symlinks, junctions or other reparse points."""
    files, directories = {}, []

    def visit(directory):
        with os.scandir(directory) as entries:
            for entry in sorted(entries, key=lambda item: item.name):
                path = Path(entry.path)
                info = entry.stat(follow_symlinks=False)
                if entry.is_symlink() or getattr(info, "st_file_attributes", 0) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400):
                    raise ValueError(f"generated site contains an aliased path: {path}")
                relative = path.relative_to(root).as_posix()
                if entry.is_dir(follow_symlinks=False):
                    directories.append(relative)
                    visit(path)
                elif entry.is_file(follow_symlinks=False):
                    if relative != MANIFEST:
                        files[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
                else:
                    raise ValueError(f"unsupported generated-site entry: {path}")

    visit(root)
    return {"files": files, "directories": sorted(directories)}


def verify_owned_site(root, adopt_existing=False, expected=None):
    if not root.exists():
        return
    if not root.is_dir():
        raise ValueError(f"site output is not a directory: {root}")
    if not any(root.iterdir()):
        return
    marker = root / MANIFEST
    if not marker.exists() and adopt_existing:
        # Explicit migration only: recognize the previous canonical layout,
        # then compare every path with this fresh build before replacing it.
        required = ("index.html", "emu/index.html", "install/index.html", "editor/index.html",
                    "pieces/index.html", "library/index.html", "manual/index.html")
        if not (root / ".nojekyll").is_file() or any(
                not (root / name).is_file() or "NAIWAN" not in (root / name).read_text(encoding="utf-8")
                for name in required):
            raise ValueError(f"cannot adopt an unrecognized legacy site layout: {root}")
        actual = inventory(root)
        if expected is not None:
            new_redirects = {"webapp/installer/index.html", "webapp/editor/index.html"}
            new_directories = {"webapp", "webapp/installer", "webapp/editor"}
            # Previous full-site builds bundled a second copy of the shared
            # emulator assets. Recognize only exact copies of their old root
            # counterpart, never arbitrary files inside those directories.
            redundant = {"emu/" + name: name for name in expected["files"]
                         if name == "site.css" or name.startswith("fonts/") or name == "img/naiwan91-mark.svg"}
            extras = set(actual["files"]) - set(expected["files"])
            if any(name not in redundant or actual["files"][name] != actual["files"].get(redundant[name]) for name in extras):
                raise ValueError(f"legacy output contains unknown or changed redundant assets: {root}")
            actual_files = set(actual["files"]) - extras
            actual_directories = set(actual["directories"]) - {"emu/fonts", "emu/img"}
            if (not actual_files <= set(expected["files"]) or not set(expected["files"]) - actual_files <= new_redirects
                    or not actual_directories <= set(expected["directories"]) or not set(expected["directories"]) - actual_directories <= new_directories):
                raise ValueError(f"legacy output contains unexpected or missing paths: {root}; choose a fresh output directory")
        return
    if not marker.is_file() or marker.is_symlink():
        raise ValueError(f"refusing to replace unrecognized output: {root}; choose a fresh directory or --adopt-existing for a verified older generated site")
    try:
        record = json.loads(marker.read_text(encoding="utf-8"))
    except (ValueError, OSError) as error:
        raise ValueError(f"cannot verify generated output: {root}: {error}") from None
    if not isinstance(record, dict) or record.get("generator") != GENERATOR or record.get("schema") != 1:
        raise ValueError(f"unrecognized site build manifest: {marker}")
    actual = inventory(root)
    if actual != {key: record.get(key) for key in ("files", "directories")}:
        raise ValueError(f"site output has local changes or unknown files: {root}; choose a fresh output directory")


class SiteOutput:
    """Build separately, then replace only a verified generated directory."""
    def __init__(self, source_root, requested, adopt_existing=False):
        self.source_root = Path(source_root).resolve()
        self.destination = output_path(self.source_root, requested)
        self.adopt_existing = adopt_existing
        self.retained_backup = None
        verify_owned_site(self.destination, adopt_existing=adopt_existing)
        self.destination.parent.mkdir(parents=True, exist_ok=True)
        self.staging = Path(tempfile.mkdtemp(prefix=f".{self.destination.name}-next-", dir=self.destination.parent)).resolve()
        output_path(self.source_root, self.staging)
        atexit.register(self.discard)

    def discard(self):
        if self.staging is not None and self.staging.exists():
            # This exclusively created staging directory is always rechecked
            # before a recursive operation; it never aliases the destination.
            checked = output_path(self.source_root, self.staging)
            inventory(checked)
            shutil.rmtree(checked)
        self.staging = None

    def publish(self, release):
        contents = inventory(self.staging)
        record = {"generator": GENERATOR, "schema": 1, "release": release, **contents}
        (self.staging / MANIFEST).write_bytes((json.dumps(record, sort_keys=True, indent=2) + "\n").encode("utf-8"))
        # Recheck both resolved absolute paths and ownership immediately before
        # moving/replacing anything. Existing unknown files are never deleted.
        final = output_path(self.source_root, self.destination)
        stage = output_path(self.source_root, self.staging)
        verify_owned_site(final, adopt_existing=self.adopt_existing, expected=contents)
        preserve_legacy = final.exists() and any(final.iterdir()) and not (final / MANIFEST).exists()
        backup = None
        if final.exists():
            backup = Path(tempfile.mkdtemp(prefix=f".{final.name}-previous-", dir=final.parent)).resolve()
            output_path(self.source_root, backup)
            backup.rmdir()  # only the freshly created empty directory
            os.replace(final, backup)
        try:
            os.replace(stage, final)
        except OSError:
            if backup is not None:
                os.replace(backup, final)
            raise
        self.staging = None
        if backup is not None:
            checked = output_path(self.source_root, backup)
            verify_owned_site(checked, adopt_existing=self.adopt_existing, expected=contents)
            if preserve_legacy:
                # A legacy path inventory cannot establish historical bytes.
                # Retain the entire old tree, including edits in known files.
                self.retained_backup = checked
            else:
                shutil.rmtree(checked)
        return record


def compatibility_redirect(target, label):
    """A relative compatibility URL that works below a GitHub Pages subpath."""
    # These values are maintained source constants, never package/user input.
    return ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1">'
            f'<title>NAIWAN 91 {label}</title><meta http-equiv="refresh" content="0; url={target}">'
            f'<link rel="canonical" href="{target}"><link rel="stylesheet" href="../../site.css"></head><body>'
            f'<p><a href="{target}">{label}</a></p>'
            f'<script>location.replace({json.dumps(target)}+location.search+location.hash);</script>'
            '</body></html>\n')
