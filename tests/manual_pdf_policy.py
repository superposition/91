#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Deterministic PDF/process/filesystem doubles. No browser or actual PDF."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import re
import subprocess
import sys
import tempfile
import types

ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    sha,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==sha,name
sys.path.insert(0,str(ROOT/'tools'))
import rust_tooling


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


old=load('old_quick_pdf',BASE/'tools/make_quick_manual.py');new=load('new_quick_pdf',ROOT/'tools/make_quick_manual.py')
sys.modules['make_quick_manual']=old;oldfull=load('old_full_pdf',BASE/'tools/make_manual_pdf.py')
sys.modules['make_quick_manual']=new;full=load('new_full_pdf',ROOT/'tools/make_manual_pdf.py')
cases=[];records=[];paths=[(sys.executable,'<PYTHON>'),(str(ROOT),'<ROOT>'),(str(BASE),'<R01>')]


def stable(v):
    if isinstance(v,bytes):
        try:return stable(v.decode('utf-8')).encode('utf-8').hex()
        except UnicodeDecodeError:return v.hex()
    if isinstance(v,(list,tuple)):return [stable(x) for x in v]
    if isinstance(v,dict):return {stable(k):stable(x) for k,x in v.items()}
    if isinstance(v,Path):v=str(v)
    if isinstance(v,str):
        for p,label in paths:v=v.replace(p.replace('\\','\\\\'),label).replace(p,label).replace(p.replace('\\','/'),label)
    return v


def capture(fn):
    output=io.StringIO()
    with contextlib.redirect_stdout(output):
        try:value=['ok',fn()]
        except BaseException as e:value=['error',type(e).__name__,str(e)]
    return value,output.getvalue()


def check(name,want,got):
    if want!=got:
        dest=ROOT/'build/manual/pdf-failure.json';dest.parent.mkdir(parents=True,exist_ok=True)
        dest.write_text(json.dumps(stable(dict(name=name,want=want,got=got))),encoding='utf-8')
        raise AssertionError((name,str(want)[:350],str(got)[:350],str(dest)))
    cases.append(name);records.append([name,hashlib.sha256(json.dumps(stable(want)).encode()).hexdigest()])


class FS:
    def __init__(self,root,growth,initial=False,pages=None,payload=None,fail=None,env=None):
        self.root=root;self.growth=growth;self.tick=0;self.phase=False;self.trace=[];self.pages=pages
        self.active_pdf=None;self.payload=payload if payload is not None else b'/Type/Page x'
        self.pdf=str(root/'build/quick-manual.pdf');self.fullpdf=str(root/'build/manual.pdf')
        self.files={str(root/'docs/QUICK.md'):b'# Quick\n\nText',str(root/'docs/manual.html'):b'<html>',
                    self.pdf:self.payload,self.fullpdf:self.payload}
        self.initial=initial;self.fail=fail
        self.env=env or {}
    def step(self,kind,*values):
        self.trace.append([kind,*values])
        if self.fail==kind:raise OSError('deliberate '+kind+' failure')
    def value(self):return self.growth[min(max(self.tick-1,0),len(self.growth)-1)]
    def install(self,module):
        fs=self;pages=self.pages
        class FakePath:
            def __init__(self,p):self.p=Path(str(p))
            def __str__(self):return str(self.p)
            def __truediv__(self,p):return FakePath(self.p/str(p))
            @property
            def parent(self):return FakePath(self.p.parent)
            def resolve(self):fs.step('resolve',str(self));return FakePath(self.p.resolve())
            def as_uri(self):fs.step('uri',str(self));return self.p.as_uri()
            def exists(self):
                fs.step('exists',str(self))
                if fs.phase and str(self)==fs.active_pdf:return fs.value() is not None
                if str(self) in [fs.pdf,fs.fullpdf]:return fs.initial
                return str(self) in fs.files
            def unlink(self):fs.step('unlink',str(self));fs.initial=False
            def stat(self):
                fs.step('stat',str(self));return types.SimpleNamespace(st_size=fs.value() if fs.phase and str(self)==fs.active_pdf else len(fs.files[str(self)]))
            def read_bytes(self):fs.step('read',str(self));return fs.files[str(self)]
            def read_text(self,**kw):fs.step('read_text',str(self),kw);return fs.files[str(self)].decode('utf-8')
            def write_text(self,s,**kw):fs.step('write_text',str(self),s,kw);fs.files[str(self)]=s.encode('utf-8');return len(s)
            def mkdir(self,**kw):fs.step('mkdir',str(self),kw)
        class Process:
            DEVNULL=subprocess.DEVNULL
            @staticmethod
            def run(cmd,**kw):
                fs.step('run',cmd,kw)
                if fs.fail=='timeout':raise subprocess.TimeoutExpired(cmd,120)
                if fs.fail=='called':raise subprocess.CalledProcessError(7,cmd)
                if '--headless' in cmd:
                    fs.phase=True;fs.tick=0;fs.active_pdf=next(v.partition('=')[2] for v in cmd if v.startswith('--print-to-pdf='));fs.files[fs.active_pdf]=fs.payload
                return types.SimpleNamespace(returncode=0 if kw.get('check') else 23)
        def sleep(n):fs.step('sleep',n);fs.tick+=1
        module.Path=FakePath;module.SRC=FakePath(self.root);module.HERE=FakePath(self.root/'tools')
        module.subprocess=Process;module.time=types.SimpleNamespace(sleep=sleep)
        module.os=types.SimpleNamespace(environ=self.env)
        if pages is None:
            sys.modules['pypdf']=None
        else:
            class Reader:
                def __init__(self,path):
                    fs.step('pdf_open',path)
                    if fs.fail=='constructor-import':raise ImportError('constructor import failure')
                @property
                def pages(self):
                    fs.step('pages')
                    class Page:
                        def __init__(self,i,value):self.i,self.value=i,value
                        @property
                        def mediabox(self):
                            page=self
                            class Box:
                                @property
                                def height(self):fs.step('height',page.i);return page.value.get('height',792)
                            return Box()
                        def extract_text(self,visitor_text=None):
                            fs.step('metrics' if visitor_text else 'text',self.i)
                            if visitor_text:
                                for item in self.value.get('items',[]):visitor_text(item[0],item[1],item[2],None,12)
                            return self.value.get('text')
                    return [Page(i,p) for i,p in enumerate(pages)]
            sys.modules['pypdf']=types.SimpleNamespace(PdfReader=Reader)
        return FakePath


