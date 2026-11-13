# SPDX-License-Identifier: GPL-3.0-only
"""The Rust guard covers a domain where the frozen C helper has no defined result."""
from pathlib import Path
import json,signal,subprocess,sys
p=subprocess.run([sys.argv[1]],capture_output=True)
assert p.returncode and b'division by zero' in p.stderr,(p.returncode,p.stderr)
q=subprocess.run([sys.argv[3]],capture_output=True)
assert q.returncode==-signal.SIGFPE,(q.returncode,q.stderr)
proof=dict(reference='independently extracted R01 tap_tempo',same_clock_taps=2,
           undefined_behavior='integer division by zero',sanitizer='GCC undefined',
           returncode=p.returncode,diagnostic=p.stderr.decode().strip(),
           plain_C_signal='SIGFPE',plain_C_returncode=q.returncode)
Path(sys.argv[2]).write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8')
print(json.dumps(proof))
