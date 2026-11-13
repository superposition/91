#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""R01 DX-table/logo policy oracle. All generated writes stay in scratch trees."""
import base64
import builtins
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import re
import shutil
import subprocess
import sys
import tempfile
import types
import PIL

ROOT=Path(__file__).resolve().parents[1]
BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI,FM1_AUTHORING_CLI=CLI)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    sha,name=line.split(maxsplit=1)
    assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==sha,name
sys.path.insert(0,str(ROOT/'tools'))
import rust_tooling
import rust_authoring


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m);return m


old=load('old_dy',BASE/'tools/gen_fm6_dy.py');new=load('new_dy',ROOT/'tools/gen_fm6_dy.py')
oldlogo=load('old_logo',BASE/'tools/make_logo.py');logo=load('new_logo',ROOT/'tools/make_logo.py')
oldjam=load('old_jam',BASE/'tools/fm1_jam.py')
cases=[];records=[];paths=[(str(ROOT),'<ROOT>'),(str(BASE),'<R01>')]


def stable(v):
    if isinstance(v,bytes):return v.hex()
    if isinstance(v,(list,tuple)):return [stable(x) for x in v]
    if isinstance(v,dict):return {k:stable(x) for k,x in v.items()}
    if isinstance(v,Path):v=str(v)
    if isinstance(v,str):
        for p,label in paths:v=v.replace(p.replace('\\','\\\\'),label).replace(p,label)
    return v


def capture(fn):
    out=io.StringIO()
    with contextlib.redirect_stdout(out):
        try:v=['ok',fn()]
        except BaseException as e:v=['error',type(e).__name__,str(e)]
    return [v,out.getvalue()]


def check(name,want,got):
    if want!=got:
        p=ROOT/'build/dx-logo/failure.json';p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(json.dumps(stable(dict(name=name,want=want,got=got))),encoding='utf-8')
        raise AssertionError((name,str(want)[:250],str(got)[:250],str(p)))
    cases.append(name);records.append([name,hashlib.sha256(json.dumps(stable(want)).encode()).hexdigest()])


rng=random.Random(0xD71060)
def bank(seed=0,high=False,trailer=b'\x00\xf7',channel=0):
    body=bytearray((i*17+seed)% (256 if high else 128) for i in range(4096))
    for i in range(32):body[i*128+118:i*128+128]=('Voice %04d'%i).encode()
    return bytes([240,67,channel,9,32,0])+body+trailer


check('init',old.INIT,new.INIT)
check('init-native',list(old.INIT),rust_tooling.call('asset.dy.init'))
check('palette',oldlogo.PAL,logo.PAL)
check('live-source-sprite',oldlogo.sprite(),logo.sprite())
for i in range(150):
    data=bytes(rng.randrange(256) for _ in range(rng.randrange(0,150)))
    check('voice-name-%d'%i,old.name_of(data),new.name_of(data))
vectors=[b'',bytes(4096),bank(),bank(high=True,trailer=b'\x80\x00',channel=255),
         b'junk'+bank()+bank(17),b'\xf0\x43\x00\x00\x20\x00'+bank(),
         b'\xf0\x43'+bytes(4100)+bank()]
for cut in [0,1,2,3,5,6,127,4096,4102,4103]:vectors.append(bank()[:cut])
for i in range(120):
    data=bytearray(bank(i,high=bool(i%2),trailer=bytes([i,255-i])))
    if i%3==0:data[3]=rng.randrange(256)
    if i%4==0:data=bytearray(b'\xf0\x43\x00\x00\x00\x00')+data
    if i%5==0:data=data[:-rng.randrange(1,4104)]
    vectors.append(bytes(data))
for i,data in enumerate(vectors):check('bulk-scan-%d'%i,old.dumps(data),new.dumps(data))

# Both real consumers see the same permissive header-only profile. The original
# authoring implementation selects a flattened bank before compiling its pack.
document=oldjam.decompile((BASE/'packs/velvet-changes.jmp').read_bytes())
for i,data in enumerate(vectors[:17]+[bank(trailer=b'xx',channel=255),b'noise'+bank()+bank(9)]):
    for index in [0,31,32,63,-1,64]:
        def expected():
            voices=[v for b in old.dumps(data) for v in b]
            if not 0<=index<len(voices):raise ValueError('voice index outside bank')
            d=oldjam.decompile(oldjam.compile_pack(document));d['parts']['bass']=len(d['voice_programs'])
            d['voice_programs'].append(dict(name=old.name_of(voices[index]),data=list(voices[index])))
            d['sources'].append('User-selected DX voice: bulk.syx, voice '+str(index))
            oldjam.compile_pack(d);return d
        check('authoring-consumer-%d-%d'%(i,index),capture(expected),capture(lambda:rust_authoring.call(
            'import-voice',bytes=list(data),document=document,voice=index,role='bass',name='bulk.syx')))


