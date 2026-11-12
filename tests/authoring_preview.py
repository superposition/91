#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Full-song preview differential on Linux. Never builds in the frozen tree.
Usage: python3 tests/authoring_preview.py FROZEN_SOURCE FM1_RS [OUTPUT]
Needs the standard native build dependencies and mido for the frozen MIDI sink.
"""
import array
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import wave
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
BASE=Path(sys.argv[1]).resolve();CLI=Path(sys.argv[2]).resolve()
OUT=Path(sys.argv[3]if len(sys.argv)>3 else ROOT/'build/authoring-preview').resolve();OUT.mkdir(parents=True,exist_ok=True)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==digest,name
sys.path.insert(0,str(BASE/'tools'))
import jam_preview
import mido
host=(BASE/'host/amb_host.c').read_text(encoding='utf-8')
cpu='uint32_t plat_cpu_pct(void) { return cpu_pct; }'
assert host.count(cpu)==1
host=host.replace(cpu,'uint32_t plat_cpu_pct(void) { return 0; } /* test-only deterministic CPU text */')
host=host.replace('"../firmware/src/', '"'+(BASE/'firmware/src').as_posix()+'/')
(OUT/'frozen_host.c').write_text(host,encoding='utf-8')
(OUT/'gen').mkdir(exist_ok=True)
subprocess.run(['python3',str(ROOT/'tools/gen_font.py'),str(OUT/'gen/felucca_font.h')],check=True,stdout=subprocess.DEVNULL)
subprocess.run(['cc','-O2','-ffp-contract=off','-std=c99','-DEN_HOST','-D_POSIX_C_SOURCE=200809L','-I'+str(BASE/'firmware/src'),'-I'+str(BASE/'firmware/src/dsp'),'-I'+str(OUT/'gen'),str(OUT/'frozen_host.c'),str(BASE/'firmware/src/dsp/amb.c'),'-lm','-o',str(OUT/'frozen-host')],check=True)
env=dict(os.environ);env.pop('AMB_LEGACY_BOOT',None)
results=[]
for song,stem in enumerate(['velvet-changes','afterhours','neon-turnaround']):
    for minor in [False,True]:
        case=f'{stem}-{int(minor)}';old=OUT/(case+'-old');new=OUT/(case+'-rust');old.mkdir(exist_ok=True);new.mkdir(exist_ok=True)
        bpm=[70,127,190][song];key=[11,4,7][song];bars=16;phrases=4;pack=BASE/'packs'/f'{stem}.json'
        # Exercise the frozen plan/file policy without invoking its build step.
        with patch.object(jam_preview.subprocess,'run'),patch.object(jam_preview,'midi_from_log'):
            jam_preview.preview(BASE,pack,old,bars,bpm,key,minor,phrases)
        reference=subprocess.run([str(OUT/'frozen-host'),'preview.amb','.'],cwd=old,env=env,capture_output=True)
        (old/'run.log').write_bytes(reference.stdout+reference.stderr);assert reference.returncode==0,reference.stderr
        jam_preview.midi_from_log(old/'midi_out.txt',old/'preview.mid',bpm)
        current=subprocess.run([str(CLI),'author','preview',str(pack),str(new),'--bars',str(bars),'--bpm',str(bpm),'--key',str(key),'--phrases',str(phrases)]+(['--minor']if minor else[]),cwd=ROOT,env=env,capture_output=True)
        (new/'run.log').write_bytes(current.stdout+current.stderr);assert current.returncode==0,current.stderr
        response=json.loads(current.stdout);assert Path(response['wav'])==new/'preview.wav'
        hashes={}
        for name in ['preview.jmp','preview.amb','preview.wav','preview.mid','preview.png','midi_out.txt']:
            a=(old/name).read_bytes();b=(new/name).read_bytes();assert a==b,(case,name);hashes[name]=hashlib.sha256(b).hexdigest()
        with wave.open(str(new/'preview.wav'))as wav:
            frames=wav.getnframes();pcm=array.array('h',wav.readframes(frames));peak=max(map(abs,pcm),default=0)
            assert wav.getnchannels()==2 and wav.getframerate()==44100 and peak<32767
        active={}
        for track in mido.MidiFile(new/'preview.mid').tracks:
            for message in track:
                if message.type in ('note_on','note_off'):
                    k=(message.channel,message.note)
                    if message.type=='note_on' and message.velocity:active[k]=active.get(k,0)+1
                    elif active.get(k,0):active[k]-=1
                elif message.type=='control_change' and message.control in (120,123):
                    for k in active:
                        if k[0]==message.channel:active[k]=0
        assert not any(active.values()),(case,'stuck MIDI notes',{k:v for k,v in active.items()if v})
        results.append(dict(case=case,bars=bars,phrases=phrases,bpm=bpm,key=key,minor=minor,frames=frames,peak_pcm16=peak,stuck_notes=0,sha256=hashes))
        print(case,frames,peak,flush=True)
# A process restart repeats all externally visible artifacts exactly.
first=OUT/'velvet-changes-0-rust';repeat=OUT/'repeat'
subprocess.run([str(CLI),'author','preview',str(BASE/'packs/velvet-changes.json'),str(repeat),'--bars','16','--bpm','70','--key','11','--phrases','4'],cwd=ROOT,env=env,check=True,capture_output=True)
for name in ['preview.jmp','preview.amb','preview.wav','preview.mid','preview.png','midi_out.txt']:assert(first/name).read_bytes()==(repeat/name).read_bytes(),name
report=dict(cases=results,frames=sum(r['frames']for r in results),source_files=440,cpu_text_override='test-only zero on frozen host; virtual Rust platform already returns zero',restart='exact',hardware_io=False)
(OUT/'preview-proof.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(cases=len(results),frames=report['frames'],restart='exact',hardware_io=False)))
