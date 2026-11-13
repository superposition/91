#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen Device transaction/timeout/error policy versus safe Rust action plans.
Usage: python tests/authoring_device.py FROZEN_SOURCE FM1_RS [OUTPUT]
Only in-memory MIDI/clock adapters are used; no ports or flash are accessed.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys
import types
import zlib
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=Path(sys.argv[2]).resolve()
OUT=Path(sys.argv[3]if len(sys.argv)>3 else ROOT/'build/authoring-device');OUT.mkdir(parents=True,exist_ok=True)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==digest,name
os.environ['FM1_AUTHORING_CLI']=os.environ['FM1_CODEC_CLI']=str(CLI)
sys.path.insert(0,str(ROOT/'tools'))
import jam_midi as current
import rust_authoring
spec=importlib.util.spec_from_file_location('frozen_jam_midi',BASE/'tools/jam_midi.py');old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)
assert old.ERRORS==current.ERRORS
assert all(current.FIELDS[name]==value for name,value in old.FIELDS.items())
assert {name:value for name,value in current.FIELDS.items() if name not in old.FIELDS}=={'keys_octave':22,'bass_octave':23}
results=[]


def outcome(fn):
    try:
        v=fn();return dict(ok=True,value=list(v)if isinstance(v,bytes)else v)
    except Exception as e:return dict(ok=False,kind=type(e).__name__,message=str(e))


def compare(name,run):
    expected=run(old);actual=run(current)
    if actual!=expected:
        (OUT/'failure.json').write_text(json.dumps(dict(case=name,expected=expected,actual=actual),indent=2),encoding='utf-8')
        raise AssertionError((name,expected,actual))
    results.append(dict(case=name,accepted=actual['result']['ok'],callbacks=len(actual['calls']),fingerprint=hashlib.sha256(json.dumps(actual,sort_keys=True).encode()).hexdigest()))


class Clock:
    def __init__(self):self.now=1000.0;self.sleeps=[]
    def sleep(self,n):self.sleeps.append(n);self.now+=n


def fake_instance(module):
    return object.__new__(module.Device)


def replies(status=0,raw=b'',token=31,op=8):
    body=[op,token,status,*[v for n in raw for v in(n>>4,n&15)]]
    return [125,80,97,*body,(-sum(body))&127]


for names,port in [([],None),(['Other'],None),(['FM-1 A','FM1 B'],None),(['m-vave fM-1'],None),(['A','FM-1'],''),(['My Keys','Other'],'KEYS'),(['FM-1 input','FM-1 output'],'output')]:
    def run(module,names=names,port=port):
        calls=[];clock=Clock()
        class Port:
            def close(self):calls.append('close')
        midi=types.SimpleNamespace(get_input_names=lambda:(calls.append('inputs')or names),get_output_names=lambda:(calls.append('outputs')or names),open_input=lambda n:(calls.append(['input',n])or Port()),open_output=lambda n:(calls.append(['output',n])or Port()))
        def construct():
            with module.Device(port):pass
        with patch.dict(sys.modules,{'mido':midi}),patch.object(module.secrets,'randbelow',lambda n:30):
            result=outcome(construct)
        return dict(result=result,calls=calls)
    compare('ports-'+str((names,port)),run)


for case in ['normal','timeout','wrong-token','mixed','checksum','nibble']+['status-'+str(n)for n in range(1,9)]:
    def run(module,case=case):
        calls=[];clock=Clock();queue=[];dev=fake_instance(module);dev.token=31
        class Input:
            def iter_pending(self):
                calls.append(['poll',clock.now])
                while queue:
                    msg=queue.pop(0);calls.append(['message',msg.type,list(getattr(msg,'data',[]))]);yield msg
        class Output:
            def send(self,message):
                calls.append(['send',clock.now,list(message.data)])
                data=replies(raw=b'abc')
                if case=='timeout':return
                if case=='wrong-token':data=replies(raw=b'abc',token=32)
                if case=='checksum':data[-1]^=1
                if case=='nibble':data[-2]=16;data[-1]=(-sum(data[3:-1]))&127
                if case.startswith('status-'):data=replies(status=int(case.split('-')[1]))
                if case=='mixed':queue.extend([types.SimpleNamespace(type='note_on'),types.SimpleNamespace(type='sysex',data=replies(token=99))])
                queue.append(types.SimpleNamespace(type='sysex',data=data))
                if case=='mixed':queue.append(types.SimpleNamespace(type='sysex',data=replies(raw=b'trailing')))
        dev.inp=Input();dev.out=Output();dev.mido=types.SimpleNamespace(Message=lambda kind,**kw:types.SimpleNamespace(type=kind,**kw))
        with patch.object(module.time,'monotonic',lambda:clock.now),patch.object(module.time,'sleep',clock.sleep):result=outcome(lambda:dev.request(8,timeout=.02))
        calls.append(['sleeps',clock.sleeps]);calls.append(['remaining',len(queue)]);return dict(result=result,calls=calls)
    compare('request-'+case,run)


