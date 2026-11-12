#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen recovery/debug policy oracle. All USB/MIDI/flash/time are fake."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import struct
import sys
import tempfile
import subprocess
import types

ROOT=Path(__file__).resolve().parents[1]
BASE=Path(sys.argv[1]).resolve()
CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    sha,name=line.split(maxsplit=1)
    assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==sha,name
sys.path.insert(0,str(ROOT/'tools'))
import fm1_rescue as current
import rust_tooling as tools


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module);return module


old=load('frozen_rescue',BASE/'tools/fm1_rescue.py')
sys.modules.setdefault('mido',types.ModuleType('mido'))
olddebug=load('frozen_debug',BASE/'tools/fm1_debug.py')
debug=load('maintained_debug',ROOT/'tools/fm1_debug.py')
cases=[];records=[]


def result(fn):
    try:return ['ok',fn()]
    except BaseException as e:return ['error',type(e).__name__,str(e)]


def check(name,want,got):
    if want!=got:
        raise AssertionError((name,want,got))
    cases.append(name)


for name in ['VID','PID','STOCK_SHA256','LOADER_URL','LOADER_SHA256','LOADER_ADDR','LOADER_ARG','FLASH_IDS','FLASH_SIZE','APP_END','WRITE_MIN','SECTOR','IO']:
    check('shared-rescue-metadata-'+name,getattr(old,name),getattr(current,name))


class Clock:
    def __init__(self):self.now=100.0
    def time(self):return self.now
    def sleep(self,n):self.now+=n
    def strftime(self,fmt):return fmt.replace('%Y%m%d-%H%M%S','20200102-030405')


class Flash:
    def __init__(self,module,clock,image,mode):
        self.m,self.clock,self.mode=module,clock,mode
        self.data=bytearray(image+bytes((i*13)&255 for i in range(module.FLASH_SIZE-len(image))))
        self.trace=[];self.reads=0;self.attempts={};self.ran=False
        self.original_upper=bytes(self.data[module.APP_END:])
        if mode!='same':
            for a in [0x4000,0x43000,0x92000]:self.data[a]^=3
        if mode=='protected':
            for a in [0,0x3000]:self.data[a]^=1
    def inquiry(self):
        self.trace.append(['inquiry'])
        return ('OTHER' if self.mode=='vendor' else 'WL82','UBOOT')
    def start_loader(self,blob):self.trace.append(['loader',len(blob),hashlib.sha256(blob).hexdigest()])
    def chip(self):
        self.trace.append(['chip'])
        return (0x1111 if self.mode=='key' else 0x980f,2 if self.mode=='type' else 3,0xb36014 if self.mode=='new-flash' else 0x111111 if self.mode=='flash' else 0x856014)
    def read(self,addr,n):
        self.trace.append(['read',addr,n]);self.clock.sleep(.125);self.reads+=1
        out=bytearray(self.data[addr:addr+n])
        if self.reads==2 and self.mode=='final-upper':out[0x93000]^=1
        if self.reads==2 and self.mode=='final-app':out[0x51000]^=1
        return bytes(out)
    def write_sector(self,addr,data):
        assert 0x4000<=addr<0x93000 and addr%4096==0 and len(data)==4096
        self.trace.append(['write',addr,hashlib.sha256(data).hexdigest()])
        self.attempts[addr]=self.attempts.get(addr,0)+1
        if self.mode=='io-error':raise OSError('transport gone')
        count=3 if self.mode=='retry-fail' else 2 if self.mode=='retry-twice' else 1 if self.mode=='retry-once' else 0
        if self.attempts[addr]<=count:
            self.data[addr:addr+512]=data[:512]
            raise self.m.Fail(f'sector {addr:#07x} did not verify')
        self.data[addr:addr+4096]=data
    def run_app(self):
        self.trace.append(['run_app']);self.ran=True
        if self.mode=='run-error':raise RuntimeError('reset interrupted')
    def cmd(self,cmd,args=b''):
        self.trace.append(['cmd',cmd,bytes(args).hex()]);return bytes(14)


