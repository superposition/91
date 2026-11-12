#!/usr/bin/env python3
"""Jam Pack compiler and USB editor for NAIWAN 91. Standard library for pack work."""
import argparse
import copy
import json
import pathlib
import struct
import sys
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
MAX = 3840
NODE, EDGE, FORM, BASS, DRUM, SOUND, META = 64, 192, 448, 576, 864, 992, 1120
REST, APPROACH, THIRD, FIFTH = -128, -127, -126, -125

def factory():
    from make_params import param_enum
    params = param_enum()
    nodes = []
    major = [(0,0,0,0),(2,5,0,1),(4,7,0,1),(5,5,0,0),(7,2,1,3),(9,0,0,1),(11,2,1,2)]
    minor = [(0,0,1,4),(2,5,1,2),(3,3,0,0),(5,8,0,1),(7,2,1,3),(8,8,0,0),(11,2,1,2)]
    for mode, group in enumerate([major, minor]):
        nodes += [dict(bass=a,sixth=b,scale=c,label=d,mode=mode,emphasis=10 if i in (3,5) else 4 if i in (1,2) else 0) for i,(a,b,c,d) in enumerate(group)]
    edges = [[0]*14 for _ in nodes]
    routes = [{1:4,3:4,5:5}, {4:12,6:1}, {5:10,3:3}, {1:7,4:6,0:3}, {0:12,5:3}, {1:10,3:3}, {4:10,0:3}]
    for mode in range(2):
        for i,route in enumerate(routes):
            for j,w in route.items(): edges[7*mode+i][7*mode+j]=w
    patterns=[]
    for events in [
        {0:0,3:FIFTH,6:0,8:THIRD,10:FIFTH,14:APPROACH},
        {0:0,2:0,6:FIFTH,8:0,11:THIRD,14:APPROACH},
        {0:0,4:THIRD,6:FIFTH,8:0,10:12,15:APPROACH},
        {0:0,3:0,6:FIFTH,8:THIRD,12:FIFTH,14:APPROACH}]:
        notes=[events.get(i,REST) for i in range(16)]
        patterns.append(dict(notes=notes,velocity=[(100 if i==0 else 88 if i%4==0 else 72) if i in events else 0 for i in range(16)],gate=58))
    sounds={"P_PAD":56,"P_BASS":72,"P_PULSE":60,"P_PADTONE":0,"P_BRIGHT":18,
        "P_PE_ATK":6,"P_PE_REL":32,"P_DR_INST":23,"P_DR_CUT":42,"P_DR_RES":8,
        "P_DE_ATK":0,"P_DE_DEC":25,"P_DE_AMT":6,"P_DRIVE":0,"P_DETUNE":0,
        "P_PL_DEPTH":0,"P_DL_DEPTH":0,"P_PL_CHO":0,"P_BL_LEN":45,"P_BL_GLIDE":0,
        "P_REVERB":10,"P_SIZE":12,"P_RV_TYPE":0,"P_RV_DAMP":75,"P_ECHO":0,
        "P_MOTION":0,"P_HUMAN":12,"P_SWING":8,"P_KIT_KICK":4,"P_KIT_SNARE":4,
        "P_KIT_HAT":4,"P_KIT_PERC":4}
    return dict(version=1,name="Harris Citypop",scales=[[0,2,4,5,7,8,9,11],[0,2,3,5,7,8,9,11]],nodes=nodes,
        cadence=[[0,1,4],[7,8,11]],transitions=edges,
        forms=[dict(name="Vamp",roles=[0]),dict(name="Verse / Chorus",roles=[0,1]),dict(name="AABA",roles=[0,0,2,0]),dict(name="Journey",roles=[0,1,2,3])],
        bass=patterns,drums=[dict(lanes=[0x0441,0x1010,0,0x5555,0x0400,0x8000],comp=0x1111),dict(lanes=[0x4541,0x1010,0x1000,0x5555,0x4040,0x8000],comp=0x1111)],
        sounds=[dict(parameter=params[k],value=v,name=k) for k,v in sounds.items()],
        sources=["Original generated accompaniment; Barry Harris sixth-diminished framework.",
            "https://github.com/aaelony/barry-harris-exercises", "https://github.com/BenjaminPoilve/minichord/issues/37",
            "Free MIDI reference (not copied into this factory pack): https://www.patreon.com/musicianparadise/posts/gospel-scripts-150853281"])

def integer(v, lo, hi, name):
    if type(v) is not int or not lo<=v<=hi: raise ValueError(f"{name}: expected integer {lo}..{hi}, got {v!r}")
    return v

