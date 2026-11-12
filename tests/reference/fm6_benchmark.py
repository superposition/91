#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Host release costs include the real fixed-queue/FIFO/output adapters."""
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
out=Path(sys.argv[1]);cases=[]
os.sched_setaffinity(0,{min(os.sched_getaffinity(0))})
for mode in range(5):
    pair={}
    for flavor in ['reference','c0','rust']:
        runs=[json.loads(subprocess.check_output([str(out/flavor),str(mode),'5000'])) for _ in range(3)]
        pair[flavor]=dict(frames=runs[0]['frames'],hash=runs[0]['hash'],median_nanoseconds=statistics.median(x['nanoseconds'] for x in runs),nanoseconds_per_frame=statistics.median(x['nanoseconds']/x['frames'] for x in runs),initialization_nanoseconds=[x['init_nanoseconds'] for x in runs],trials=[x['nanoseconds'] for x in runs])
    assert pair['reference']['hash']==pair['c0']['hash']==pair['rust']['hash']
    cases.append(dict(mode=mode,costs=pair,rust_over_c=pair['rust']['median_nanoseconds']/pair['reference']['median_nanoseconds']))
report=dict(scope='WSL host release, one CPU affinity; real 256-frame job, queue/span/output adapters and periodic six/five-plus-one retrigger; no physical timing claim',cases=cases)
(out/'benchmark.json').write_text(json.dumps(report,indent=2)+'\n')
print('FM6 host release ratios: '+json.dumps({x['mode']:round(x['rust_over_c'],3) for x in cases}))
