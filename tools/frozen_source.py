#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Reconstruct the hash-pinned R01 comparison source from its Git checkpoint."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = "43db648fb9b31f944b7f29fb24e70f152a3b5b13"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    destination = args.destination.resolve()
    expected = []
    for line in (ROOT / "tests/reference/source.sha256").read_text().splitlines():
        digest, name = line.split(maxsplit=1)
        path = PurePosixPath(name)
        if path.is_absolute() or ".." in path.parts:
            raise ValueError("unsafe reference path: " + name)
        expected.append((digest, name))
    archive = subprocess.check_output(
        ["git", "archive", "--format=tar", REFERENCE], cwd=ROOT
    )
    files = []
    restored_crlf = []
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        for digest, name in expected:
            member = source.getmember(name)
            if not member.isfile():
                raise ValueError("reference is not a regular file: " + name)
            data = source.extractfile(member).read()
            if hashlib.sha256(data).hexdigest() != digest:
                # Restore captured line endings only when the pinned hash
                # proves every resulting byte. Never adjust source content.
                data = data.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
                if hashlib.sha256(data).hexdigest() != digest:
                    raise ValueError("reference hash mismatch: " + name)
                restored_crlf.append(name)
            target = destination / name
            if target.exists() and target.read_bytes() != data:
                raise ValueError("destination contains different data: " + str(target))
            files.append((target, data))
    # Validate the entire archive and destination before writing any file.
    for target, data in files:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    print(json.dumps({"reference": REFERENCE, "files": len(files),
                      "restored_crlf": restored_crlf, "destination": str(destination)}))


if __name__ == "__main__":
    main()
