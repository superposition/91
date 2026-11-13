#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare every native scenario's effects, not just a compile or exit status."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import hashlib,json,os,re,subprocess,sys

out=Path(sys.argv[1]).resolve()
repo=Path.cwd()
current_family=len(sys.argv)>2 and sys.argv[2]=='--current'
assert len(sys.argv)==2 or (len(sys.argv)==3 and current_family),'usage: simulator_compare.py OUT [--current]'
family='current-jmp3' if current_family else 'frozen-r01-factory'
flavors=['current-c0','current-c0-runner','current-rust'] if current_family else ['reference','c0','frozen-runner','c0-runner','rust']
known={c['name'][9:]:c for c in json.loads((repo/'docs/rust/baseline.json').read_text())['tests']if c['name'].startswith('scenario-')}
extra=json.loads((repo/'tests/simulator/expected.json').read_text())
scenarios=([repo/'tests/scenarios/jam-helper.amb']+sorted((repo/'tests/current-scenarios').glob('*.amb')) if current_family else
           sorted((repo/'tests/scenarios').glob('*.amb'))+sorted((repo/'tests/simulator').glob('*.amb')))
select=os.getenv('SIM_CURRENT_SCENARIOS' if current_family else 'SIM_SCENARIOS')
if select:scenarios=[p for p in scenarios if p.stem in select.split(',')]
assert scenarios,'no scenarios selected'
run_root=out/('current-runs' if current_family else 'runs')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def normalized(text):return re.sub(r'render [\d.]+% of real time','render <host-cost>% of real time',text)
def run(path):
    name=path.stem;results={}
    for flavor in flavors:
        dest=run_root/name/flavor;dest.mkdir(parents=True,exist_ok=True)
        for old in dest.iterdir():
            assert old.is_file(),old
            old.unlink()
        actual_path=path
        if path.parent.name=='simulator':
            script=path.read_text().replace('@DIR@',str(dest.relative_to(repo)))
            if name=='restarts':
                script+='\n'.join(f'storefuzz {n}\nseed {n}\nreboot\nwait 1\nexpect parsbad 0'for n in range(1,129))+'\n'
            actual_path=out/f'{name}-{flavor}.amb';actual_path.write_text(script)
        env=dict(os.environ,AMB_LEGACY_BOOT='1',FM1_SIM_TRACE=str(dest/'platform.bin'),FM1_SIM_PCM=str(dest/'audio.pcm'))
        p=subprocess.run([str(out/flavor),str(actual_path),str(dest)],capture_output=True,env=env)
        (dest/'stdout.txt').write_bytes(p.stdout);(dest/'stderr.txt').write_bytes(p.stderr)
        results[flavor]=dict(status=p.returncode,stdout=normalized(p.stdout.decode()),stderr=p.stderr.decode(),artifacts={p.name:sha(p)for p in sorted(dest.iterdir())if p.name not in ['stdout.txt','stderr.txt']})
    ref=results[flavors[0]]
    for flavor in flavors[1:]:
        current=results[flavor]
        assert current==ref,(name,flavor,{'expected':ref,'actual':current})
    failures=[line for line in ref['stdout'].splitlines()if line.startswith('FAIL')]
    if current_family:
        assert ref['status']==0 and not failures,(name,'current JMP3 scenario must pass',ref['status'],failures)
    elif name in known:
        assert ref['status']==(0 if known[name]['status']=='ok'else 1),(name,ref['status'],known[name])
        assert failures==known[name]['assertions'],(name,failures,known[name]['assertions'])
    else:
        assert ref['status']==extra[name]['status'],(name,ref['status'],extra[name])
        assert failures==extra[name]['assertions'],(name,failures,extra[name]['assertions'])
    pcm=run_root/name/flavors[-1]/'audio.pcm'
    summary=dict(scenario=name,fixture_family=family,status=ref['status'],historical=not current_family and name in known,failures=failures,audio_frames=pcm.stat().st_size//8,artifact_count=len(ref['artifacts']),artifact_bytes=sum(p.stat().st_size for p in (run_root/name/flavors[-1]).iterdir()if p.name not in ['stdout.txt','stderr.txt']),artifacts=ref['artifacts'],normalized_log_sha256=hashlib.sha256(ref['stdout'].encode()).hexdigest())
    return summary
jobs=int(os.getenv('PARITY_JOBS','4'));assert 1<=jobs<=8
with ThreadPoolExecutor(max_workers=jobs)as pool:
    results=[]
    for result in pool.map(run,scenarios):results.append(result);print(json.dumps(result),flush=True)
(out/('current-results.json' if current_family else 'results.json')).write_text(json.dumps(dict(fixture_family=family,flavors=flavors,cases=len(results),historical_cases=sum(r['historical']for r in results),known_failures=sum(r['status']==1 and r['historical']for r in results),artifact_count=sum(r['artifact_count']for r in results),results=results),indent=2)+'\n')
print(('Current JMP3 C/Rust' if current_family else 'Frozen R01 factory across all five implementations')+': scenario status, exact failure fingerprints, logs, PCM/WAV/MIDI/PNG and all bounded platform/object bytes match.',flush=True)
