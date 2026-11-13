#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Manual HTML against R01 and its byte-identical current-C0 source pins."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve();CLI=str(Path(sys.argv[2]).resolve())
os.environ.update(FM1_TOOLING_CLI=CLI,FM1_CODEC_CLI=CLI)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    sha,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==sha,name
pins=json.loads((ROOT/'tests/reference/manual_current_c0/manifest.json').read_text())
for name,pin in pins['files'].items():assert hashlib.sha256((BASE/'tools'/name).read_bytes()).hexdigest()==pin['sha256']
sys.path.insert(0,str(ROOT/'tools'))
import rust_tooling


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


old=load('old_manual',BASE/'tools/manual_html.py');new=load('new_manual',ROOT/'tools/manual_html.py')
oldquick=load('old_quick',BASE/'tools/make_quick_manual.py');quick=load('new_quick',ROOT/'tools/make_quick_manual.py')
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
    out=io.StringIO()
    with contextlib.redirect_stdout(out):
        try:value=['ok',fn()]
        except BaseException as e:value=['error',type(e).__name__,str(e)]
    return value,out.getvalue()


def check(name,want,got):
    if want!=got:
        p=ROOT/'build/manual/failure.json';p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(json.dumps(stable(dict(name=name,want=want,got=got))),encoding='utf-8')
        raise AssertionError((name,str(want)[:250],str(got)[:250],str(p)))
    cases.append(name);records.append([name,hashlib.sha256(json.dumps(stable(want)).encode()).hexdigest()])


check('manual-css',old.CSS,new.CSS);check('quick-css',oldquick.CSS,quick.CSS);check('name',old.NAME,new.NAME)
check('edge-default',oldquick.EDGE,quick.EDGE);check('wanted-pages',oldquick.WANT_PAGES,quick.WANT_PAGES)
snippets=['',"<&>\"'",'`**a** *x* [link](u)`','a **bold** and *italics*','x**b**y','a*b*c',
          '* x* ** x ** **x**','[a & b](u?a&b) [empty]()','***three***',"`a'b\"`",'\x000\x00',
          'a\u0301*x* b\u0301*x* x*italic*','é*x* e\u0301*x* é**x**',
          '²*x* ٣*x* _*x* \u0345*x*', '*x*\x1f**y**\x1c*z*','\u0130\u00c5A\u030a ẞ ß',
          'zero\x00one', '`one`\x000\x00','*x\nline* **x\nline**']
rng=random.Random(0xD0C)
alphabet=list('abcXYZ09 _*`[]()<>\'"&.-\n\t')+['é','\u0301','\x1c','\x1f']
snippets += [''.join(rng.choice(alphabet) for _ in range(rng.randrange(1,80))) for _ in range(180)]
for i,s in enumerate(snippets):
    check('inline-full-%d'%i,capture(lambda:old.inline(s)),capture(lambda:new.inline(s)))
    check('inline-quick-%d'%i,capture(lambda:oldquick.inline(s)),capture(lambda:quick.inline(s)))
    check('slug-%d'%i,capture(lambda:old.slug(s)),capture(lambda:new.slug(s)))
for colors in [{},{'Pad':'#123456','Bass':'red'},{'P.d':'pink'}]:
    old.PART_COLOURS=new.PART_COLOURS=colors
    for s in ['**Pad** and **Bass**','**PXd**','**Bass** / **Pad**']:
        check('swatch-'+str(len(cases)),capture(lambda:old.inline(s)),capture(lambda:new.inline(s)))
old.PART_COLOURS=new.PART_COLOURS={}
tables=[[],['| a | b |'],['| a | b |','|---|:--:|','|`x`|**y**|'],['|||'],['a|b','x|y'],
        ['| a | b |','| | |','| x | y |'],['|a|','|:--|'],['|a|','|-|'],['||a||','|--|--|','||z||']]
