#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Adapt only the protocol test harness to the independently frozen C sources."""
import hashlib
from pathlib import Path
import sys

repo = Path(__file__).resolve().parents[2]
baseline = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
assert output.is_relative_to(repo / "build"), "reference probe must stay inside build"
count = 0
for line in (repo / "tests/reference/source.sha256").read_text().splitlines():
    expected, relative = line.split(maxsplit=1)
    path = baseline / relative.lstrip(" *")
    assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, str(path)
    count += 1
probe = (repo / "tests/host/jam_usb_test.c").read_text()
for local, frozen in [
    ("../../host/amb_host.c", "host/amb_host.c"),
    ("../../firmware/src/app/jam_usb.h", "firmware/src/app/jam_usb.h"),
]:
    old = '#include "' + local + '"'
    new = '#include "' + (baseline / frozen).as_posix() + '"'
    assert probe.count(old) == 1
    if frozen.endswith("jam_usb.h"):
        new += "\n#define ju_owner() (ju.owner)"
    probe = probe.replace(old, new)
output.write_text(probe)
print(f"Verified all {count} frozen source hashes; generated test-only C adapter")
