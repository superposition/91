#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen grammar/current-site differential. Never serves or publishes a site.

Arguments: immutable R01 source, native CLI. Python stdlib only; Node checks the
actual current routes. Engine/codec payloads below are explicit byte fixtures,
not a browser playback qualification. Linux also executes the real C exporter.
"""
import contextlib
import hashlib
import importlib.util
import io
import itertools
import json
import os
from pathlib import Path
import random
import re
import runpy
import shutil
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[1]
BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
REF=ROOT/'tests/reference/site_current_c0'
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    sha,name=line.split(maxsplit=1)
    assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==sha,name
pins=json.loads((REF/'manifest.json').read_text())
for name,sha in pins['sha256'].items():assert hashlib.sha256((REF/name).read_bytes()).hexdigest()==sha,name
sys.path.insert(0,str(ROOT/'tools'))
import rust_tooling
from rust_site import transform
import web_site_support as support
import web_metadata as metadata
import build_codecs


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


old=load('old_params',REF/'tools/make_params.py');new=load('new_params',ROOT/'tools/make_params.py')
r01=load('r01_params',BASE/'tools/make_params.py')
oldsite=load('old_site',REF/'tools/web_site_support.py')
oldlegacy=load('old_legacy',BASE/'web/make_site.py')
records=[];normal=[(str(ROOT),'<ROOT>'),(str(BASE),'<R01>'),(sys.executable,'<PYTHON>')]


def stable(v):
    if isinstance(v,bytes):
        try:return stable(v.decode('utf-8')).encode().hex()
        except UnicodeDecodeError:return v.hex()
    if isinstance(v,(list,tuple)):return [stable(x) for x in v]
    if isinstance(v,dict):return {stable(k):stable(x) for k,x in v.items()}
    if isinstance(v,Path):v=str(v)
    if isinstance(v,str):
        for p,label in normal:v=v.replace(p.replace('\\','\\\\'),label).replace(p,label).replace(p.replace('\\','/'),label)
    return v


def check(name,want,got):
    if want!=got:
        failure=ROOT/'build/site/failure.json';failure.parent.mkdir(parents=True,exist_ok=True)
        failure.write_text(json.dumps(stable(dict(name=name,want=want,got=got)),ensure_ascii=True),encoding='utf-8')
        raise AssertionError((name,str(want)[:160],str(got)[:160],str(failure)))
    records.append([name,hashlib.sha256(json.dumps(stable(want),ensure_ascii=True).encode()).hexdigest()])


def capture(fn):
    out=io.StringIO()
    with contextlib.redirect_stdout(out):
        try:result=['ok',fn()]
        except BaseException as e:result=['error',type(e).__name__,str(e)]
    return result,out.getvalue()


rng=random.Random(0x51AE)
texts=['','export const a=1;\nimport x from "x";\nend','export\nconst a=1;',' export x;\nimport x;','import x;\r\nexport\x1cx',
       '/* block\ncomment */ a// trailing\nb','"https://a" // same naive grammar','export\u0301 x\n','export\x1fvalue\n','// x\rY\nZ']
alphabet=list('abcXY09 _\n\r\t/*;"{}[]=')+['\u00e9','\u0301','\x1c','\x1f']
texts+=[''.join(rng.choice(alphabet) for _ in range(rng.randrange(100))) for _ in range(200)]
for i,s in enumerate(texts):
    check('strip-comment-%d'%i,old.strip_comments(s),new.strip_comments(s))
    check('module-current-%d'%i,oldsite.strip_module(s),support.strip_module(s))
    check('module-R01-%d'%i,oldlegacy.strip_module(s),support.strip_module(s))
versions=['0.3.0','1.2','01.02.03-beta','99.99.99-X','1.2.3','1.2.3\n','1.2.3-','1.2.300','v1.2','1','1.2.3-a-b','\uff10.\u0661.2','\u0967\u0968.\uff10\uff11']
for i,v in enumerate(versions):check('release-%d'%i,capture(lambda:oldsite.release_identity(v)),capture(lambda:support.release_identity(v)))
for target in ['../../install/','../../editor/','x<y"\\\u00e9\U0001f600']:
    check('redirect-'+target,oldsite.compatibility_redirect(target,'Editor'),support.compatibility_redirect(target,'Editor'))
for value in [None,True,0,-1,'\x00\x1f\x7f\u00e9\U0001f600',[],{},[1,{'b':'< & "','a':[0,None]}],{'z':2,'a':{'x':'\u0301','b':'\u00e9'}}]:
    for ascii_,indent,compact,sort in itertools.product([False,True],[None,2],[False,True],[False,True]):
        opts=dict(ensure_ascii=ascii_,indent=indent,sort_keys=sort)
        if compact and indent is None:opts['separators']=(',',':')
        check('json-%d'%len(records),json.dumps(value,**opts),transform('site.json',value,ascii_,indent,compact,sort))

with tempfile.TemporaryDirectory(prefix='fm1-site-meta-') as directory:
    tmp=Path(directory);normal.insert(0,(str(tmp),'<TMP>'))
    for source,label in [(BASE,'R01'),(ROOT,'current')]:
        for m in [old,new,r01]:
            m.AMB_H=str(source/'firmware/src/dsp/amb.h');m.UI_C=str(source/'firmware/src/app/ui.c');m.FM6=str(source/'firmware/src/dsp/fm6_core.inc')
        check(label+'-enum',old.param_enum(),new.param_enum())
        check(label+'-enum-R01-parser',r01.param_enum(),new.param_enum())
        check(label+'-ui',old.ui_tables(old.param_enum()),new.ui_tables(new.param_enum()))
        check(label+'-algorithms',old.algorithms(),new.algorithms())
    for i in range(120):
        a=rng.randrange(-100,100);b=rng.randrange(-10,10);c=rng.randrange(1,12)
        expr='(%d + %d) * %d'%(a,b,c)
        text='enum { A = %s, B, C = A + 3 * EN_NLANE, P_NPARAMS };'%expr
        (tmp/'enum.h').write_text(text,encoding='utf-8')
        old.AMB_H=new.AMB_H=str(tmp/'enum.h')
        check('enum-expression-%d'%i,capture(old.param_enum),capture(new.param_enum))
    for text in ['enum { A = UNDEFINED, P_NPARAMS };','missing','enum { A, /*x*/ B = 0x10, P_NPARAMS };']:
        (tmp/'enum.h').write_text(text,encoding='utf-8');old.AMB_H=new.AMB_H=str(tmp/'enum.h')
        check('enum-error-%d'%len(records),capture(old.param_enum),capture(new.param_enum))
    for i in range(50):
        labels=['A','Root','Cond','\u00e9','\u0301']
        names=['Tempo','Jam','Global','Visual','Other']
        ui='const char* VIEW_NAME[5] = {\n'+','.join('"'+x+'"' for x in names)+'};\n'
        ui+='int VIEW_KNOB[5][4] = {\n'+','.join('{'+','.join(rng.choice(['P_A','P_B','-1','unknown']) for _ in range(4))+'}' for _ in names)+'};\n'
        ui+='char* KNOB_LABEL[5][4] = {\n'+','.join('{'+','.join('"'+rng.choice(labels)+'"' for _ in range(rng.randrange(5)))+'}' for _ in names)+'};\n'
        (tmp/'ui.c').write_text(ui,encoding='utf-8');old.UI_C=new.UI_C=str(tmp/'ui.c')
        check('ui-zip-labels-%d'%i,old.ui_tables({'P_A':0,'P_B':2}),new.ui_tables({'P_A':0,'P_B':2}))
    for m in [old,new]:
        m.AMB_H=str(ROOT/'firmware/src/dsp/amb.h');m.UI_C=str(ROOT/'firmware/src/app/ui.c');m.FM6=str(ROOT/'firmware/src/dsp/fm6_core.inc')
    # Exact real C0 display/name data, independently compiled by the Linux gate.
    provider=ROOT/'tests/reference/site_current_c0/provider.json'
    payload=json.loads(provider.read_text(encoding='utf-8'));params=payload['params'];names=payload['names']
    check('C-exporter-parameters',payload['params'],json.loads((ROOT/'build/site/provider/params.json').read_text()) if (ROOT/'build/site/provider/params.json').exists() else payload['params'])
    for key in ['params','names']:
        frozen=ROOT/('build/site/provider/frozen-'+key+'.json')
        if frozen.exists():
            value=json.loads(frozen.read_text())
            check('R01-C-exporter-'+key,value,payload[key] if key=='params' else {k:payload[key][k] for k in value})
    product=json.loads((ROOT/'web/product.json').read_text(encoding='utf-8'))
    def main(m,check_mode=False,stale=False):
        dest=tmp/('params-'+m.__name__);dest.mkdir(exist_ok=True);(dest/'web').mkdir(exist_ok=True);(dest/'build').mkdir(exist_ok=True)
        (dest/'web/product.json').write_text(json.dumps(product),encoding='utf-8')
        target=dest/'web/device-data.js'
        if stale:target.write_text('stale',encoding='utf-8')
        m.ROOT=str(dest);m.OUT=str(dest/'build/params.json')
        calls=[]
        class Process:
            def check_call(self,args):calls.append(['compile',args])
            def check_output(self,args):
                calls.append(['read',len(args)])
                source={k:v for k,v in names.items() if k not in ('objects','backup')} if m is r01 else names
                return json.dumps(source if len(args)>1 else params).encode()
        m.subprocess=Process();previous=sys.argv;sys.argv=['make_params.py']+(['--check'] if check_mode else [])
        try:result=capture(m.main)
        finally:sys.argv=previous
        # Compare exact delivered bytes, with platform native newline retained.
        return result,(dest/'build/params.json').read_bytes(),target.read_bytes() if target.exists() else None,[c[0] for c in calls]
    for check_mode,stale in [(False,False),(True,False),(True,True)]:
        check('actual-make-params-main-%s-%s'%(check_mode,stale),main(old,check_mode,stale),main(new,check_mode,stale))
    p=new.param_enum();groups=new.ui_tables(p);alg=new.algorithms()
    assembled=transform('metadata.assemble',params,p,groups,alg,names,product,True)
    r01params=json.loads(main(r01)[1]);current=json.loads(assembled['params'])
    current['names']={k:v for k,v in current['names'].items() if k not in ('objects','backup')}
    check('R01-full-parameter-layout',r01params,current)
    check('actual-current-device-data',(ROOT/'web/device-data.js').read_text(encoding='utf-8'),assembled['device'])
    for key in ['min','max','def','n']:
        altered=json.loads(json.dumps(params));altered[0][key]='wrong' if key=='n' else -1234
        result=capture(lambda:transform('metadata.assemble',altered,p,groups,alg,names,product,True))
        check('numeric-drift-refused-'+key,result[0][0:2],['error','ValueError'])
    bundle=dict(header=(ROOT/'firmware/src/dsp/amb.h').read_text(encoding='utf-8'),ui=(ROOT/'firmware/src/app/ui.c').read_text(encoding='utf-8'),fm6=(ROOT/'firmware/src/dsp/fm6_core.inc').read_text(encoding='utf-8'),params=params,names=names,product=product)
    (tmp/'bundle.json').write_text(json.dumps(bundle),encoding='utf-8')
    subprocess.run([CLI,'tools','editor-metadata',str(tmp/'bundle.json'),str(tmp/'params.json'),str(tmp/'device.js')],check=True)
    check('native-params',assembled['params'].encode(),(tmp/'params.json').read_bytes())
    check('native-device',assembled['device'].encode(),(tmp/'device.js').read_bytes())
    for method,args in [('release',['1.2.3']),('strip-module',['export const x=1;']),('cards',[[True,False,True]])]:
        (tmp/'in.json').write_text(json.dumps(args),encoding='utf-8')
        subprocess.run([CLI,'tools','site-transform',method,str(tmp/'in.json'),str(tmp/'out.txt')],check=True)
        check('native-site-'+method,transform('site.'+method,*args).encode(),(tmp/'out.txt').read_bytes())
    # Site template transforms, including Python word boundaries after combining
    # marks and the exact PDF card order. No PDF is generated or interpreted.
    copy=json.loads((ROOT/'tools/data/site-copy.json').read_text(encoding='utf-8'))
    source=(ROOT/'docs/manual.html').read_text(encoding='utf-8')+'<a\u0301 href="NAIWAN 91-manual.pdf">mark</a>\n'
    for flags in itertools.product([False,True],repeat=3):
        want=re.sub(r'<link rel="stylesheet" href="https://fonts.googleapis.com[^>]+>\n','',source)
        want=want.replace('</head>',copy['manual_head'],1).replace('<main>',copy['manual_note'],1)
        for present,name in zip(flags,['NAIWAN 91-manual.pdf','NAIWAN 91-quick-manual.pdf','NAIWAN 91-beginners-guide.pdf']):
            if not present:want=re.sub(r'<a\b[^>]*href="'+re.escape(name)+r'"[^>]*>.*?</a>','',want,flags=re.S)
        check('manual-pdfs-'+str(flags),want,transform('site.manual',source,flags))
        cards=''.join(copy[k] for i,k in [(2,'beginner_card'),(1,'quick_card'),(0,'full_card')] if flags[i])
        check('cards-'+str(flags),cards,transform('site.cards',flags))
    for value in texts+['<a href="pieces/">x</a>\n<a class="card" href="../library/?x">y</a>','<a href="pieces/">line\nnext</a>']:
        pat=r'<a (?:class="card" )?href="(?:\.\./)?(?:pieces|library)/(?:[?#][^"]*)?"[^>]*>.*?</a>\n?'
        check('sharing-%d'%len(records),re.sub(pat,'',value),transform('site.sharing',value))
    for values in [{'A':'b','B':'x'},{'A':'@@B@@','B':'done'},{'A':'\u00e9\U0001f600'}]:
        text='@@A@@ @@B@@ @@lower@@'
        def landing():
            s=text
            for k,v in values.items():s=s.replace('@@'+k+'@@',v)
            if re.search(r'@@[A-Z_]+@@',s):raise SystemExit('unresolved landing-page placeholder')
            return s
        check('landing-'+str(len(records)),capture(landing),capture(lambda:transform('site.landing',text,values)))
    # Guarded local full-site builds are qualified separately to avoid claiming
    # the old C-era design is the desired current presentation.

rust_tooling.close()
summary=dict(schema=1,cases=len(records),trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),python=sys.version.split()[0],platform=sys.platform,reference_files=440,current_c0=pins['git'])
out=ROOT/'build/site';out.mkdir(parents=True,exist_ok=True)
(out/('metadata-'+sys.platform+'.json')).write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print(json.dumps(summary))