for i,t in enumerate(tables):check('quick-table-%d'%i,capture(lambda:oldquick.table(t)),capture(lambda:quick.table(t)))
blocks=['', '# Title\n\nA paragraph\ncontinued.\n\n## Child',
        '# NAIWAN 91\n\n**Status.** Works\n\n## Contents\n\n1. First\n2. Other\n---\n## 1. First\n### À A\n- one\n  next\n2. mixed',
        '> Callout\n> next\n\n<details><summary>**Open**</summary>\n\ninside\n</details>',
        '```c\nint x=1 < 2;\n```\n\n````\nunterminated',
        '![A **caption**](img/a.png)\n\n![wide](hyp-hero.svg)\n\n\n![x](img/a.png)',
        '![malformed\n\n## Next','<div class="cols">\ntext\n</div>',
        '## First\rSecond\vThird\fFourth\x1cFifth\x1dSixth\x1eSeventh\x85Eight\u2028Nine\u2029Ten\n',
        '<!-- one\ncomment -->\n# Heading\n\n<!-- unclosed',
        '| Head | Two |\n|---|:--:|\n|x|**y**|\n\n- item\n   continued\n* next\n\n1. ordered\n2. next',
        '<!-- footer: x \\ " & < -->\n# Quick\n---\n## Page2\n<div class="page"></div>\nlast',
        '#\x1cHeader\n\n1.\x1fitem\n2. next\n\n**bold** *em*']
tokens=['# Top','## 1. Number','## Plain','### Third','text & < >','', '|a|b|','|---|---|','|x|y|',
        '- one','1. two','  continuation','> call','</details>','---','<p>raw</p>','<!-- c -->','`x` **b** *e*']
blocks += ['\n'.join(rng.choice(tokens) for _ in range(rng.randrange(1,20))) for _ in range(100)]
for i,s in enumerate(blocks):
    check('full-blocks-%d'%i,capture(lambda:old.convert(s)),capture(lambda:new.convert(s)))
    check('quick-blocks-%d'%i,capture(lambda:oldquick.blocks(s)),capture(lambda:quick.blocks(s)))
    check('quick-document-%d'%i,capture(lambda:oldquick.convert(s)),capture(lambda:quick.convert(s)))