def compile_pack(d):
    if d.get('version')==2:
        from songbook import compile_pack as song_compile
        return song_compile(d)
    integer(d['version'],1,1,'version')
    b=bytearray(META); b[:5]=b'JMP1\x01'
    def name(at,n,value):
        data=value.encode('ascii')
        if not data or len(data)>=n or any(x<32 or x>126 for x in data): raise ValueError('name must be printable ASCII and fit its field')
        b[at:at+len(data)]=data
    name(16,24,d['name'])
    for at,key,maximum in [(5,'nodes',16),(6,'bass',8),(7,'drums',8),(8,'sounds',32)]: b[at]=integer(len(d[key]),0 if key=='sounds' else 1,maximum,key)
    if len(d['scales'])!=2 or len(d['forms'])!=4 or len(d['cadence'])!=2: raise ValueError('need two scales, two cadences, four forms')
    for i,scale in enumerate(d['scales']):
        if len(scale)!=8 or scale[0]!=0 or sorted(set(scale))!=scale: raise ValueError('scale must be eight ascending distinct pitch classes starting at 0')
        b[40+i*8:48+i*8]=bytes(integer(x,0,11,'scale') for x in scale)
    for i,group in enumerate(d['cadence']):
        if len(group)!=3: raise ValueError('cadence is [tonic, ii, dominant]')
        for j,x in enumerate(group):
            b[9+i*3+j]=integer(x,0,b[5]-1,'cadence node')
            if d['nodes'][x]['mode']!=i: raise ValueError('cadence crosses key mode')
    for i,a in enumerate(d['nodes']):
        for j,(key,hi) in enumerate([('bass',11),('sixth',11),('scale',1),('label',5),('mode',1)]): b[NODE+i*8+j]=integer(a[key],0,hi,key)
        b[NODE+i*8+5]=integer(a.get("emphasis",0),0,15,"section emphasis")
    if len(d['transitions'])!=b[5]: raise ValueError('transition row count')
    for i,row in enumerate(d['transitions']):
        if len(row)!=b[5] or not sum(row): raise ValueError('each node needs outgoing weights')
        for j,w in enumerate(row):
            b[EDGE+i*16+j]=integer(w,0,255,'weight')
            if w and d['nodes'][i]['mode']!=d['nodes'][j]['mode']: raise ValueError('transition crosses key mode')
    for i,f in enumerate(d['forms']):
        o=FORM+i*32; name(o,16,f['name']); b[o+16]=integer(len(f['roles']),1,8,'section count')
        for j,r in enumerate(f['roles']): b[o+17+j]=integer(r,0,3,'section role')
    for i,p in enumerate(d['bass']):
        o=BASS+i*36
        if len(p['notes'])!=16 or len(p['velocity'])!=16: raise ValueError('bass pattern needs 16 notes and velocities')
        for j,v in enumerate(p['notes']):
            integer(v,-128,12,'bass note')
            if v < -12 and v not in (REST,APPROACH,THIRD,FIFTH): raise ValueError('unknown bass operation')
            b[o+j]=v&255
        b[o+16:o+32]=bytes(integer(v,0,127,'velocity') for v in p['velocity'])
        b[o+32]=integer(p['gate'],1,100,'gate')
    for i,p in enumerate(d['drums']):
        if len(p['lanes'])!=6: raise ValueError('six drum lanes required')
        struct.pack_into('<7H',b,DRUM+i*16,*[integer(x,0,65535,'mask') for x in p['lanes']+[p['comp']]])
        b[DRUM+i*16+14]=integer(p.get("comp_gate",80),1,100,"comp gate")
    for i,p in enumerate(d['sounds']):
        ix=integer(p['parameter'],0,175,'parameter')
        if ix in (5,71,72,73,74,75): raise ValueError('packs cannot overwrite tempo, key or global preferences')
        table=ROOT/'build/params.json'
        if table.exists():
            limits=json.loads(table.read_text(encoding='utf-8'))['params'][ix]
            integer(p['value'],limits['min'],limits['max'],'sound value')
        struct.pack_into('<Hh',b,SOUND+i*4,ix,integer(p['value'],-32768,32767,'sound value'))
    if not isinstance(d.get('sources',[]),list) or any(not isinstance(x,str) for x in d.get('sources',[])): raise ValueError('sources must be a list of strings')
    b.extend(json.dumps(d.get('sources',[]),ensure_ascii=False,separators=(',',':')).encode())
    if len(b)>MAX: raise ValueError(f'pack is {len(b)} bytes, limit {MAX}')
    return bytes(b)

