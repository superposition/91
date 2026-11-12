#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise generated-output ownership, replacement/rollback and old entrypoints."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from web_site_support import MANIFEST, SiteOutput, compatibility_redirect, inventory, output_path, product_of, release_identity, strip_module


class BuildSafety(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        (self.root / "build").mkdir()
        self.destination = self.root / "build/site-test"

    def output(self, adopt=False):
        output = SiteOutput(self.root, self.destination, adopt_existing=adopt)
        self.addCleanup(output.discard)
        return output

    def populate(self, output, value="one", redirects=True):
        routes = ["index.html", "emu/index.html", "install/index.html", "editor/index.html",
                  "pieces/index.html", "library/index.html", "manual/index.html"]
        if redirects:
            routes += ["webapp/installer/index.html", "webapp/editor/index.html"]
        for route in routes:
            page = output.staging / route
            page.parent.mkdir(parents=True, exist_ok=True)
            page.write_text(f"<title>NAIWAN 91</title>{value}", encoding="utf-8")
        for asset in ("site.css", "fonts/test.woff2", "img/naiwan91-mark.svg"):
            path=output.staging/asset;path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(b"shared test asset")
        (output.staging / ".nojekyll").touch()

    def test_rejects_repository_build_inputs_and_external_outputs(self):
        for target in [self.root, self.root / "outside", self.root / "build", self.root / "build/emu/site",
                       self.root / "build/gen", self.root / "build/rust-emu/site"]:
            with self.subTest(target=target), self.assertRaises(ValueError):
                output_path(self.root, target)
        self.assertEqual(output_path(self.root, self.destination), self.destination)

    def test_unknown_output_and_local_changes_are_preserved(self):
        self.destination.mkdir()
        note = self.destination / "notes.txt"
        note.write_text("keep this", encoding="utf-8")
        with self.assertRaises(ValueError): self.output()
        self.assertEqual(note.read_text(), "keep this")
        note.unlink()  # only this test's named fixture
        first = self.output(); self.populate(first); first.publish({"version":"0.3.0"})
        (self.destination / "index.html").write_text("my local edit", encoding="utf-8")
        with self.assertRaises(ValueError): self.output()
        self.assertEqual((self.destination / "index.html").read_text(), "my local edit")

    def test_own_output_rebuilds_deterministically_and_rejects_unknown_additions(self):
        first = self.output(); self.populate(first); original = first.publish({"version":"0.3.0"})
        bytes_before = (self.destination / MANIFEST).read_bytes()
        second = self.output(); self.populate(second); repeated = second.publish({"version":"0.3.0"})
        self.assertEqual(original, repeated)
        self.assertEqual(bytes_before, (self.destination / MANIFEST).read_bytes())
        note = self.destination / "notes.txt"; note.write_text("keep this", encoding="utf-8")
        with self.assertRaises(ValueError): self.output()
        self.assertTrue(note.exists())

    def test_publish_failure_restores_previous_site(self):
        first = self.output(); self.populate(first); first.publish({"version":"0.3.0"})
        before = inventory(self.destination)
        second = self.output(); self.populate(second, "two")
        import web_site_support
        original = web_site_support.os.replace
        def fail_new(source, destination):
            if Path(source) == second.staging: raise OSError("simulated final rename failure")
            return original(source, destination)
        with patch.object(web_site_support.os, "replace", side_effect=fail_new):
            with self.assertRaises(OSError): second.publish({"version":"0.3.0"})
        self.assertEqual(before, inventory(self.destination))

    def test_explicit_legacy_adoption_recognizes_layout_and_refuses_unknown_files(self):
        legacy = self.output(); self.populate(legacy, redirects=False)
        for asset in ("site.css", "fonts/test.woff2", "img/naiwan91-mark.svg"):
            duplicate=legacy.staging/'emu'/asset;duplicate.parent.mkdir(parents=True,exist_ok=True)
            duplicate.write_bytes((legacy.staging/asset).read_bytes())
        legacy.staging.rename(self.destination); legacy.staging = None
        local_edit=b'<title>NAIWAN 91</title>my edited legacy landing page'
        (self.destination/'index.html').write_bytes(local_edit)
        with self.assertRaises(ValueError): self.output()
        migrating = self.output(adopt=True); self.populate(migrating)
        (self.destination/'emu/site.css').write_bytes(b"changed duplicate")
        with self.assertRaises(ValueError): migrating.publish({"version":"0.3.0"})
        (self.destination/'emu/site.css').write_bytes((self.destination/'site.css').read_bytes())
        migrating.publish({"version":"0.3.0"})
        self.assertTrue((self.destination / MANIFEST).is_file())
        self.assertIsNotNone(migrating.retained_backup)
        self.assertEqual((migrating.retained_backup/'index.html').read_bytes(),local_edit)
        self.assertNotEqual((self.destination/'index.html').read_bytes(),local_edit)
        (self.destination / MANIFEST).unlink()  # recreate only our old-format fixture
        note = self.destination / "notes.txt"; note.write_text("keep this", encoding="utf-8")
        migrating = self.output(adopt=True); self.populate(migrating)
        with self.assertRaises(ValueError): migrating.publish({"version":"0.3.0"})
        self.assertEqual(note.read_text(), "keep this")

    def test_symlink_output_and_nested_generated_site_are_rejected(self):
        outside = self.root / "outside"; outside.mkdir()
        link = self.root / "build/alias"
        try: link.symlink_to(outside, target_is_directory=True)
        except OSError: self.skipTest("symlinks unavailable for this account")
        self.addCleanup(link.unlink)
        with self.assertRaises(ValueError): output_path(self.root, link / "site")
        first = self.output(); self.populate(first); first.publish({"version":"0.3.0"})
        with self.assertRaises(ValueError): output_path(self.root, self.destination / "nested")

    def test_old_builder_keeps_helpers_but_never_writes(self):
        spec = importlib.util.spec_from_file_location("legacy_site", ROOT / "web/make_site.py")
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertIs(module.product_of, product_of); self.assertIs(module.strip_module, strip_module)
        result = subprocess.run([sys.executable, str(ROOT / "web/make_site.py"), "missing.fwsc", "0.3.0", str(self.destination)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("tools/make_pages_site.py", result.stderr)
        self.assertFalse(self.destination.exists())

    def test_package_identity_and_inline_module_helpers_preserve_wire_values(self):
        for identity in ["FM-1_7000300", "FM-1_906"]:
            raw = bytearray([0x7D] * (20 * 0x30))
            for i, character in enumerate(identity): raw[i * 0x30 + 0x2F] = (ord(character) + i + 1) & 255
            self.assertEqual(product_of(raw), identity)
        with self.assertRaises(ValueError): product_of(b"short")
        self.assertEqual(strip_module('import x from "./x.js";\nexport const marker="PERF-1";\n'), 'const marker="PERF-1";\n')
        for version,identity in [('0.3','FM-1_7000300'),('0.3.0','FM-1_7000300'),('2.4.15-rc1','FM-1_7020415')]:
            self.assertEqual(release_identity(version),identity)
        with self.assertRaises(ValueError): release_identity('100.2.3')

    def test_compatibility_urls_preserve_host_subpaths(self):
        from urllib.parse import urljoin
        for route, target in [("installer", "install"), ("editor", "editor")]:
            page = compatibility_redirect(f"../../{target}/", route)
            self.assertIn("location.search+location.hash", page)
            self.assertEqual(urljoin(f"https://example.test/nested/91/webapp/{route}/", f"../../{target}/"), f"https://example.test/nested/91/{target}/")

    @unittest.skipIf(os.name == "nt", "shell entrypoint checked in the Linux job")
    def test_archived_api_builder_requires_explicit_test_mode(self):
        result = subprocess.run(['sh',str(ROOT/'web/app/build.sh')],capture_output=True,text=True)
        self.assertEqual(result.returncode,2)
        self.assertIn('web/emu/build_clang.sh',result.stderr)
        self.assertIn('--legacy-test',result.stderr)

    def test_release_argument_cannot_be_a_path_or_html(self):
        result = subprocess.run([sys.executable,str(ROOT/'tools/make_pages_site.py'),'../../outside',str(self.destination)],capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('release version',result.stderr)
        self.assertFalse(self.destination.exists())


if __name__ == "__main__":
    unittest.main()