class File:
    def __init__(self,raw,trace):self.raw,self.trace=raw,trace
    def read(self,*a):
        value=self.raw.read(*a);self.trace.append(['read',value]);return value
    def readline(self):
        value=self.raw.readline();self.trace.append(['line',value]);return value
    def write(self,v):self.trace.append(['write',v]);return self.raw.write(v)
    def __iter__(self):return self
    def __next__(self):
        value=self.readline()
        if not value:raise StopIteration
        return value
    def close(self):self.raw.close()
    def __del__(self):self.raw.close()


def traced(module,trace,refuse=None):
    def opened(path,*a,**kw):
        trace.append(['open',str(path),a,kw])
        if refuse is not None and refuse(str(path),a):raise PermissionError('deliberate write refusal')
        return File(builtins.open(path,*a,**kw),trace)
    def mkdir(path,**kw):trace.append(['mkdir',str(path),kw]);return os.makedirs(path,**kw)
    def listing(path):trace.append(['listdir',str(path)]);return os.listdir(path)
    module.open=opened
    module.os=types.SimpleNamespace(path=os.path,makedirs=mkdir,listdir=listing)


with tempfile.TemporaryDirectory(prefix='fm1-dx-logo-') as directory:
    tmp=Path(directory);paths.insert(0,(str(tmp),'<TMP>'))
    banks=tmp/'banks';banks.mkdir();(banks/'one.syx').write_bytes(b'junk'+bank()+bank(13))
    (banks/'high.syx').write_bytes(bank(2,high=True));(banks/'empty.syx').write_bytes(b'empty')
    rom=tmp/'rom';rom.mkdir();(rom/'ROM2.SYX').write_bytes(bank(91));(rom/'rom1.syx').write_bytes(bank(37));(rom/'notrom.syx').write_bytes(bank())
    target=tmp/'out';(target/'firmware/src/dsp').mkdir(parents=True);(target/'build').mkdir()
    manifest=tmp/'selection.txt'
    targets=[target/'firmware/src/dsp/fm6_dy.h',target/'build/fm6_dy.bin',target/'build/PERF-1-test-voices.syx']
    def run_dy(module,args,content):
        trace=[];traced(module,trace);module.SRC=str(target);manifest.write_text(content,encoding='utf-8')
        for p in targets:p.write_bytes(b'old sentinel')
        sys.argv=['gen_fm6_dy.py']+[str(manifest) if a=='MANIFEST' else str(banks) if a=='BANKS' else str(rom) if a=='ROM' else a for a in args]
        value=capture(module.main)
        return value,trace,[p.read_bytes() for p in targets]
    lines=['0|First|one.syx|0|source','1|Second|one.syx#1|31|origin','U|Audition|one.syx|4|user']
    scenarios=[([],''),(['--rom'],''),(['MANIFEST','BANKS','--rom'],''),
        (['MANIFEST','BANKS'],'# comment\n\n'),(['MANIFEST','BANKS'],'\n'.join(lines)),
        (['MANIFEST','BANKS','--rom','ROM'],'\n'.join(lines)),
        (['MANIFEST','BANKS'],'0|Last|one.syx#-1|-1|negative\n0|Overwrite|high.syx|+0|s|with pipes'),
        (['MANIFEST','BANKS'],'1|Gap|one.syx|0|source'),
        (['MANIFEST','BANKS'],'-1|Negative|one.syx|0|source'),
        (['MANIFEST','BANKS'],'0|Bad|one.syx#2|0|source'),
        (['MANIFEST','BANKS'],'0|Bad|one.syx|32|source'),
        (['MANIFEST','BANKS'],'0|Bad|one.syx|bad|source'),
        (['MANIFEST','BANKS'],'0|Bad|empty.syx|0|source'),
        (['MANIFEST','BANKS'],'only|two'),
        (['MANIFEST','BANKS'],'0|Bad|absent.syx|0|source'),
        (['MANIFEST','BANKS'],'\n'.join(['U|Many|one.syx|1|u']*35)),
        (['MANIFEST','BANKS'],'0|Only|one.syx|1|s\nU|Late failure|absent.syx|0|x')]
    scenarios.append((['MANIFEST','BANKS'],'U|High-bit audition|high.syx|3|source'))
    # A ROM match ignores the ten-byte voice name; refusal follows the printed row.
    scenarios.append((['MANIFEST','BANKS','--rom','ROM'],'0|Match|one.syx|0|source'))
    for i,(args,content) in enumerate(scenarios):
        if i==len(scenarios)-1:
            renamed=bytearray(bank())
            for j in range(32):renamed[6+j*128+118:6+j*128+128]=b'ROM NAME  '
            (rom/'rom3.syx').write_bytes(renamed)
        check('dy-main-%d'%i,run_dy(old,args,content),run_dy(new,args,content))
    # Replay the full real manifest using exact frozen factory records in their
    # original bank/voice positions. Unavailable U-only records use deterministic
    # fixture values; no source-bank download or provenance claim is implied.
    table=(BASE/'firmware/src/dsp/fm6_dy.h').read_text(encoding='utf-8')
    body=re.search(r'static const uint8_t FM6_DY\[FM6_NDY\]\[128\] = \{(.*?)\n\};',table,re.S).group(1)
    body=re.sub(r'/\*.*?\*/','',body,flags=re.S)
    factory=[bytes(int(x.strip()) for x in row.split(',') if x.strip()) for row in re.findall(r'\{([^{}]*)\}',body,re.S)]
    assert len(factory)==30 and all(len(r)==128 for r in factory)
    full_manifest=(BASE/'tools/fm6_dy.txt').read_text(encoding='utf-8');fixtures={}
    for line in full_manifest.splitlines():
        if not line.strip() or line.lstrip().startswith('#'):continue
        slot,name,path,vslot,source=[s.strip() for s in line.split('|',4)]
        path,_,which=path.partition('#');which=int(which or 0)
        if path not in fixtures:fixtures[path]=bytearray(bank(len(fixtures))+bank(len(fixtures)+1))
        if slot!='U':fixtures[path][which*4104+6+int(vslot)*128:which*4104+6+(int(vslot)+1)*128]=factory[int(slot)]
    for path,data in fixtures.items():
        dest=banks/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
    expected_full=run_dy(old,['MANIFEST','BANKS'],full_manifest)
    check('dy-full-manifest',expected_full,run_dy(new,['MANIFEST','BANKS'],full_manifest))
    check('frozen-factory-header-reproduced',(BASE/'firmware/src/dsp/fm6_dy.h').read_bytes(),expected_full[2][0])
    # Successful process-level -I invocation, with bank data never imported.
    scriptroot=tmp/'isolated';(scriptroot/'tools/data').mkdir(parents=True);(scriptroot/'firmware/src/dsp').mkdir(parents=True)
    for name in ['gen_fm6_dy.py','rust_tooling.py','rust_codecs.py']:shutil.copyfile(ROOT/'tools'/name,scriptroot/'tools'/name)
    shutil.copyfile(ROOT/'tools/data/dy-init.json',scriptroot/'tools/data/dy-init.json')
    manifest.write_text('\n'.join(lines),encoding='utf-8')
    expected=run_dy(old,['MANIFEST','BANKS'],'\n'.join(lines))
    completed=subprocess.run([sys.executable,'-I',str(scriptroot/'tools/gen_fm6_dy.py'),str(manifest),str(banks)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
    check('dy-isolated-cli-output',expected[0][1].replace(str(target),'<OUT>'),completed.stdout.decode().replace('\r\n','\n').replace(str(scriptroot),'<OUT>'))
    check('dy-isolated-cli-files',expected[2],[(scriptroot/p.relative_to(target)).read_bytes() for p in targets])
    native=tmp/'native';(native/'firmware/src/dsp').mkdir(parents=True)
    completed=subprocess.run([CLI,'tools','dy-build',str(manifest),str(banks),str(native)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
    check('native-dy-files',expected[2],[(native/p.relative_to(target)).read_bytes() for p in targets])
    check('native-dy-output',expected[0][1].replace(str(target),'<OUT>'),completed.stdout.decode().replace('\r\n','\n').replace(str(native),'<OUT>'))
    # Logo byte APIs, shape errors, centering and clipped paste with actual Pillow.
    ui=tmp/'ui.c';source=(BASE/'firmware/src/app/ui.c').read_text(encoding='utf-8');ui.write_text(source,encoding='utf-8')
    oldlogo.UI=logo.UI=str(ui);rows=oldlogo.sprite();check('source-sprite',rows,logo.sprite())
    for i in range(70):
        grid=[''.join(rng.choice('.KTHhYRrL?') for _ in range(23)) for _ in range(23)]
        scale=rng.randrange(0,5);size=rng.choice([None,0,1,12,23,24,31,32,93])
        def image(m):
            im=m.render(grid,scale,size);return im.mode,im.size,im.tobytes()
        check('logo-rgba-%d'%i,capture(lambda:image(oldlogo)),capture(lambda:image(logo)))
    for scale,size in [(-1,None),(1,-1),(1,None),(0,None)]:
        check('logo-png-%d-%s'%(scale,size),capture(lambda:oldlogo.png(oldlogo.render(rows,scale,size))),capture(lambda:logo.png(logo.render(rows,scale,size))))
    for grid in [['.'*23]*24,['.'*23]*23+['K'],['K'*24]]:
        check('logo-row-edge-'+str(len(cases)),capture(lambda:oldlogo.render(grid,1).tobytes()),capture(lambda:logo.render(grid,1).tobytes()))
    saved_pal=copy.deepcopy(logo.PAL)
    for module in (oldlogo,logo):module.PAL['.']=(1,2,3)
    check('mutable-logo-palette',oldlogo.render(['.'],2).tobytes(),logo.render(['.'],2).tobytes())
    for module in (oldlogo,logo):module.PAL=copy.deepcopy(saved_pal)
    for body in ['',source.replace('MASCOT[23] = {','MASCOT[24] = {'),source.replace('MASCOT[23] = {','MASCOT[23] = {"short",')]:
        ui.write_text(body,encoding='utf-8');check('logo-bad-source-%d'%len(cases),capture(oldlogo.sprite),capture(logo.sprite))
    ui.write_text(source,encoding='utf-8')
    html='<img class="logo" src="old"><img class="logo" src="two\nlines"><img class="other" src="keep"><link rel="icon" type="image/png" href="old"><img class="logo" src="unfinished'
    for i in range(100):
        a=bytes(rng.randrange(256) for _ in range(i));b=a[::-1]
        want=oldlogo.re.sub(r'(<img class="logo" src=")[^"]*(")',lambda m:m[1]+'data:image/png;base64,'+base64.b64encode(a).decode()+m[2],html)
        want=oldlogo.re.sub(r'(<link rel="icon" type="image/png" href=")[^"]*(")',lambda m:m[1]+'data:image/png;base64,'+base64.b64encode(b).decode()+m[2],want)
        check('logo-inline-%d'%i,want,rust_tooling.call('asset.logo.inline',[html,list(a),list(b)]))
    for inline in (False,True):
        logo_out=tmp/'logo';(logo_out/'tools').mkdir(parents=True,exist_ok=True);(logo_out/'web/editor').mkdir(parents=True,exist_ok=True)
        def run_logo(module):
            trace=[];traced(module,trace);module.HERE=str(logo_out/'tools');module.UI=str(ui)
            for name in ['amb_installer.html','editor/index.html']:(logo_out/'web'/name).write_text(html,encoding='utf-8')
            sys.argv=['make_logo.py']+(['--inline'] if inline else [])
            result=capture(module.main)
            files={str(p.relative_to(logo_out)):p.read_bytes() for p in sorted((logo_out/'web').rglob('*')) if p.is_file()}
            return result,trace,files
        check('logo-main-'+str(inline),run_logo(oldlogo),run_logo(logo))
    for failed in ['slinky.png','favicon.png','editor'+os.sep+'index.html']:
        def run_failed(module):
            trace=[];traced(module,trace,lambda p,a:failed in p and a and 'w' in a[0])
            sys.argv=['make_logo.py','--inline']
            for name in ['amb_installer.html','editor/index.html']:(logo_out/'web'/name).write_text(html,encoding='utf-8')
            return capture(module.main),trace
        check('logo-write-refusal-'+failed,run_failed(oldlogo),run_failed(logo))
    for name in ['amb_installer.html','editor/index.html']:
        page=(BASE/'web'/name).read_text(encoding='utf-8')
        a=oldlogo.png(oldlogo.render(rows,4));b=oldlogo.png(oldlogo.render(rows,1,32))
        want=oldlogo.re.sub(r'(<img class="logo" src=")[^"]*(")',lambda m:m[1]+'data:image/png;base64,'+base64.b64encode(a).decode()+m[2],page)
        want=oldlogo.re.sub(r'(<link rel="icon" type="image/png" href=")[^"]*(")',lambda m:m[1]+'data:image/png;base64,'+base64.b64encode(b).decode()+m[2],want)
        check('full-html-inline-'+name,want,rust_tooling.call('asset.logo.inline',[page,list(a),list(b)]))
    raw=tmp/'logo.rgba';done=subprocess.run([CLI,'tools','logo-rgba',str(ui),'1','32',str(raw)],stdout=subprocess.PIPE,check=True)
    check('native-logo-rgba',oldlogo.render(rows,1,32).tobytes(),raw.read_bytes())
    check('native-logo-metadata',dict(width=32,height=32,rgba_bytes=4096),json.loads(done.stdout))

rust_tooling.close();rust_authoring.close()
print(json.dumps(dict(cases=len(cases),trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),
    frozen_files=440,python=sys.version.split()[0],pillow=PIL.__version__,physical_io=False)))
