#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen full-song acceptance versus Rust policy using deterministic I/O fakes.

No MIDI/audio ports are opened. Usage: python tests/authoring_acceptance.py
FROZEN_SOURCE FM1_RS [OUTPUT]. Requires NumPy for the independent old reporter.
"""
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import types
from unittest.mock import patch
import numpy as np
import mido

ROOT=Path(__file__).resolve().parents[1]
BASE=Path(sys.argv[1]).resolve()
CLI=Path(sys.argv[2]).resolve()
OUT=Path(sys.argv[3] if len(sys.argv)>3 else ROOT/'build/authoring-acceptance')
OUT.mkdir(parents=True,exist_ok=True)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==digest,name
os.environ['FM1_AUTHORING_CLI']=str(CLI)
sys.path.insert(0,str(ROOT/'tools'))
import rust_authoring


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


class Fake:
    def __init__(self,folder,scenario):
        self.folder,self.scenario=folder,scenario
        self.now=1000.0;self.start=self.now;self.calls=[];self.callback=None;self.blocks=0;self.song=0;self.reads=0
        self.state=dict(xruns=0,key=4,minor=1,bars=8,hold=1,energy=67,crc32='song0',pending=0)
        self.queue=[]
        fake=self
        class Input:
            def iter_pending(self):
                while fake.queue:yield fake.queue.pop(0)
            def close(self):fake.calls.append(('close',fake.now))
        class Output:
            def send(self,msg):fake.calls.append(('send',fake.now,list(msg.data)))
        class Device:
            def __init__(self):self.inp=Input();self.out=Output()
            def __enter__(self):fake.calls.append(('open',fake.now));return self
            def __exit__(self,*args):self.inp.close()
            def status(self):
                fake.calls.append(('status',fake.now))
                elapsed=fake.now-fake.start
                row=dict(ms=int(fake.now*1000),cpu=12,cpu1=7,max_us=1250,shed=0,xruns=0,late=0,crashes=0)
                if fake.scenario=='xruns' and elapsed>590:fake.state['xruns']=1
                if fake.scenario=='late' and elapsed>590:row['late']=1
                if fake.scenario=='shed':row['shed']=1
                if fake.scenario=='key' and elapsed>590:fake.state['key']=5
                fake.queue.append(types.SimpleNamespace(type='sysex',data=row))
                list(self.inp.iter_pending())
                return copy.deepcopy(fake.state)
            def read(self):fake.calls.append(('read',fake.now));return b'original'+bytes([fake.song])
            def features(self):fake.calls.append(('features',fake.now));return dict(factory_songs=2 if fake.scenario=='features' else 3)
            def song_status(self):
                fake.calls.append(('song-status',fake.now));fake.reads+=1
                return dict(pack_version=1 if fake.scenario=='timeout' or (fake.scenario=='delayed' and fake.reads<3) else 2,loading=0)
            def request(self,command,data=()):fake.calls.append(('request',fake.now,command,list(data)))
            def control(self,field,value):
                fake.calls.append(('control',fake.now,field,value))
                if fake.scenario=='control' and field=='brightness':raise RuntimeError('control failed')
                if fake.scenario=='cleanup' and field=='energy':raise RuntimeError('cleanup failed')
                if field=='song':
                    fake.song=value
                    if fake.scenario!='songs':fake.state['crc32']='song'+str(value)
                if field=='key':fake.state['key']=value%12;fake.state['minor']=value//12
                elif field in fake.state:fake.state[field]=value
            def audition(self,active):fake.calls.append(('audition',fake.now,list(active)))
        self.Device=Device
        class Stream:
            def __init__(self,**kwargs):self.callback=kwargs['callback'];fake.calls.append(('recorder',fake.now,{k:v for k,v in kwargs.items()if k!='callback'}))
            def __enter__(self):fake.callback=self.callback;return self
            def __exit__(self,*args):fake.callback=None
        self.Stream=Stream
    def sleep(self,seconds):
        if self.callback:
            values=((np.arange(64,dtype='int32')+self.blocks*73)%65536-32768).astype('int16').reshape(32,2)
            self.callback(values,32,None,'overflow' if self.scenario=='flags' else None);self.blocks+=1
        self.now+=seconds
        if self.scenario=='stop' and self.now-self.start>=181:(self.folder/'stop').write_text('')


def run(source,folder,scenario,seconds):
    fake=Fake(folder,scenario)
    sd=types.SimpleNamespace(InputStream=fake.Stream)
    log=types.SimpleNamespace(decode=lambda data:None if scenario=='telemetry' else data,
                              decode_extra=lambda data:None,decode_diag=lambda data:None)
    modules={'sounddevice':sd,'jam_midi':types.SimpleNamespace(Device=fake.Device),'fm1log':log}
    output=io.StringIO();code=0
    with patch.dict(sys.modules,modules),patch('time.monotonic',lambda:fake.now),patch('time.sleep',fake.sleep),patch.object(sys,'argv',['songbook_accept.py','--seconds',str(seconds),'--audio-device','17','--out',str(folder)]),contextlib.redirect_stdout(output):
        module=load('accept_'+scenario+'_'+str(len(source.as_posix())),source)
        try:module.main()
        except SystemExit as error:code=error.code or 0
    reports={p.name:json.loads(p.read_text(encoding='utf-8')) for p in folder.glob('*.json')}
    return dict(calls=fake.calls,exit=code,reports=reports,wav=(folder/'device-songbook.wav').read_bytes(),active=(folder/'active-before.jmp').read_bytes(),stdout=output.getvalue())


results=[]
with tempfile.TemporaryDirectory() as td:
    td=Path(td)
    for scenario,seconds in [('normal',600),('delayed',600),('short',20),('features',600),('timeout',600),('telemetry',600),('xruns',600),('late',600),('shed',600),('flags',600),('songs',600),('key',600),('stop',600),('control',600),('cleanup',600)]:
        a=td/(scenario+'-old');b=td/(scenario+'-rust');a.mkdir();b.mkdir()
        expected=run(BASE/'tools/songbook_accept.py',a,scenario,seconds)
        actual=run(ROOT/'tools/songbook_accept.py',b,scenario,seconds)
        assert actual['calls']==expected['calls'],(scenario,'callback order',next(((i,x,y)for i,(x,y)in enumerate(zip(actual['calls'],expected['calls']))if x!=y),None))
        for key in ['exit','reports','wav','active']:
            if actual[key]!=expected[key]:
                (OUT/'failure.json').write_text(json.dumps(dict(scenario=scenario,key=key,expected=expected['reports'],actual=actual['reports']),indent=2),encoding='utf-8')
                raise AssertionError((scenario,key))
        results.append(dict(scenario=scenario,exit=actual['exit'],calls=len(actual['calls']),frames=(len(actual['wav'])-44)//4,errors=actual['reports']['report.json']['errors'],wav_sha256=hashlib.sha256(actual['wav']).hexdigest()))
    # Numeric report controls, including signed minimum and a large reduction
    # crossing float64's exact-integer range with NumPy's independent tree.
    reductions=[]
    for count in [0,2,14,16,126,128,130,254,256,258,1024,16777218]:
        values=((np.arange(count,dtype='int64')*32749)%65536-32768).astype('int16')
        raw=td/'raw.pcm';raw.write_bytes(values.tobytes());folder=td/('rms-'+str(count))
        request=dict(folder=str(folder),pcm=str(raw),before=None,after=None,errors=[],flags=[],events=[],telemetry=[],trace=[],extra=[],diag=[])
        report=rust_authoring.call('accept-finalize',**request)
        rms=float(np.sqrt(np.mean(values.astype('float64')**2)))if count else 0.0
        peak=int(np.abs(values.astype('int32')).max())if count else 0
        assert report['rms_pcm16']==rms,(count,report['rms_pcm16'],rms)
        assert report['peak_pcm16']==peak
        reductions.append(dict(samples=count,rms=rms,peak=peak))
rust_authoring.close()
report=dict(scenarios=results,reduction_controls=reductions,hardware_io=False,callback_order='exact',json_reports='exact',wav_bytes='exact')
(OUT/'acceptance.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(scenarios=len(results),reduction_controls=len(reductions),virtual_frames=sum(r['frames']for r in results),hardware_io=False)))
