#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""The actual release wrapper must not conceal a failed emulator build."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.name == "posix" and shutil.which("sh"), "requires a POSIX shell")
class ReleaseExitStatus(unittest.TestCase):
    def run_release(self, emulator_status):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directory in ("tools", "tests", "web/emu", "build"):
                (root / directory).mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / "tools/release.sh", root / "tools/release.sh")
            for script in ("build_codecs.py", "make_params.py", "manual_html.py"):
                (root / "tools" / script).write_text("pass\n", encoding="utf-8")
            scripts = {
                "tests/run_tests.sh": "exit 0\n",
                "build.sh": "printf package > build/naiwan91-0.4.3.fwsc\n",
                "web/emu/build_clang.sh": (
                    "[ \"$EN_VERSION\" = 0.4.3 ] || exit 99\n"
                    "echo emulator-builder-ran\n"
                    f"exit {emulator_status}\n"
                ),
            }
            for name, body in scripts.items():
                path = root / name
                path.write_text("#!/bin/sh\n" + body, encoding="utf-8")
                path.chmod(0o755)
            return subprocess.run(
                ["sh", str(root / "tools/release.sh"), "0.4.3"],
                cwd=root, capture_output=True, text=True,
            )

    def test_failed_emulator_stops_release_even_when_package_exists(self):
        result = self.run_release(23)
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertIn("emulator-builder-ran", result.stdout)
        self.assertNotIn("build/naiwan91-0.4.3.fwsc", result.stdout)

    def test_successful_emulator_finishes_release(self):
        result = self.run_release(0)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("build/naiwan91-0.4.3.fwsc", result.stdout)


if __name__ == "__main__":
    unittest.main()
