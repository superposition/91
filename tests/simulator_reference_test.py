#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise reference selection and verification before the native compiler runs."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.name == "posix" and shutil.which("sh"), "requires a POSIX shell")
class SimulatorReference(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for directory in ("tools", "tests/reference", "bin"):
            (self.root / directory).mkdir(parents=True)
        shutil.copyfile(ROOT / "tests/rust_simulator.sh", self.root / "tests/rust_simulator.sh")
        shutil.copyfile(ROOT / "tests/reference/factory_fixture.py", self.root / "tests/reference/factory_fixture.py")
        shutil.copyfile(ROOT / "tests/reference/build_inputs.py", self.root / "tests/reference/build_inputs.py")
        (self.root / "Cargo.toml").write_text("# fixture\n")
        (self.root / "Cargo.lock").write_text("# fixture\n")
        digest = hashlib.sha256(b"frozen\n").hexdigest()
        (self.root / "tests/reference/source.sha256").write_text(digest + "  proof.txt\n" + digest + "  firmware/src/dsp/song_factory.inc\n")
        (self.root / "tools/frozen_source.py").write_text(
            "from pathlib import Path\nimport sys\n"
            "p=Path(sys.argv[1]);p.mkdir(parents=True,exist_ok=True)\n"
            "(p/'proof.txt').write_bytes(b'frozen\\n')\n"
            "h=p/'firmware/src/dsp/song_factory.inc';h.parent.mkdir(parents=True,exist_ok=True);h.write_bytes(b'frozen\\n')\n"
        )
        for name in ("tools/gen_font.py", "tests/reference/simulator_oracle.py"):
            (self.root / name).write_text("pass\n")
        # Stop at the first compiler invocation; no fake parity result is produced.
        compiler = self.root / "bin/cc"
        compiler.write_text("#!/bin/sh\necho compiler-reached\nexit 73\n")
        compiler.chmod(0o755)

    def run_script(self, *args):
        environment = dict(os.environ, PATH=str(self.root / "bin") + os.pathsep + os.environ["PATH"])
        return subprocess.run(["sh", "tests/rust_simulator.sh", *args], cwd=self.root,
                              env=environment, capture_output=True, text=True)

    def test_default_reconstructs_and_verifies_before_compilation(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 73, result.stderr)
        self.assertEqual((self.root / "build/r01-source/proof.txt").read_bytes(), b"frozen\n")
        self.assertIn("proof.txt: OK", (self.root / "build/simulator/source-verification.log").read_text())
        self.assertIn("compiler-reached", result.stdout)

    def test_explicit_reference_is_verified_and_tampering_stops_compilation(self):
        reference = self.root / "external"
        reference.mkdir()
        (reference / "proof.txt").write_bytes(b"frozen\n")
        header = reference / "firmware/src/dsp/song_factory.inc"
        header.parent.mkdir(parents=True)
        header.write_bytes(b"frozen\n")
        self.assertEqual(self.run_script(str(reference)).returncode, 73)
        (reference / "proof.txt").write_bytes(b"changed\n")
        result = self.run_script(str(reference))
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("compiler-reached", result.stdout)

    def test_current_tree_is_rejected_as_a_reference(self):
        result = self.run_script(str(self.root))
        self.assertEqual(result.returncode, 2)
        self.assertIn("not the current source tree", result.stderr)
        self.assertNotIn("compiler-reached", result.stdout)


if __name__ == "__main__":
    unittest.main()
