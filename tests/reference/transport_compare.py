#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare independent real-instrument scenarios with bounded parallelism."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    out = Path(sys.argv[1])
    scenarios = [int(case) for case in sys.argv[2:]]
    jobs = int(os.environ.get("PARITY_JOBS", "1"))
    assert 1 <= jobs <= 8, "PARITY_JOBS must be between one and eight"

    def compare(scenario):
        for flavor in ("reference", "rust"):
            with (out / f"{flavor}-{scenario}.json").open("wb") as summary:
                subprocess.run([str(out / flavor), str(scenario),
                                str(out / f"{flavor}-{scenario}.trace")],
                               stdout=summary, check=True)
        # Read each trace once. Each worker's two instrument processes execute
        # sequentially; only entirely independent scenarios run concurrently.
        assert ((out / f"reference-{scenario}.trace").read_bytes() ==
                (out / f"rust-{scenario}.trace").read_bytes()), f"scenario {scenario} trace"
        reference = (out / f"reference-{scenario}.json").read_bytes()
        current = (out / f"rust-{scenario}.json").read_bytes()
        assert reference == current, f"scenario {scenario} summary"
        json.loads(current)  # Require a complete summary, including subprocess completion.
        return current.decode().strip()

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        for result in pool.map(compare, scenarios):
            print(result, flush=True)


if __name__ == "__main__":
    main()
