#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Current Spectrum builder versus pinned current C0 on owned local fixtures.

Does not start a server, deploy files, use a browser, or touch hardware. The
WASM/PDF/package contents are deterministic fixtures, not executable evidence.
"""
import hashlib
import importlib.util
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[1];CLI=str(Path(sys.argv[1]).resolve())
REF=ROOT/'tests/reference/site_current_c0'
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI)
sys.path.insert(0,str(ROOT/'tools'))
import build_codecs
import web_metadata
import web_site_support
import rust_tooling

records=[]


def check(name,a,b):
    if a!=b:
        (ROOT/'build/site/build-failure.json').write_text(json.dumps(dict(name=name,want=a,got=b),ensure_ascii=True,default=str),encoding='utf-8')
        raise AssertionError((name,str(a)[:160],str(b)[:160]))
    records.append([name,hashlib.sha256(json.dumps(a,sort_keys=True,default=str).encode()).hexdigest()])


def inventory(path):
    return {p.relative_to(path).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(path.rglob('*')) if p.is_file()}


def make_root(dest,old):
    dest.mkdir()
    for name in ['web','docs']:
        shutil.copytree(ROOT/name,dest/name)
    (dest/'tools').mkdir()
    for name in ['make_pages_site.py','web_site_support.py','web_metadata.py','build_codecs.py','rust_codecs.py','rust_tooling.py','rust_site.py']:
        source=REF/'tools'/name if old and (REF/'tools'/name).exists() else ROOT/'tools'/name
        shutil.copyfile(source,dest/'tools'/name)
    for name in ['Cargo.toml','Cargo.lock']:
        shutil.copyfile(ROOT/name,dest/name)
    for crate in ['fm1-types','fm1-formats','fm1-codecs-wasm']:
        (dest/'rust'/crate).mkdir(parents=True)
        shutil.copyfile(ROOT/'rust'/crate/'Cargo.toml',dest/'rust'/crate/'Cargo.toml')
        shutil.copytree(ROOT/'rust'/crate/'src',dest/'rust'/crate/'src')
    build=dest/'build';(build/'codecs').mkdir(parents=True)
    # Explicit fixtures: the static builder copies/embeds these bytes and never
    # instantiates them. Browser audio qualification belongs the existing gates.
    wasm=b'\0asm\x01\0\0\0'
    (build/'codecs/fm1-codecs.wasm').write_bytes(wasm)
    digest=build_codecs.source_hash(dest)
    (build/'codecs/manifest.json').write_text(json.dumps(dict(source=digest,sha256=hashlib.sha256(wasm).hexdigest())),encoding='utf-8')
    shutil.copytree(dest/'web/emu/src',build/'emu/src')
    for name in ['index.html','worklet.js']:shutil.copyfile(dest/'web/emu'/name,build/'emu'/name)
    s=(build/'emu/index.html').read_text(encoding='utf-8').replace('href="../site.css"','href="site.css"').replace('href="../fonts/','href="fonts/').replace('href="../img/naiwan91-mark.svg"','href="img/naiwan91-mark.svg"')
    (build/'emu/index.html').write_text(s,encoding='utf-8')
    (build/'emu/amb.wasm').write_bytes(wasm)
    shutil.copytree(dest/'web/fonts',build/'emu/fonts');(build/'emu/img').mkdir()
    shutil.copyfile(dest/'web/site.css',build/'emu/site.css')
    shutil.copyfile(dest/'web/img/naiwan91-mark.svg',build/'emu/img/naiwan91-mark.svg')
    shutil.copyfile(ROOT/'build/params.json',build/'params.json')
    raw=bytearray(1400)
    for i in range(20):raw[i*48+47]=125
    for i,c in enumerate(b'FM-1_7000300'):raw[i*48+47]=c+i+1
    raw[1100:1116]=b'FELUCCA-LOADER-1'
    (build/'naiwan91-0.3.0.fwsc').write_bytes(raw)


# Python3.8's pre-existing Path.write_text(newline=...) failure is measured with
# actual entrypoints first. Full builds use a test-only Python3.10 I/O shim on
# that host; maintained entrypoints are not changed to hide the incompatibility.
RUNNER='''import pathlib,runpy,sys,os
if sys.version_info < (3,10):
 def write(self,data,encoding=None,errors=None,newline=None):
  with self.open('w',encoding=encoding,errors=errors,newline=newline) as f:return f.write(data)
 pathlib.Path.write_text=write
if os.environ.get('FM1_SITE_PDF_GROWTH'):
 exists=pathlib.Path.exists
 observations={}
 def growing(self):
  if self.parent.name=='build' and self.name in ('manual.pdf','quick-manual.pdf','beginners-guide.pdf'):
   observations[self.name]=observations.get(self.name,0)+1
   if observations[self.name]==1:return False
  return exists(self)
 pathlib.Path.exists=growing
sys.argv=sys.argv[1:]
sys.path.insert(0,str(pathlib.Path(sys.argv[0]).parent))
runpy.run_path(sys.argv[0],run_name='__main__')
'''


def run(root,args=(),shim=True,growth=False):
    command=[sys.executable,'-c',RUNNER] if growth or (shim and sys.version_info<(3,10)) else [sys.executable]
    command += [str(root/'tools/make_pages_site.py'),'0.3.0',str(root/'build/site-output')]+list(args)
    env=dict(os.environ)
    if growth:env['FM1_SITE_PDF_GROWTH']='1'
    done=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=90)
    def norm(s):
        s=s.decode('utf-8').replace(str(root),'SRC').replace(str(root).replace('\\','/'),'SRC')
        # Tracebacks in intentional CLI refusals are otherwise genuine output.
        return s
    return done.returncode,norm(done.stdout),norm(done.stderr)


with tempfile.TemporaryDirectory(prefix='fm1-full-site-') as directory:
    tmp=Path(directory);old=tmp/'c0';new=tmp/'rust'
    make_root(old,True);make_root(new,False)
    # Actual historical Windows failure fingerprint is checked separately.
    if sys.version_info<(3,10):
        a=run(old,shim=False);b=run(new,shim=False)
        check('python38-native-newline-failure',('TypeError' in a[2],a[0]),('TypeError' in b[2],b[0]))
    for flags in itertools.product([False,True],repeat=3):
        for root in [old,new]:
            for present,name in zip(flags,['manual.pdf','quick-manual.pdf','beginners-guide.pdf']):
                path=root/'build'/name
                if present:path.write_bytes(b'PDF-copy-only-fixture:'+name.encode())
                elif path.exists():path.unlink()
        for sharing in [True,False]:
            args=[] if sharing else ['--no-sharing']
            a=run(old,args);b=run(new,args)
            check('full-build-status-%s-%s'%(flags,sharing),a,b)
            assert b[0]==0,b
            check('full-build-bytes-%s-%s'%(flags,sharing),inventory(old/'build/site-output'),inventory(new/'build/site-output'))
            # Actual current route/control/asset/JS/subpath assertions for both
            # sharing profiles, with ordinary Node syntax checks (no browser).
            if flags==(False,False,False):
                done=subprocess.run([sys.executable,str(ROOT/'tests/web_site_test.py'),str(new/'build/site-output'),str(new/'build/emu')]+args,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=os.environ,timeout=90)
                assert done.returncode==0,done.stderr.decode()
                check('current-route-contract-'+str(sharing),True,json.loads(done.stdout)['shared_theme'])
    # Each optional PDF is observed absent by the manual pass and present by the
    # later copy/card pass. The original independent existence checks matter.
    check('pdf-arrival-status',run(old,growth=True),run(new,growth=True))
    check('pdf-arrival-bytes',inventory(old/'build/site-output'),inventory(new/'build/site-output'))
    # The existing guarded publish implementation still owns directory moves.
    # Prove the ported helpers did not weaken its local-change refusal.
    for root in [old,new]:(root/'build/site-output/unknown.txt').write_text('local edit',encoding='utf-8')
    check('dirty-output-refusal',run(old),run(new))
    for root in [old,new]:(root/'build/site-output/unknown.txt').unlink()
    for required in ['build/params.json','build/emu/worklet.js','docs/manual.html']:
        saved=[]
        for root in [old,new]:
            path=root/required;saved.append(path.read_bytes());path.unlink()
        check('missing-'+required,run(old),run(new))
        for root,data in zip([old,new],saved):(root/required).write_bytes(data)
    # Runtime-source policy is live through web_metadata; preserve stale/hash
    # refusal and module assembly without touching root-owned build_codecs.
    for root in [old,new]:(root/'build/codecs/manifest.json').write_text('{"source":"stale"}',encoding='utf-8')
    def capture(fn):
        try:return ['ok',fn()]
        except Exception as e:return ['error',type(e).__name__,str(e)]
    check('runtime-stale',capture(lambda:build_codecs.runtime_source(old)),capture(lambda:web_metadata.runtime_source(new)))
    # Retired compatibility entrypoint remains a refusal and imports live Rust
    # module/product helpers; it never generates an obsolete website.
    for root in [old,new]:
        done=subprocess.run([sys.executable,str(root/'web/make_site.py')],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        assert done.returncode!=0 and b'is retired' in done.stderr
    check('retired-cli',True,True)

# Deterministic file-growth/read/error adapters compare the exact old runtime
# policy with the live Rust caller. Preserve its historical two separate reads.
def runtime_case(module,case):
    log=[];reads=[0]
    data=b'first';other=b'second read changes length'
    source='head\n// NODE_ONLY_BEGIN\nnode only\n// NODE_ONLY_END\n/*CODECS_BASE64*/ tail'
    record=dict(source='source',sha256=hashlib.sha256(data).hexdigest())
    if case=='stale-source':record['source']='wrong'
    if case=='stale-sha':record['sha256']='wrong'
    if case=='missing-marker':source='no node marker'
    class File:
        def __init__(self,name):self.name=str(name)
        def __str__(self):return self.name
        def __truediv__(self,name):return File(self.name+'/'+str(name))
        @property
        def parent(self):return File(self.name.rsplit('/',1)[0])
        def read_text(self,encoding=None):
            if self.name.endswith('manifest.json'):
                log.append('manifest')
                if case=='bad-json':return '{broken'
                if case=='wrong-json-type':return '[]'
                return json.dumps(record)
            log.append('source')
            if case=='source-error':raise OSError('source failure')
            return source
        def read_bytes(self):
            n=reads[0];reads[0]+=1;log.append('wasm-'+str(n))
            if case=='second-read-error' and n==1:raise OSError('second read failure')
            return other if case=='second-read-growth' and n==1 else data
    def digest(root):log.append('hash');return 'source'
    original_hash=module.source_hash;original_path=getattr(module,'Path',None)
    module.source_hash=digest
    if original_path is not None:module.Path=File
    try:value=capture(lambda:module.runtime_source(File('root')))
    finally:
        module.source_hash=original_hash
        if original_path is not None:module.Path=original_path
    return value,log


for case in ['valid','stale-source','stale-sha','missing-marker','bad-json','wrong-json-type','source-error','second-read-error','second-read-growth']:
    check('runtime-order-'+case,runtime_case(build_codecs,case),runtime_case(web_metadata,case))

rust_tooling.close()
summary=dict(schema=1,cases=len(records),trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),python=sys.version.split()[0],platform=sys.platform,full_build_profiles=16,wasm_and_pdf_payloads='deterministic copy-only fixtures',browser_or_server=False)
(ROOT/('build/site/build-'+sys.platform+'.json')).write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary))
