#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""USB capture/retry/keepalive policy with descriptor and endpoint doubles."""
import ast
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import types

ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI);sys.path.insert(0,str(ROOT/'tools'))
import fm1_rescue as current
import rust_tooling as tools
spec=importlib.util.spec_from_file_location('frozen_capture',BASE/'tools/fm1_rescue.py');old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old)
# Reuse the independently qualified endpoint/NOR double without executing its
# scenario driver. The fake transport uses only the frozen Python wire maths.
tree=ast.parse((ROOT/'tests/rescue_bulk.py').read_text());selected=[n for n in tree.body if isinstance(n,ast.ClassDef)and n.name in ('USBError','Clock','Transport')]
scope=dict(old=old,hashlib=hashlib,struct=struct,INITIAL_FLASH=bytes((i*37+i//4096)&255 for i in range(0x100000)))
exec(compile(ast.Module(body=selected,type_ignores=[]),'shared-virtual-endpoints','exec'),scope)
Transport,USBError=scope['Transport'],scope['USBError']
cases=[];records=[]
class Cancel(BaseException):pass


def check(name,a,b):assert a==b,(name,a,b);cases.append(name)


def result(fn):
    try:return ['ok',fn()]
    except BaseException as e:return ['error',type(e).__name__,str(e)]


class Clock:
    def __init__(self):self.now=100.0
    def time(self):return self.now
    def sleep(self,n):self.now+=n
    def strftime(self,fmt):return fmt.replace('%Y%m%d-%H%M%S','20200102-030405')


class Endpoint:
    def __init__(self,t,address):self.t,self.bEndpointAddress,self.bmAttributes=t,address,2
    def write(self,*a):return self.t.write(*a)
    def read(self,*a):return self.t.read(*a)


class Device:
    def __init__(self,usb,transport):self.usb,self.t,self.configured=usb,transport,False;self.ep_in=Endpoint(transport,129);self.ep_out=Endpoint(transport,2)
    def get_active_configuration(self):
        self.usb.trace.append(['configuration'])
        if not self.configured:raise USBError('not configured')
        return Config(self)
    def set_configuration(self):self.usb.trace.append(['set_configuration']);self.configured=True
    def is_kernel_driver_active(self,index):self.usb.trace.append(['active',index]);return True
    def detach_kernel_driver(self,index):
        self.usb.trace.append(['detach',index])
        if self.usb.mode=='detach':raise USBError(5,'detach failed')
    def ctrl_transfer(self,*a,**kw):
        self.usb.trace.append(['reset',list(a),kw])
        if self.usb.mode=='reset-error':raise USBError('reset failed')
    def clear_halt(self,endpoint):self.usb.trace.append(['clear',endpoint.bEndpointAddress])


class Config:
    def __init__(self,device):self.device=device
    def __getitem__(self,index):assert index==(0,0);return Interface(self.device)


class Interface:
    bInterfaceNumber=0
    def __init__(self,device):self.device=device
    def __iter__(self):return iter([self.device.ep_out,self.device.ep_in])


class USB:
    def __init__(self,module,mode):
        self.mode,self.trace,self.calls,self.claims=mode,[],0,0
        self.t=Transport(module,'signature'if mode in ('inquiry','reset-error')else'normal')
        self.device=Device(self,self.t)
        self.core=types.SimpleNamespace(USBError=USBError,find=self.find)
        self.util=types.SimpleNamespace(ENDPOINT_OUT=0,ENDPOINT_IN=128,ENDPOINT_TYPE_BULK=2,
            endpoint_direction=lambda a:a&128,endpoint_type=lambda a:a&3,
            find_descriptor=self.descriptor,claim_interface=self.claim,dispose_resources=self.dispose)
    def find(self,**kw):
        self.trace.append(['find',kw]);self.calls+=1
        assert kw['idVendor']==0x4c4a and kw['idProduct']==0x8057
        if self.mode=='find-error':raise USBError('enumeration failed')
        if self.mode=='never'or self.calls<2:return None
        return self.device
    def descriptor(self,intf,custom_match):
        for endpoint in intf:
            if custom_match(endpoint):return None if self.mode=='endpoints'else endpoint
    def claim(self,device,index):
        self.trace.append(['claim',index]);self.claims+=1
        if self.mode=='cancel':raise Cancel('cancelled')
        if self.mode=='claim'and self.claims==1:raise USBError('claim failed')
    def dispose(self,device):self.trace.append(['dispose']);self.t.queue.clear();self.t.pending=None


for mode in ['normal','never','find-error','detach','claim','endpoints','inquiry','reset-error','cancel']:
    outputs=[]
    for module in [old,current]:
        clock=Clock();module.time=clock;usb=USB(module,mode)
        module.backend=lambda u:u.trace.append(['backend'])or'mock-backend'
        out=io.StringIO()
        def run():
            bot=module.open_device(usb,2.05 if mode=='claim'else .05)
            return bot.tag
        with contextlib.redirect_stdout(out):outcome=result(run)
        outputs.append([outcome,out.getvalue(),usb.trace,usb.t.trace,round(clock.now,6)])
    check('capture-'+mode,*outputs);records.append(outputs[0])

# Execute the exact frozen main retry block independently of file/network hash
# acquisition, which is qualified separately. Both sides use the real BOT path.
source=(BASE/'tools/fm1_rescue.py').read_text()
start=source.index('        for attempt in range(1, args.tries + 1):')
end=source.index('    except Fail as e:',start)
fragment='def frozen_retry(image,loader,backup_dir,args,usb):\n'+source[start:end]
namespace=dict(old.__dict__);exec(compile(fragment,'frozen-main-retry','exec'),namespace)
for mode,tries,write in [('normal',1,False),('different',1,True),('protected',2,True),('inquiry',2,False),('normal',0,False)]:
    with tempfile.TemporaryDirectory(prefix='fm1-capture-proof-')as directory:
        outputs=[]
        for module in [old,current]:
            clock=Clock();module.time=clock;usb=USB(module,mode);module.backend=lambda u:u.trace.append(['backend'])or'mock-backend'
            image=bytes(usb.t.flash[:0x93000])
            if mode=='different':
                for addr in [0x4000,0x43000,0x92000]:usb.t.flash[addr]^=3
            if mode=='protected':
                image=bytes([image[0]^1])+image[1:]
            args=types.SimpleNamespace(tries=tries,wait=.05,write=write,ask=False)
            out=io.StringIO()
            def run():
                if module is old:
                    namespace.update(time=clock,backend=module.backend,open_device=module.open_device,rescue=module.rescue)
                    return namespace['frozen_retry'](image,b'virtual loader',directory,args,usb)
                adapter=module._Adapter(usb=usb,say=lambda *a,**kw:print(*a,**kw,flush=True))
                return module._call('recover',[list(image),list(b'virtual loader'),write,directory,False,tries,.05],adapter,bulk=True)
            with contextlib.redirect_stdout(out):outcome=result(run)
            outputs.append([outcome,out.getvalue().replace(directory,'<backup>'),usb.trace,usb.t.trace])
            for path in Path(directory).iterdir():path.unlink()
        check(f'main-retry-{mode}-{tries}',*outputs);records.append(outputs[0])

# Keepalive job determinism: a fake Event and Thread exercise the original
# thread/Event adapter while Rust decides every wait/command/loss transition.
import threading
real_event,real_thread=threading.Event,threading.Thread
for count in [0,1,3]:
    for lost in [False,True]:
        outputs=[]
        for module in [old,current]:
            trace=[]
            class Event:
                def __init__(self):self.n=0;self.stop=False
                def wait(self,secs):trace.append(['wait',secs]);self.n+=1;return self.stop or self.n>count
                def set(self):trace.append(['stop']);self.stop=True
            class Thread:
                def __init__(self,target,daemon):self.target=target;trace.append(['thread',daemon])
                def start(self):trace.append(['start']);self.target()
                def join(self):trace.append(['join'])
            class Dev:
                def cmd(self,cmd,args=b''):
                    trace.append(['cmd',cmd])
                    if lost:raise module.Fail('lost loader')
                    return bytes(14)
            threading.Event,threading.Thread=Event,Thread
            try:
                outcome=result(lambda:module.keep_alive(Dev(),lambda:trace.append(['confirm'])or True))
            finally:threading.Event,threading.Thread=real_event,real_thread
            outputs.append([outcome,trace])
        check(f'keepalive-{count}-{lost}',*outputs);records.append(outputs[0])

print(json.dumps(dict(cases=len(cases),trace_sha256=hashlib.sha256(json.dumps(records,separators=(',',':'),sort_keys=True).encode()).hexdigest(),physical_usb_opened=False),indent=2))
