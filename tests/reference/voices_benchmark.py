#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Release host costs, including block adapters, clears and periodic retrigger.

These timings do not qualify the physical target's stack, RAM or deadlines.
"""
import json
from pathlib import Path
import statistics
import subprocess
import sys

out = Path(sys.argv[1])
cases = [int(value) for value in sys.argv[2:]] or [*range(7), 97, 100, 113, 135]
results = []
for case in cases:
    pair = {}
    for flavor in ["reference", "rust"]:
        runs = [json.loads(subprocess.check_output([str(out / flavor), str(case), "bench", "10000"]))
                for _ in range(3)]
        pair[flavor] = dict(
            blocks=runs[0]["blocks"], frames=runs[0]["frames"],
            median_nanoseconds=statistics.median(run["nanoseconds"] for run in runs),
            nanoseconds_per_frame=statistics.median(run["nanoseconds"] / run["frames"] for run in runs),
            trials=[run["nanoseconds"] for run in runs])
    results.append(dict(test=case, costs=pair,
                        rust_over_c=pair["rust"]["median_nanoseconds"] / pair["reference"]["median_nanoseconds"]))
report = dict(scope="WSL host release; 128-frame block including clears and periodic retrigger; no hardware deadline claim", cases=results)
(out / "benchmark.json").write_text(json.dumps(report, indent=2) + "\n")
print("Voice host release measurements: " + json.dumps({r["test"]: round(r["rust_over_c"], 3) for r in results}))
