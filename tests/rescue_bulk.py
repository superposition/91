#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""ROM/BOT policy against virtual endpoints and NOR flash. Never imports USB."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI);sys.path.insert(0,str(ROOT/'tools'))
import fm1_rescue as current


def load():
    spec=importlib.util.spec_from_file_location('frozen_bulk',BASE/'tools/fm1_rescue.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


old=load();cases=[];traces=[]
INITIAL_FLASH=bytes((i*37+i//4096)&255 for i in range(0x100000))


def result(fn):
    try:
        v=fn();return ['ok',v.hex()if isinstance(v,bytes)else v]
    except BaseException as e:return ['error',type(e).__name__,str(e)]


def check(name,a,b):
    assert a==b,(name,a,b);cases.append(name)


class USBError(OSError):pass


class Clock:
    def time(self):return 100.0
    def strftime(self,fmt):return fmt.replace('%Y%m%d-%H%M%S','20200102-030405')


class Transport:
    def __init__(self,module,mode='normal',flash=None,cut=None):
        self.m,self.mode,self.cut=module,mode,cut
        self.flash=bytearray(INITIAL_FLASH if flash is None else flash)
        self.initial=bytes(self.flash);self.trace=[];self.queue=[];self.pending=None;self.programmed=0;self.ran=False;self.required_backup=None
        self.flash_reads=0
    def status(self,tag):
        status=1 if self.mode=='status'else 0
        packet=struct.pack('<4sIIB',b'BAD!'if self.mode=='signature'else b'USBS',(tag+1)&0xffffffff if self.mode=='tag'else tag,99,status)
        if self.mode=='short-status':packet=packet[:-1]
        if self.mode=='long-status':packet+=b'\0'
        return packet
    def write(self,packet,timeout):
        packet=bytes(packet);self.trace.append(['write',timeout,len(packet),packet.hex()if len(packet)<=31 else hashlib.sha256(packet).hexdigest()])
        if self.pending is not None:
            cdb,tag,length=self.pending;self.pending=None
            cmd=struct.unpack('>H',cdb[:2])[0];addr,n=struct.unpack('>IH',cdb[2:8]);crc=struct.unpack('<H',cdb[9:11])[0]
            assert n==length==len(packet)and old.crc16(packet)==crc
            if cmd==self.m.CMD_WRITE_FLASH:
                assert 0x4000<=addr and addr+n<=0x93000
                if self.cut==self.programmed:raise USBError('power cut')
                self.programmed+=1
                for i,b in enumerate(packet):self.flash[addr+i]&=b
            elif cmd!=self.m.CMD_WRITE_MEMORY:raise AssertionError(cmd)
            self.queue.append(self.status(tag));return len(packet)
        assert len(packet)==31 and packet[:4]==b'USBC'
        tag,length,flags,lun,clen=struct.unpack('<IIBBB',packet[4:15]);cdb=packet[15:15+clen]
        if flags&128:
            if cdb[0]==18 and clen==6:
                data=bytearray(36);data[8:16]=b'WL82    ';data[16:32]=b'UBOOT1.00       '
            else:
                cmd=struct.unpack('>H',cdb[:2])[0];body=bytearray(14)
                if cmd==self.m.CMD_READ_FLASH:
                    self.flash_reads+=1
                    if self.mode=='empty-data'and self.flash_reads==3:raise USBError('non-converging read limit')
                    addr,n=struct.unpack('>IH',cdb[2:8]);data=self.flash[addr:addr+n]
                    if self.mode=='short-data'and len(data)>8:data=data[:7]
                    if self.mode=='verify':data=bytearray(data);data[0]^=1
                    if self.mode=='empty-data':data=b''
                    if self.mode=='oversized-data':data=bytes(data)+b'x'
                else:
                    if cmd==self.m.CMD_READ_KEY:
                        key=bytearray([0x0f,0x98]);old.crc_cipher(key);body[5],body[4]=key
                    elif cmd==self.m.CMD_GET_ONLINE_DEVICE:body[0]=3;body[2:6]=struct.pack('<I',0xb36014)
                    elif cmd==self.m.CMD_ERASE_SECTOR:
                        addr=struct.unpack('>I',cdb[2:6])[0];assert 0x4000<=addr<0x93000 and addr%4096==0
                        if self.required_backup is not None:
                            assert self.required_backup.exists()and self.required_backup.read_bytes()==self.initial,'write preceded full backup'
                        self.flash[addr:addr+4096]=b'\xff'*4096
                    elif cmd==self.m.CMD_RUN_APP:self.ran=True
                    elif cmd!=self.m.CMD_JUMP:raise AssertionError(hex(cmd))
                    data=cmd.to_bytes(2,'big')+body
            self.queue.append(bytes(data));self.queue.append(self.status(tag))
        elif length:
            self.pending=(cdb,tag,length)
        else:self.queue.append(self.status(tag))
        return len(packet)
    def read(self,n,timeout):
        if self.mode=='read-error':raise USBError('read failed')
        if self.mode=='run-error'and self.ran:raise USBError('reset before status')
        assert self.queue,'no virtual reply'
        data=self.queue.pop(0);self.trace.append(['read',n,timeout,len(data),data.hex()if len(data)<=36 else hashlib.sha256(data).hexdigest()]);return data


def device(module,transport,tag=1):
    bot=module.Bot.__new__(module.Bot);bot.ep_in=transport;bot.ep_out=transport;bot.tag=tag
    return module.Uboot(bot),bot


operations=[('inquiry',lambda d:d.inquiry()),('chip',lambda d:d.chip()),('read',lambda d:d.read(0x4000,1027)),('loader',lambda d:d.start_loader(bytes(range(256))*4+b'last')),('sector',lambda d:d.write_sector(0x41000,bytes((i*17)&255 for i in range(4096)))),('run',lambda d:d.run_app())]
for mode in ['normal','signature','tag','status','short-status','long-status','read-error','short-data','verify','run-error']:
    for name,fn in operations:
        outputs=[]
        for module in [old,current]:
            t=Transport(module,mode);dev,bot=device(module,t,0xffffffff if name=='chip'else 1)
            outputs.append([result(lambda:fn(dev)),t.trace,bot.tag,hashlib.sha256(t.flash).hexdigest(),t.ran])
            assert t.flash[:0x4000]==t.initial[:0x4000]and t.flash[0x93000:]==t.initial[0x93000:]
        check(f'wire-{mode}-{name}',*outputs);traces.append(outputs[0])

# Every accepted sector and all forbidden boundaries execute the live guard.
for addr in list(range(0,0x100000,4096))+[-1,0x3fff,0x4001,0x92fff,0x93001,0x100000,0xffffffff]:
    outputs=[]
    for module in [old,current]:
        t=Transport(module);dev,bot=device(module,t);outcome=result(lambda:dev.write_sector(addr,b'\x55'*4096))
        outputs.append([outcome,t.trace,hashlib.sha256(t.flash).hexdigest()])
        assert t.flash[:0x4000]==t.initial[:0x4000]and t.flash[0x93000:]==t.initial[0x93000:]
    check(f'sector-boundary-{addr:x}',*outputs)

# Power loss at each of the eight program boundaries, then a clean session on
# the same persistent NOR bytes, must restore the sector and preserve the rest.
for cut in range(8):
    outputs=[]
    for module in [old,current]:
        t=Transport(module,cut=cut);dev,bot=device(module,t);first=result(lambda:dev.write_sector(0x92000,b'\xa5'*4096));t.cut=None;t.queue.clear();t.pending=None
        dev,bot=device(module,t);second=result(lambda:dev.write_sector(0x92000,b'\xa5'*4096))
        assert t.flash[0x92000:0x93000]==b'\xa5'*4096
        assert t.flash[:0x4000]==t.initial[:0x4000]and t.flash[0x93000:]==t.initial[0x93000:]
        outputs.append([first,second,t.trace,hashlib.sha256(t.flash).hexdigest()])
    check(f'restart-program-cut-{cut}',*outputs);traces.append(outputs[0])

with tempfile.TemporaryDirectory(prefix='fm1-bulk-proof-')as directory:
    outputs=[]
    for module in [old,current]:
        module.time=Clock();t=Transport(module);image=bytes(t.flash[:0x93000])
        for a in [0x4000,0x43000,0x92000]:t.flash[a]^=3
        t.initial=bytes(t.flash);t.required_backup=Path(directory)/'fm1-backup-20200102-030405.bin'
        dev,bot=device(module,t);said=[]
        outcome=result(lambda:module.rescue(dev,image,b'virtual loader'*50,True,directory,say=lambda *a,**kw:said.append([[str(x).replace(directory,'<backup>')for x in a],kw])))
        assert outcome==['ok',True]and t.flash==image+t.initial[0x93000:]
        t.required_backup.unlink()
        outputs.append([outcome,t.trace,said,hashlib.sha256(t.flash).hexdigest(),t.ran])
    check('full-bulk-backup-restore-verify',*outputs);traces.append(outputs[0])

# Explicit bounded refusal outside a converging legal read. The legacy empty
# fragment loop is capped by the fake only to record its non-progress fingerprint.
t=Transport(old,'empty-data');dev,bot=device(old,t);legacy=result(lambda:dev.read(0,512))
assert legacy[1]=='USBError'and t.flash_reads==3
for mode in ['empty-data','oversized-data']:
    t=Transport(current,mode);dev,bot=device(current,t);refusal=result(lambda:dev.read(0,512))
    assert refusal==['error','Fail','short or oversized flash read']and t.flash_reads==1
    assert t.flash==t.initial

print(json.dumps(dict(cases=len(cases),trace_sha256=hashlib.sha256(json.dumps(traces,separators=(',',':')).encode()).hexdigest(),invalid_fragment_refusals=2,protected_head_and_upper_preserved=True,physical_usb_opened=False),indent=2))