with tempfile.TemporaryDirectory(prefix='fm1-pdf-policy-') as directory:
    root=Path(directory);paths.insert(0,(str(root),'<TMP>'))
    def run(module,method,growth=(100,100),initial=False,pages=None,payload=None,fail=None,args=None,relative=False,env=None):
        fs=FS(root,list(growth),initial,pages,payload,fail,env);P=fs.install(module)
        if module is oldfull:fs.install(old)
        if module is full:fs.install(new)
        if method=='to_pdf':fn=lambda:module.to_pdf(P('relative.html' if relative else root/'build/source.html'),P(fs.pdf),'edge')
        elif method in ['measure','page_report']:fn=lambda:getattr(module,method)(P(fs.pdf))
        else:fn=lambda:module.main(args or [])
        value=capture(fn)
        return value,fs.trace,dict(fs.files),str(getattr(module,'IMG_BASE',None))
    for i,growth in enumerate([[100,100],[None,0,100,100],[1,2,3,4,5,5],[0]*120,[None]*120,list(range(1,121)),[10,None,10],[10,0,10,10]]):
        for initial in [False,True]:
            check('to-pdf-growth-%d-%s'%(i,initial),run(old,'to_pdf',growth,initial),run(new,'to_pdf',growth,initial))
    for fail in ['exists','unlink','run','sleep','stat','timeout','called']:
        check('to-pdf-error-'+fail,run(old,'to_pdf',initial=True,fail=fail),run(new,'to_pdf',initial=True,fail=fail))
    check('relative-uri-refusal',run(old,'to_pdf',relative=True),run(new,'to_pdf',relative=True))
    # Distinct byte regex profiles: full report consumes a following non-s byte,
    # quick fallback checks an ASCII word boundary without consuming it.
    vectors=[b'',b'/Type/Page',b'/Type/Pages',b'/Type/Page/Type/Page!',b'/Type\t\n/Page!',b'/Type\x1c/Page!']
    vectors += [b'/Type/Page'+bytes([b])+b'/Type /Pages /Type/Page!' for b in range(256)]
    rng=random.Random(0xD0C0)
    vectors += [bytes(rng.choice(b'/Type Pagess\n\r\t!X09_') for _ in range(200)) for _ in range(80)]
    for i,data in enumerate(vectors):
        for full_profile in [False,True]:
            expected=len(re.findall(rb'/Type\s*/Page[^s]' if full_profile else rb'/Type\s*/Page\b',data))
            check('count-%d-%s'%(i,full_profile),expected,rust_tooling.call('asset.manual.count',[list(data),full_profile]))
        if i<12:check('missing-reader-report-%d'%i,run(old,'page_report',payload=data or b'x'),run(new,'page_report',payload=data or b'x'))
    check('missing-reader-measure',run(old,'measure'),run(new,'measure'))
    fixtures=[[],[dict(text=None)],[dict(text='\n  \nHello\nsecond')],
              [dict(text='α'*80),dict(text='\x1c A\u2028B\n')]]
    for i,pages in enumerate(fixtures):check('page-text-%d'%i,run(old,'page_report',pages=pages),run(new,'page_report',pages=pages))
    def item(text,y,x=0,cm=None):return [text,cm or [1,0,0,1,0,0],[1,0,0,1,x,y]]
    measures=[[],[dict(items=[])],
        [dict(items=[item('Top',750),item('Body',350),item('page 1 of 8',20)])],
        [dict(items=[item('continued',700)]),dict(items=[item('Next',740),item('bottom',200),item('page 2 of 8',20)])],
        [dict(height='792',items=[item('Zulu',600),item('Alpha',600),item('page ٣ of ٨',-0.0),item('body',7)])],
        [dict(items=[[' ',[],[]],item('page 1 of 8',10)])],
        [dict(items=[item('top',12),item('page 1 of 8',10),item('body',16)])],
        [dict(items=[item('transform',30,40,[1,2,3,4,5,6]),item('page 1 of 8',0)])],
        [dict(items=[['bad',[1],[1]]])]]
    for i,pages in enumerate(measures):check('measure-%d'%i,run(old,'measure',pages=pages),run(new,'measure',pages=pages))
    for i in range(50):
        items=[item('text%d'%j,rng.choice([-0.0,0.0,10.5,100.5,700.25,rng.uniform(-40,800)]),rng.uniform(-50,50),
                    [1,rng.choice([0,0.5,-0.25]),0,rng.choice([1,2,-1]),0,rng.choice([0,6,-6])]) for j in range(8)]
        items.append(item('page 1 of 8',rng.choice([-0.0,0,10])))
        pages=[dict(height=rng.choice([792,842,841.89]),items=items)]
        check('measure-numeric-%d'%i,run(old,'measure',pages=pages),run(new,'measure',pages=pages))
    for i,total in enumerate([0.5,1.5,2.5,266.5,267.0,267.00000000000006,267.49999999999994,267.5,268.5]):
        y=842-((total-3)/25.4*72+12*72/25.4)
        pages=[dict(height=842,items=[item('boundary',y),item('page 1 of 8',20)])]
        check('measure-boundary-%d'%i,run(old,'measure',pages=pages),run(new,'measure',pages=pages))
    for method in ['measure','page_report']:
        for fail in ['pdf_open','pages','height','text','metrics','read','constructor-import']:
            check(method+'-error-'+fail,run(old,method,pages=measures[2],fail=fail),run(new,method,pages=measures[2],fail=fail))
    for n in [0,7,8,9]:
        pages=[dict(text='Page '+str(i),items=[item('first',700),item('page %d of %d'%(i+1,n),20)]) for i in range(n)]
        for args in [[],['--measure'],['--out','guide','--font','11pt'],['--html-only']]:
            check('quick-main-%d-%s'%(n,str(args)),run(old,'main',pages=pages,args=args),run(new,'main',pages=pages,args=args))
    for args in [['--edge'],['--font'],['--out'],['missing.md','--html-only']]:
        check('quick-args-'+str(args),run(old,'main',args=args),run(new,'main',args=args))
    for args in [[],['--edge','override']]:
        check('quick-env-edge-'+str(args),run(old,'main',env={'PERF1_EDGE':'environment'},args=args),run(new,'main',env={'PERF1_EDGE':'environment'},args=args))
    for fail in ['mkdir','resolve','read_text','write_text','stat','run']:
        check('quick-main-error-'+fail,run(old,'main',fail=fail),run(new,'main',fail=fail))
    for n in [0,10,11,18]:
        data=b'/Type/Page x '*n
        for args in [[],['--edge','other']]:
            check('full-main-%d-%s'%(n,str(args)),run(oldfull,'main',payload=data or b'x',args=args),run(full,'main',payload=data or b'x',args=args))
    for fail in ['run','exists','unlink','sleep','read','timeout','called']:
        check('full-main-error-'+fail,run(oldfull,'main',initial=True,fail=fail),run(full,'main',initial=True,fail=fail))
    for args in [[],['--edge','override']]:
        check('full-env-edge-'+str(args),run(oldfull,'main',env={'PERF1_EDGE':'environment'},args=args),run(full,'main',env={'PERF1_EDGE':'environment'},args=args))
    check('full-missing-edge',run(oldfull,'main',args=['--edge']),run(full,'main',args=['--edge']))
    raw=root/'fake-pdf-bytes.bin';raw.write_bytes(b'/Type/Page/Type/Page!')
    for flag in [[],['--full']]:
        value=json.loads(subprocess.check_output([CLI,'tools','pdf-count',str(raw)]+flag))
        check('native-count-'+str(flag),dict(pages=1 if flag else 2,bytes=len(raw.read_bytes())),value)

rust_tooling.close()
print(json.dumps(dict(cases=len(cases),trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),
    frozen_files=440,python=sys.version.split()[0],edge_launched=False,pdf_rendered=False)))
