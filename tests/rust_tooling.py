#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Run the frozen tooling gate on Windows/Linux without device or audio I/O."""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    baseline = parser.parse_args().baseline.resolve()
    target = ROOT / "build/tooling" / ("windows" if os.name == "nt" else "linux")
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.94.1")
    subprocess.run(["cargo", "+1.94.1", "build", "--release", "--locked",
                    "-p", "fm1-cli", "--target-dir", str(target)],
                   cwd=ROOT, env=env, check=True)
    executable = target / "release" / ("fm1-rs.exe" if os.name == "nt" else "fm1-rs")
    env.update(FM1_TOOLING_CLI=str(executable), FM1_CODEC_CLI=str(executable))
    commands = [
        [sys.executable, "tests/tooling_differential.py", str(baseline), str(executable)],
        [sys.executable, "tests/install_test.py"],
        [sys.executable, "tools/fm1log.py", "--selftest"],
    ]
    for command in commands:
        subprocess.run(command, cwd=ROOT, env=env, check=True)


if __name__ == "__main__":
    main()
