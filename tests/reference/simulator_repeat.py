#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Repeat complete-song/interaction/fuzz runs in fresh Rust processes."""
from pathlib import Path
import hashlib,json,os,re,subprocess,sys
out=Path(sys.argv[1]).resolve();repo=Path.cwd()
results=json.loads((out/'results.json').read_text())
lookup={row['scenario']:row for row in results['results']}
repeated=[]
for name in ['first','compat','restarts']:
    dest=out/'repeat'/name;dest.mkdir(parents=True,exist_ok=True)
    for old in dest.iterdir():assert old.is_file();old.unlink()
    if name=='first':source=repo/'tests/scenarios/first.amb'
    else:
        script=(out/f'{name}-rust.amb').read_text()
        script=script.replace(str((out/'runs'/name/'rust').relative_to(repo)),str(dest.relative_to(repo)))
        source=out/f'{name}-repeat.amb';source.write_text(script)
    env=dict(os.environ,AMB_LEGACY_BOOT='1',FM1_SIM_TRACE=str(dest/'platform.bin'),FM1_SIM_PCM=str(dest/'audio.pcm'))
    p=subprocess.run([str(out/'rust'),str(source),str(dest)],env=env,capture_output=True)
    normalized=re.sub(r'render [\d.]+% of real time','render <host-cost>% of real time',p.stdout.decode())
    artifacts={f.name:hashlib.sha256(f.read_bytes()).hexdigest()for f in sorted(dest.iterdir())}
    expected=lookup[name]
    assert p.returncode==expected['status']and not p.stderr,(name,p.returncode,p.stderr)
    assert hashlib.sha256(normalized.encode()).hexdigest()==expected['normalized_log_sha256'],name
    assert artifacts==expected['artifacts'],name
    repeated.append(dict(scenario=name,status=p.returncode,artifacts=artifacts))
(out/'repeat-results.json').write_text(json.dumps(repeated,indent=2)+'\n')
print('Fresh Rust processes reproduce song, interaction and129 damaged-store trials exactly.')