image=bytes((i*37+i//4096)&255 for i in range(0x93000))
with tempfile.TemporaryDirectory(prefix='fm1-rescue-proof-') as directory:
    bad_loader=Path(directory)/'bad-loader.bin'
    for n in [0,1,512,1200]:
        bad_loader.write_bytes(bytes(n))
        check('reject-unpinned-loader-'+str(n),result(lambda:old.get_loader(str(bad_loader))),result(lambda:current.get_loader(str(bad_loader))))
    bad_loader.unlink()
    for mode in ['same','different','new-flash','vendor','key','type','flash','protected','retry-once','retry-twice','retry-fail','io-error','final-upper','final-app','run-error']:
        for write in [False,True]:
            outputs=[]
            for module in [old,current]:
                clock=Clock();module.time=clock;dev=Flash(module,clock,image,mode);said=[]
                say=lambda *a,**kw:said.append([list(a),kw])
                outcome=result(lambda:module.rescue(dev,image,b'fake encrypted loader',write,directory,say=say))
                path=Path(directory)/'fm1-backup-20200102-030405.bin'
                backup=path.read_bytes() if path.exists() else b''
                if path.exists():path.unlink()
                assert bytes(dev.data[0x93000:])==dev.original_upper
                outputs.append(dict(result=outcome,trace=dev.trace,said=said,backup=[len(backup),hashlib.sha256(backup).hexdigest()],flash=hashlib.sha256(dev.data).hexdigest(),ran=dev.ran))
            check(f'restore-{mode}-{write}',*outputs)
            record=outputs[0].copy();record['said']=[[ [str(x).replace(directory,'<backup>') for x in a],k]for a,k in record['said']]
            records.append(record)

for n in [0,1,2,31,32,55,56,63,64,65,127,128,1000,65536,1048576]:
    data=bytes((i*37)&255 for i in range(n))
    check(f'sha-{n}',hashlib.sha256(data).hexdigest(),tools.call('rescue.sha256',[list(data)]))
for cmd in [0,1,0xfb01,0xfb04,0xfd05,0xfc09,0xffff]:
    for n in range(15):
        args=bytes(range(n));check(f'cdb-{cmd}-{n}',old.cdb_for(cmd,args),current.cdb_for(cmd,args))


class Message:
    def __init__(self,type,data=()):self.type,self.data=type,tuple(data)


class Midi:
    def __init__(self,module,clock,mode,reply=()):self.module,self.clock,self.mode,self.reply=module,clock,mode,reply;self.trace=[];self.queue=[Message('sysex',[0])]
    def get_output_names(self):self.trace.append(['outputs']);return ['Other', 'fm-1 lowercase' if self.mode=='missing' else 'X0X 0' if self.mode=='x0x' else 'FM-1 0']
    def open_input(self,name):self.trace.append(['open_in',name]);return Port(self,'input')
    def open_output(self,name):
        self.trace.append(['open_out',name])
        if self.mode=='open-error':raise OSError('occupied')
        return Port(self,'output')
    Message=Message


class Port:
    def __init__(self,midi,kind):self.midi,self.kind=midi,kind
    def __enter__(self):self.midi.trace.append(['enter',self.kind]);return self
    def __exit__(self,*args):self.midi.trace.append(['close',self.kind])
    def poll(self):
        self.midi.trace.append(['poll'])
        return self.midi.queue.pop(0) if self.midi.queue else None
    def send(self,message):
        self.midi.trace.append(['send',list(message.data)])
        if self.midi.mode=='send-error':raise OSError('lost output')
        if self.midi.mode!='timeout':
            self.midi.queue.extend([Message('note_on'),Message('sysex',[0,1,2]),Message('sysex',[125,70,76,message.data[3],*self.midi.reply])])


for mode in ['normal','x0x','missing','timeout','open-error','send-error']:
    values=[]
    for module in [olddebug,debug]:
        clock=Clock();module.time=clock;midi=Midi(module,clock,mode,[1,2,3]);module.mido=midi
        outcome=result(lambda:module.ask(40,[0]*10,timeout=.01))
        values.append(dict(result=outcome,trace=midi.trace,time=round(clock.now,9)))
    check('debug-ask-'+mode,*values)
    records.append(values[0])

# Reports and request bytes execute the exact frozen CLI policy against fake ask.
for command,numbers in [('peek',[0x1c00000,16]),('poke',[0x10000,0xffffffff]),('clock',[2000000]),('clock',[0])]:
    for seed in range(20):
        rng=random.Random(seed);n=numbers[1]if command=='peek'else 3
        words=[rng.randrange(1,0xffffffff)for _ in range(n+1)]
        reply=[]
        for word in words:reply.extend(olddebug.w35(word))
        outputs=[]
        for module in [olddebug,debug]:
            sent=[];module.ask=lambda cmd,args,timeout=3:sent.append([cmd,args,timeout])or reply
            argv=sys.argv;sys.argv=['fm1_debug.py',command,*map(str,numbers)];out=io.StringIO()
            try:
                with contextlib.redirect_stdout(out):outcome=result(module.main)
            finally:sys.argv=argv
            outputs.append([outcome,out.getvalue(),sent])
        check(f'debug-report-{command}-{numbers}-{seed}',*outputs)

# Malformed reply errors already use #6's shared CodecError profile in the
# Python adapter. Execute the frozen presentation with that same retained byte
# profile to prove prefix/error preservation; raw R01 used IndexError here.
import ast
tree=ast.parse((BASE/'tools/fm1_debug.py').read_text())
node=next(n for n in tree.body if isinstance(n,ast.FunctionDef)and n.name=='main')
profile=dict(olddebug.__dict__);profile.update(w35=debug.w35,r35=debug.r35)
exec(compile(ast.Module(body=[node],type_ignores=[]),'frozen-report-shared-codec-profile','exec'),profile)
for size in range(16):
    reply=[0]*size;outputs=[]
    for function,namespace in [(profile['main'],profile),(debug.main,debug.__dict__)]:
        namespace['ask']=lambda *a,**kw:reply
        argv=sys.argv;sys.argv=['fm1_debug.py','peek','0x10000','2'];out=io.StringIO()
        try:
            with contextlib.redirect_stdout(out):outcome=result(function)
        finally:sys.argv=argv
        outputs.append([outcome,out.getvalue()])
    check('short-debug-retained-codec-profile-'+str(size),*outputs)

for command,a,b in [('peek',0x10000,2),('poke',-1,0xffffffff),('clock',2000000,0)]:
    expected=tools.call('debug.plan',[command,a,b])
    completed=subprocess.run([CLI,'tools','debug-plan',command,str(a),str(b)],capture_output=True,check=True)
    check('native-debug-plan-'+command,expected,json.loads(completed.stdout))

stock=Path(os.environ['FM1_STOCK_PACKAGE'])if 'FM1_STOCK_PACKAGE'in os.environ else ROOT.parent.parent/'refs/fm1_open/sdk/firmware/FM-1_015.fwsc'
stock_positive=stock.exists()
if stock_positive:
    raw=stock.read_bytes();assert hashlib.sha256(raw).hexdigest()==old.STOCK_SHA256
    image=old.package_flash_image(raw)
    check('actual-stock-pin-and-flash-image',image,bytes(tools.call('rescue.stock',[list(raw),str(stock)])))
    with tempfile.TemporaryDirectory(prefix='fm1-stock-plan-')as folder:
        snapshot=Path(folder)/'flash.bin';snapshot.write_bytes(image+bytes(0x100000-len(image)))
        completed=subprocess.run([CLI,'tools','rescue-plan',str(stock),str(snapshot)],capture_output=True,check=True)
        plan=json.loads(completed.stdout)
        check('native-stock-plan-no-write',dict(changed_sectors=[],write_order=[],protected_head_bytes=0x4000,preserved_upper_bytes=0x100000-0x93000,writes_performed=0),plan)

print(json.dumps(dict(cases=len(cases),case_names_sha256=hashlib.sha256(json.dumps(cases,separators=(',',':')).encode()).hexdigest(),traces_sha256=hashlib.sha256(json.dumps(records,separators=(',',':'),sort_keys=True).encode()).hexdigest(),actual_stock_pin=stock_positive,hardware_access=False),indent=2))