def decompile(b):
    if b[:5]==b'JMP2\2':
        from songbook import decompile as song_decode
        return song_decode(b)
    if not META<=len(b)<=MAX or b[:5]!=b'JMP1\x01': raise ValueError('unsupported or truncated pack')
    if not 1<=b[5]<=16 or not 1<=b[6]<=8 or not 1<=b[7]<=8 or b[8]>32:raise ValueError("invalid pack counts")
    def name(o,n): return b[o:o+n].split(b'\0')[0].decode('ascii')
    d=dict(version=1,name=name(16,24),scales=[list(b[40+i*8:48+i*8]) for i in range(2)],
        cadence=[list(b[9:12]),list(b[12:15])],nodes=[],transitions=[],forms=[],bass=[],drums=[],sounds=[])
    for i in range(b[5]):
        d['nodes'].append(dict(zip(['bass','sixth','scale','label','mode','emphasis'],b[NODE+i*8:NODE+i*8+6])))
        d['transitions'].append(list(b[EDGE+i*16:EDGE+i*16+b[5]]))
    for i in range(4):
        o=FORM+i*32; d['forms'].append(dict(name=name(o,16),roles=list(b[o+17:o+17+b[o+16]])))
    for i in range(b[6]):
        o=BASS+i*36; d['bass'].append(dict(notes=list(struct.unpack_from('16b',b,o)),velocity=list(b[o+16:o+32]),gate=b[o+32]))
    for i in range(b[7]):
        v=struct.unpack_from('<7H',b,DRUM+i*16); d['drums'].append(dict(lanes=list(v[:6]),comp=v[6],comp_gate=b[DRUM+i*16+14] or 80))
    for i in range(b[8]):
        ix,v=struct.unpack_from('<Hh',b,SOUND+i*4); d['sounds'].append(dict(parameter=ix,value=v))
    d['sources']=json.loads(b[META:] or b'[]')
    compile_pack(d)
    return d

def load(path):
    p=pathlib.Path(path)
    return compile_pack(json.loads(p.read_text(encoding='utf-8'))) if p.suffix=='.json' else compile_pack(decompile(p.read_bytes()))

