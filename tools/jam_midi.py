"""USB-MIDI Jam Pack transport. No firmware flashing or implicit persistent writes."""
import pathlib
import secrets
import struct
import time
import zlib

ERRORS = {1:'malformed request or invalid pack',2:'checksum mismatch',3:'device busy',4:'stop playback before saving',5:'persistent save/readback failed',6:'unsupported pack version'}
FIELDS={'on':0,'key':1,'bars':2,'form':3,'movement':4,'energy':5,'hold':6,'bass':7,'idea':8,'slot':9,'play':10,'song':11,'swell':12,'variation':13,'keys_program':14,'bass_program':15,'touch':16,'release':17,'bass_gate':18,'swell_voice':19,'bass_attack':20,'brightness':21}

def checksum(a): return (-sum(a)) & 127
def nibble(a): return [v for x in a for v in (x>>4,x&15)]
def frame(op,token,data=()):
    a=[op,token,*data]
    if any(not 0<=x<128 for x in a): raise ValueError('SysEx payload is not seven-bit clean')
    return [0x7d,0x50,0x60,*a,checksum(a)]

def parse_reply(data,op,token):
    a=list(data)
    if len(a)<7 or a[:5]!=[0x7d,0x50,0x61,op,token]: return None
    if checksum(a[3:-1])!=a[-1] or (len(a)-7)%2 or any(x>15 for x in a[6:-1]): raise ValueError('damaged device reply')
    if a[5]: raise RuntimeError(ERRORS.get(a[5],f'device error {a[5]}'))
    return bytes(a[i]<<4|a[i+1] for i in range(6,len(a)-1,2))

class Device:
    def __init__(self,port=None):
        import mido
        self.mido=mido;self.token=secrets.randbelow(127)+1
        ins=mido.get_input_names();outs=mido.get_output_names()
        def pick(names):
            found=[n for n in names if (port.lower() in n.lower() if port else 'fm-1' in n.lower() or 'fm1' in n.lower())]
            if len(found)!=1: raise RuntimeError(f'Expected one FM-1 port; use --port. Available: {names}')
            return found[0]
        self.inp=mido.open_input(pick(ins));self.out=mido.open_output(pick(outs))
    def __enter__(self): return self
    def __exit__(self,*args): self.inp.close();self.out.close()
    def request(self,op,data=(),timeout=3):
        self.out.send(self.mido.Message('sysex',data=frame(op,self.token,data)))
        until=time.monotonic()+timeout
        while time.monotonic()<until:
            for msg in self.inp.iter_pending():
                if msg.type=='sysex':
                    r=parse_reply(msg.data,op,self.token)
                    if r is not None:return r
            time.sleep(.005)
        raise TimeoutError(f'FM-1 did not answer jam operation {op}; it may need the initial Jam firmware upgrade')
    def capabilities(self):
        r=self.request(1)
        if len(r)!=6 or r[0]!=1: raise RuntimeError('unsupported Jam protocol')
        return dict(protocol=r[0],max_bytes=r[1]|r[2]<<8,slots=r[3],first_object=r[4],session_object=r[5])
    def features(self):
        try:r=self.request(11)
        except RuntimeError as e:
            if str(e)==ERRORS[1]:raise RuntimeError('device needs the Songbook 0.3.0 engine upgrade for v2 songs and DX voice roles') from e
            raise
        if len(r)!=4:raise ValueError('bad feature response')
        return dict(pack_versions=[i+1 for i in range(2) if r[0]>>i&1],voice_programs=r[1],factory_songs=r[2],fm_voices=r[3])
    def song_status(self):
        r=self.request(12)
        if len(r)!=3:raise ValueError('bad song status')
        return dict(song=r[0],pack_version=r[1],loading=r[2])
    def status(self):
        r=self.request(8)
        if len(r)!=25: raise ValueError('bad status length')
        keys=['on','bars','bar','beat','hold','pending','slot'];d=dict(zip(keys,r[:7]))
        d.update(crc32=f'{struct.unpack_from("<I",r,7)[0]:08x}',revision=struct.unpack_from('<I',r,11)[0],xruns=struct.unpack_from('<I',r,15)[0])
        d.update(zip(['key','minor','form','movement','energy','playing'],r[19:25]));return d
    def read(self,slot=0):
        b=bytearray();index=0
        while True:
            r=self.request(2,[slot,index])
            if len(r)<4: raise ValueError('truncated readback')
            total,off=struct.unpack_from('<HH',r)
            if total>3840 or off!=len(b) or len(r)-4!=min(64,total-off): raise ValueError('inconsistent readback')
            b.extend(r[4:])
            if len(b)==total:return bytes(b)
            index+=1
    def audition(self,b):
        crc=zlib.crc32(b)
        self.request(3,[len(b)>>7,len(b)&127,*[(crc>>s)&15 for s in range(28,-1,-4)]])
        try:
            for i,off in enumerate(range(0,len(b),256)):
                c=b[off:off+256];ack=self.request(4,[i,len(c)>>7,len(c)&127,*nibble(c)])
                if ack!=bytes([i]):raise ValueError('wrong chunk acknowledgement')
            self.request(5);self.request(6)
        except Exception:
            try:self.request(9)
            except Exception:pass
            raise
    def wait_active(self,b,timeout=75):
        want=f'{zlib.crc32(b):08x}';until=time.monotonic()+timeout
        while time.monotonic()<until:
            s=self.status()
            if not s['pending'] and s['crc32']==want:return s
            time.sleep(.25)
        raise TimeoutError('pack is still pending; status can check it without resending')
    def save(self,slot):
        before=self.status()
        if before['playing']: raise RuntimeError(ERRORS[4])
        r=self.request(7,[slot],timeout=15)
        saved=self.read(slot)
        if len(r)!=4 or zlib.crc32(saved)!=struct.unpack('<I',r)[0] or f'{zlib.crc32(saved):08x}'!=before['crc32']:
            raise RuntimeError('saved pack failed readback comparison')
        return dict(slot=slot,bytes=len(saved),crc32=before['crc32'])
    def control(self,field,value):
        self.request(10,[FIELDS[field],value>>7,value&127]);time.sleep(.04);return self.status()

def ports():
    import mido
    return dict(inputs=mido.get_input_names(),outputs=mido.get_output_names())