with tempfile.TemporaryDirectory(prefix='fm1-manual-') as directory:
    tmp=Path(directory);paths.insert(0,(str(tmp),'<TMP>'))
    (tmp/'img').mkdir();(tmp/'img/a.png').write_bytes(b'pixel fixture');(tmp/'hyp-hero.svg').write_bytes(b'<svg/>')
    old.IMG_BASE=new.IMG_BASE=tmp
    oldquick.IMG_BASE=quick.IMG_BASE=tmp
    for embed in [False,True]:
        old.EMBED=new.EMBED=embed
        for caption,src in [('a & " < **bold**','img/a.png'),('wide','hyp-hero.svg'),('missing','absent.png')]:
            check('figure-'+str(len(cases)),capture(lambda:old.figure(caption,src)),capture(lambda:new.figure(caption,src)))
    check('data-uri',old.data_uri(tmp/'img/a.png'),new.data_uri(tmp/'img/a.png'))
    s='![x](img/../img/a.png)\n\n![spaces](img/missing %25 é.png)'
    check('quick-figure-uri',capture(lambda:oldquick.blocks(s)),capture(lambda:quick.blocks(s)))
    for embed in [False,True]:
        old.EMBED=new.EMBED=embed
        check('embedded-group-'+str(embed),capture(lambda:old.convert(blocks[5])),capture(lambda:new.convert(blocks[5])))
    for source_root,label in [(BASE,'R01'),(ROOT,'current')]:
        old.IMG_BASE=new.IMG_BASE=source_root/'docs';old.EMBED=new.EMBED=False
        oldquick.IMG_BASE=quick.IMG_BASE=None
        for name in ['MANUAL.md','QUICK.md','GUIDE.md']:
            md=(source_root/'docs'/name).read_text(encoding='utf-8')
            check(label+'-full-'+name,old.convert(md),new.convert(md))
            check(label+'-quick-'+name,oldquick.convert(md),quick.convert(md))
    old.EMBED=new.EMBED=False;oldquick.IMG_BASE=quick.IMG_BASE=None
    for module in [old,new,oldquick,quick]:module.SRC=tmp
    (tmp/'docs').mkdir();(tmp/'docs/MANUAL.md').write_text('# NAIWAN 91\n\nHello.',encoding='utf-8')
    (tmp/'docs/QUICK.md').write_text('# Quick\n\nBody',encoding='utf-8')
    for args in [[],['--fragment'],[str(tmp/'docs/MANUAL.md'),str(tmp/'out.html')],['absent.md','--fragment']]:
        target=tmp/'out.html' if len(args)>=2 and args[1].endswith('.html') else tmp/'docs/manual.html'
        def run(m):
            target.write_bytes(b'sentinel');sys.argv=['manual_html.py']+args
            value=capture(m.main);return value,target.read_bytes(),m.EMBED
        check('full-main-'+str(len(cases)),run(old),run(new))
    for args in [['--html-only'],['--edge'],['--out'],['--font'],['absent.md','--html-only'],['--font','11pt','--out','guide','--html-only']]:
        def run(m):
            value=capture(lambda:m.main(args));return value,str(m.IMG_BASE)
        check('quick-main-'+str(len(cases)),run(oldquick),run(quick))
    text_='# NAIWAN 91\n\nHello\n\n## 1. World';src=tmp/'native.md';src.write_text(text_,encoding='utf-8')
    old.PART_COLOURS={};oldquick.IMG_BASE=None;old.EMBED=False
    for cmd,expected in [('manual-body',old.convert(text_)),('quick-html',oldquick.convert(text_))]:
        dst=tmp/(cmd+'.html');subprocess.run([CLI,'tools',cmd,str(src),str(dst)],check=True)
        check('native-'+cmd,expected.encode('utf-8'),dst.read_bytes())
    process_roots=[]
    for label,source in [('frozen',BASE),('current',ROOT)]:
        tree=tmp/label;(tree/'tools/data').mkdir(parents=True)
        for name in ['manual_html.py','make_quick_manual.py']:
            shutil.copyfile(source/'tools'/name,tree/'tools'/name)
        if label=='current':
            for name in ['rust_manual.py','rust_tooling.py','rust_codecs.py']:
                shutil.copyfile(ROOT/'tools'/name,tree/'tools'/name)
            for name in ['manual-copy.json','manual.css','quick-manual.css']:
                shutil.copyfile(ROOT/'tools/data'/name,tree/'tools/data'/name)
        process_roots.append(tree)
    for script,arguments in [('manual_html.py',[str(src),str(tmp/'process.html')]),
                             ('manual_html.py',[str(src),str(tmp/'process.html'),'--fragment']),
                             ('make_quick_manual.py',[str(src),'--html-only'])]:
        # Quick's normal output directory is repo/build. Use --out's absolute
        # stem to keep both actual process entry points confined to scratch.
        if script=='make_quick_manual.py':arguments += ['--out',str(tmp/'process')]
        target=tmp/'process.html'
        def run_process(root):
            target.write_bytes(b'sentinel')
            completed=subprocess.run([sys.executable,'-I',str(root/'tools'/script)]+arguments,
                                     stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            last=completed.stderr.decode(errors='replace').splitlines()
            return completed.returncode,completed.stdout,last[-1] if last else '',target.read_bytes()
        check('process-cli-'+script+'-'+str(len(cases)),run_process(process_roots[0]),run_process(process_roots[1]))

rust_tooling.close()
print(json.dumps(dict(cases=len(cases),trace_sha256=hashlib.sha256(json.dumps(records).encode()).hexdigest(),
    frozen_files=440,current_c0_base=pins['base'],python=sys.version.split()[0],actual_pdf_rendering=False)))