def write_json(path,d): pathlib.Path(path).write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--port');sub=ap.add_subparsers(dest='cmd',required=True)
    p=sub.add_parser('factory');p.add_argument('output')
    p=sub.add_parser('factory-song');p.add_argument('output');p.add_argument('--song',type=int,choices=range(3),default=0)
    p=sub.add_parser('compile');p.add_argument('input');p.add_argument('output');p.add_argument('--c-header')
    p=sub.add_parser('validate');p.add_argument('input')
    p=sub.add_parser('decode');p.add_argument('input');p.add_argument('output')
    p=sub.add_parser('preview');p.add_argument('input');p.add_argument('output');p.add_argument('--bars',type=int,choices=[8,16],default=16);p.add_argument('--bpm',type=int,default=100);p.add_argument('--key',type=int,choices=range(12),default=0);p.add_argument('--minor',action='store_true');p.add_argument('--phrases',type=int,default=2)
    p=sub.add_parser('import-voice');p.add_argument('input');p.add_argument('base');p.add_argument('output');p.add_argument('--voice',type=int,default=0);p.add_argument('--role',choices=['keys','bass'],required=True)
    p=sub.add_parser('inspect-midi');p.add_argument('input')
    p=sub.add_parser('import-midi');p.add_argument('input');p.add_argument('base');p.add_argument('output');p.add_argument('--track',type=int,required=True);p.add_argument('--tonic',type=int,choices=range(12),required=True);p.add_argument('--bar',type=int,default=1);p.add_argument('--role',choices=['bass','harmony'],required=True);p.add_argument('--minor',action='store_true')
    for cmd in ('ports','capabilities','features','song-status','status','abort'):sub.add_parser(cmd)
    p=sub.add_parser('pull');p.add_argument('output');p.add_argument('--slot',type=int,choices=range(5),default=0)
    p=sub.add_parser('diff');p.add_argument('input');p.add_argument('--against')
    for cmd in ('audition','rollback'):
        p=sub.add_parser(cmd);p.add_argument('input');p.add_argument('--backup-dir',default=str(ROOT/'build/jam-backups'));p.add_argument('--no-wait',action='store_true')
    p=sub.add_parser('save');p.add_argument('slot',type=int,choices=range(1,5))
    p=sub.add_parser('control');p.add_argument('field',choices=['on','key','bars','form','movement','energy','hold','bass','idea','play','song','swell','variation','keys_program','bass_program','touch','release','bass_gate','swell_voice','bass_attack','brightness']);p.add_argument('value',type=int)
    a=ap.parse_args()
    if a.cmd=='factory-song':
        from songbook import factory as factory_song
        write_json(a.output,factory_song(a.song));return
    if a.cmd=='factory':write_json(a.output,factory());return
    if a.cmd=='preview':
        from jam_preview import preview
        if not 70<=a.bpm<=190 or not 1<=a.phrases<=16:raise ValueError('BPM 70..190; phrases 1..16')
        print(json.dumps(preview(ROOT,a.input,a.output,a.bars,a.bpm,a.key,a.minor,a.phrases)));return
    if a.cmd=='import-voice':
        from gen_fm6_dy import dumps,name_of
        banks=dumps(pathlib.Path(a.input).read_bytes());voices=[v for bank in banks for v in bank]
        if not 0<=a.voice<len(voices):raise ValueError('voice index outside bank')
        d=decompile(load(a.base))
        if d['version']!=2:raise ValueError('voice roles require a Songbook v2 pack')
        if len(d['voice_programs'])>=8:raise ValueError('all eight program slots are used')
        d['parts'][a.role]=len(d['voice_programs']);d['voice_programs'].append(dict(name=name_of(voices[a.voice]),data=list(voices[a.voice])))
        d['sources'].append('User-selected DX voice: '+pathlib.Path(a.input).name+', voice '+str(a.voice))
        compile_pack(d);write_json(a.output,d);print(a.output);return
    if a.cmd=='inspect-midi':
        from jam_preview import inspect_midi
        print(json.dumps(inspect_midi(a.input),indent=2));return
    if a.cmd=='import-midi':
        from jam_preview import import_midi
        if a.bar<1:raise ValueError('bar is one-based')
        d=import_midi(a.input,a.base,a.track,a.tonic,a.bar,a.role,a.minor);compile_pack(d);write_json(a.output,d);print(a.output);return
    if a.cmd=='ports':
        from jam_midi import ports
        print(json.dumps(ports(),indent=2));return
    if a.cmd=='diff':
        import difflib
        left=decompile(load(a.input))
        if a.against:right=decompile(load(a.against))
        else:
            from jam_midi import Device
            with Device(a.port) as dev:right=decompile(dev.read())
        print(''.join(difflib.unified_diff(json.dumps(left,indent=2).splitlines(True),json.dumps(right,indent=2).splitlines(True),fromfile=a.input,tofile=a.against or 'device')));return
    if a.cmd in ('capabilities','features','song-status','status','abort','pull','audition','rollback','save','control'):
        from jam_midi import Device
        with Device(a.port) as dev:
            if a.cmd=='capabilities':result=dev.capabilities()
            elif a.cmd=='features':result=dev.features()
            elif a.cmd=='song-status':result=dev.song_status()
            elif a.cmd=='status':result=dev.status()
            elif a.cmd=='abort':dev.request(9);result={'aborted':True}
            elif a.cmd=='pull':
                b=dev.read(a.slot)
                if not b:raise ValueError('slot is empty')
                if pathlib.Path(a.output).suffix=='.json':write_json(a.output,decompile(b))
                else:pathlib.Path(a.output).write_bytes(b)
                result={'output':a.output,'crc32':f'{zlib.crc32(b):08x}'}
            elif a.cmd in ('audition','rollback'):
                b=load(a.input);dev.capabilities();
                if b[4]==2 and 2 not in dev.features()['pack_versions']:raise ValueError('device needs Songbook engine upgrade')
                previous=dev.read();folder=pathlib.Path(a.backup_dir);folder.mkdir(parents=True,exist_ok=True)
                backup=folder/(time.strftime('%Y%m%d-%H%M%S')+f'-{time.time_ns()%1000000:06d}.jmp');backup.write_bytes(previous)
                print(json.dumps({'rollback':str(backup)}),flush=True);dev.audition(b)
                result={'queued':True} if a.no_wait else dev.wait_active(b)
            elif a.cmd=='save':result=dev.save(a.slot)
            else:
                limits={'on':(0,1),'key':(0,23),'bars':(8,16),'form':(0,3),'movement':(0,3),'energy':(0,100),'hold':(0,1),'bass':(0,7),'idea':(1,1),'play':(0,1),'song':(0,6),'swell':(0,100),'variation':(0,3),'keys_program':(0,7),'bass_program':(0,7),'touch':(0,100),'release':(1,100),'bass_gate':(1,100),'swell_voice':(0,23),'bass_attack':(0,100),'brightness':(0,100)}
                integer(a.value,*limits[a.field],a.field)
                if a.field=='bars' and a.value not in (8,16):raise ValueError('bars must be 8 or 16')
                result=dev.control(a.field,a.value)
            print(json.dumps(result,indent=2));return
    b=load(a.input)
    if a.cmd=='compile':
        pathlib.Path(a.output).write_bytes(b)
        if a.c_header:pathlib.Path(a.c_header).write_text('/* Generated by tools/fm1_jam.py; GPL-3.0-only */\nstatic const uint8_t JP_FACTORY[] = {\n'+''.join('    '+','.join(str(x) for x in b[i:i+24])+',\n' for i in range(0,len(b),24))+'};\n',encoding='utf-8')
    elif a.cmd=='decode':write_json(a.output,decompile(b))
    print(json.dumps(dict(bytes=len(b),crc32=f'{zlib.crc32(b):08x}',name=decompile(b)['name'])))

if __name__=='__main__':
    try:main()
    except (ValueError,KeyError,OSError,RuntimeError) as e:sys.exit(str(e))
