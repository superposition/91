#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise shipped Python entry points and the useful Rust offline CLI.
Usage: python tests/authoring_cli.py FROZEN_SOURCE FM1_RS [OUTPUT]
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=Path(sys.argv[2]).resolve()
OUT=Path(sys.argv[3]if len(sys.argv)>3 else ROOT/'build/authoring-cli');OUT.mkdir(parents=True,exist_ok=True)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==digest,name
env=dict(os.environ,FM1_AUTHORING_CLI=str(CLI),FM1_CODEC_CLI=str(CLI),FM1_AUTHOR_ROOT=str(BASE));results=[]
os.environ['FM1_AUTHORING_CLI']=os.environ['FM1_CODEC_CLI']=str(CLI)
def run(command):return subprocess.run(command,cwd=ROOT,env=env,capture_output=True)
with tempfile.TemporaryDirectory()as td:
    td=Path(td)
    for song in range(4):
        command=['factory']if song==0 else['factory-song','--song',str(song-1)]
        values=[]
        for name,entry in [('frozen',[sys.executable,str(BASE/'tools/fm1_jam.py')]),('python',[sys.executable,str(ROOT/'tools/fm1_jam.py')]),('rust',[str(CLI),'author'])]:
            output=td/f'{name}-{song}.json';result=run(entry+command+[str(output)])
            assert result.returncode==0,(name,result.stderr);values.append(json.loads(output.read_text(encoding='utf-8')))
        assert values[0]==values[1]==values[2]
        results.append(dict(command='factory-'+str(song),exit=0))
        outputs=[]
        for name,entry in [('frozen',[sys.executable,str(BASE/'tools/fm1_jam.py')]),('python',[sys.executable,str(ROOT/'tools/fm1_jam.py')]),('rust',[str(CLI),'author'])]:
            pack=td/f'{name}-{song}.jmp';header=td/f'{name}-{song}.h';decoded=td/f'{name}-{song}-decoded.json'
            result=run(entry+['compile',str(td/f'frozen-{song}.json'),str(pack),'--c-header',str(header)])
            assert result.returncode==0,(name,result.stderr)
            summary=json.loads(result.stdout)
            valid=run(entry+['validate',str(pack)]);assert valid.returncode==0;assert json.loads(valid.stdout)==summary
            decoded_result=run(entry+['decode',str(pack),str(decoded)]);assert decoded_result.returncode==0;assert json.loads(decoded_result.stdout)==summary
            outputs.append(dict(pack=pack.read_bytes(),header=header.read_text(encoding='utf-8'),decoded=json.loads(decoded.read_text(encoding='utf-8')),summary=summary))
        assert outputs[0]==outputs[1]==outputs[2]
        results.append(dict(command='compile/decode/validate/header-'+str(song),exit=0,sha256=hashlib.sha256(outputs[0]['pack']).hexdigest()))
    for bad in ['{}','{"version":3}','{broken json']:
        path=td/'bad.json';path.write_text(bad)
        for source in [BASE,ROOT]:
            result=run([sys.executable,str(source/'tools/fm1_jam.py'),'compile',str(path),str(td/'bad.jmp')]);assert result.returncode==1 and result.stderr
        result=run([str(CLI),'author','compile',str(path),str(td/'bad.jmp')]);assert result.returncode==2 and result.stderr
        results.append(dict(command='invalid-'+bad,python_exit=1,rust_exit=2))
    # An unsuccessful request must leave the serialized binding usable.
    good=json.loads((td/'frozen-0.json').read_text());bad=dict(good,version=3)
    sys.path.insert(0,str(ROOT/'tools'));import rust_authoring
    try:rust_authoring.call('compile',document=bad)
    except ValueError:pass
    else:raise AssertionError('bad version accepted')
    assert bytes(rust_authoring.call('compile',document=good))==(td/'frozen-0.jmp').read_bytes()
    rust_authoring.close()
report=dict(cases=results,python_cli='compatible flags/JSON semantics/pack and header output/statuses',native_cli='documented offline commands; errors use existing fm1-rs exit2',stream_recovery='pass',hardware_io=False)
(OUT/'cli.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps(dict(cases=len(results),stream_recovery='pass',hardware_io=False)))