for method,body in [('capabilities',bytes([1,0,15,5,26,30])),('features',bytes([3,8,3,6])),('song_status',bytes([2,2,0])),('status',bytes(range(25)))]:
    for length in [0,len(body)-1,len(body),len(body)+1]:
        def run(module,method=method,body=body,length=length):
            calls=[];dev=fake_instance(module)
            def request(op,data=(),timeout=3):calls.append([op,list(data),timeout]);return(body+b'\0')[:length]
            dev.request=request;return dict(result=outcome(lambda:getattr(dev,method)()),calls=calls)
        compare('decode-'+str((method,length)),run)
for error in [RuntimeError(old.ERRORS[1]),RuntimeError('another error'),ConnectionError('OS lost port')]:
    def run(module,error=error):
        dev=fake_instance(module);calls=[]
        def request(op,data=(),timeout=3):calls.append([op,list(data),timeout]);raise error
        dev.request=request;return dict(result=outcome(dev.features),calls=calls)
    compare('features-error-'+repr(error),run)


for length in [0,1,63,64,65,255,256,257,3840]:
    for fault in ['none','truncated','total','offset','length']:
        def run(module,length=length,fault=fault):
            calls=[];dev=fake_instance(module);body=bytes((n*37)&255 for n in range(length))
            def request(op,data=(),timeout=3):
                calls.append([op,list(data),timeout]);off=data[1]*64;raw=struct.pack('<HH',length,off)+body[off:off+64]
                if fault=='truncated':return raw[:3]
                if fault=='total':return struct.pack('<HH',3841,off)+body[off:off+64]
                if fault=='offset':return struct.pack('<HH',length,off+1)+body[off:off+64]
                if fault=='length':return raw+b'\0'
                return raw
            dev.request=request;return dict(result=outcome(lambda:dev.read(4)),calls=calls)
        compare('read-'+str((length,fault)),run)


for length in [0,1,255,256,257,3840,16383]:
    for fault in ['none','ack','begin','chunk','commit','activate','abort']:
        def run(module,length=length,fault=fault):
            calls=[];dev=fake_instance(module);body=bytes((n*37)&255 for n in range(length))
            def request(op,data=(),timeout=3):
                calls.append([op,list(data),timeout])
                if op=={'begin':3,'chunk':4,'commit':5,'activate':6,'abort':4}.get(fault):raise ConnectionError('wire lost')
                if fault=='abort' and op==9:raise OSError('abort failed')
                if op==4:return bytes([(data[0]+1)&255 if fault=='ack' else data[0]])
                return b''
            dev.request=request;return dict(result=outcome(lambda:dev.audition(body)),calls=calls)
        compare('audition-'+str((length,fault)),run)


for mode in ['active','pending','crc','boundary']:
    def run(module,mode=mode):
        calls=[];clock=Clock();dev=fake_instance(module);body=b'payload'
        def status():
            calls.append(['status',clock.now]);return dict(pending=int(mode=='pending'),crc32=f'{zlib.crc32(body):08x}'if mode in ['active','pending']or(mode=='boundary'and clock.now>=1000.5)else'00000000')
        dev.status=status
        with patch.object(module.time,'monotonic',lambda:clock.now),patch.object(module.time,'sleep',clock.sleep):result=outcome(lambda:dev.wait_active(body,timeout=.5))
        calls.append(['sleeps',clock.sleeps]);return dict(result=result,calls=calls)
    compare('activation-'+mode,run)


for fault in ['none','playing','short','ack-crc','before-crc','readback']:
    def run(module,fault=fault):
        calls=[];dev=fake_instance(module);body=b'payload';crc=zlib.crc32(body)
        def status():calls.append('status');return dict(playing=int(fault=='playing'),crc32=f'{0 if fault=="before-crc" else crc:08x}')
        def request(op,data=(),timeout=3):calls.append([op,list(data),timeout]);return b'x'if fault=='short'else struct.pack('<I',crc^int(fault=='ack-crc'))
        def read(slot):calls.append(['read',slot]);return b'changed'if fault=='readback'else body
        dev.status=status;dev.request=request;dev.read=read
        return dict(result=outcome(lambda:dev.save(4)),calls=calls)
    compare('save-'+fault,run)
for field in list(old.FIELDS)+['invalid']:
    def run(module,field=field):
        calls=[];clock=Clock();dev=fake_instance(module)
        def request(op,data=(),timeout=3):calls.append([op,list(data),timeout]);return b''
        dev.request=request;dev.status=lambda:(calls.append('status')or dict(ok=1))
        with patch.object(module.time,'sleep',clock.sleep):result=outcome(lambda:dev.control(field,65))
        calls.append(['sleeps',clock.sleeps]);return dict(result=result,calls=calls)
    compare('control-'+field,run)
rust_authoring.close()
report=dict(cases=len(results),accepted=sum(r['accepted']for r in results),rejected=sum(not r['accepted']for r in results),hardware_io=False,results=results)
(OUT/'device.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print(json.dumps({k:v for k,v in report.items()if k!='results'}))
