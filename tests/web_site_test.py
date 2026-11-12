#!/usr/bin/env python3
"""Check the built site's real routes/assets and parse its shipped JavaScript.

python3 tests/web_site_test.py build/site-web-review [build/emu]
"""
from html.parser import HTMLParser
from collections import Counter
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit

ROOT=Path(__file__).resolve().parents[1]
OUT=Path(sys.argv[1] if len(sys.argv)>1 else ROOT/'build/site-web-review').resolve()
STANDALONE=Path(sys.argv[2] if len(sys.argv)>2 else ROOT/'build/emu').resolve()

class Page(HTMLParser):
    def __init__(self,text):
        super().__init__();self.links=[];self.scripts=[];self.styles=[];self.ids=[];self.tag=None;self.buffer=[];self.script_type='';self.feed(text)
    def handle_starttag(self,tag,attributes):
        a=dict(attributes)
        if a.get('id'):self.ids.append(a['id'])
        for key in ('href','src'):
            if a.get(key):self.links.append(a[key])
        if tag in ('script','style') and not a.get('src'):
            self.tag=tag;self.buffer=[];self.script_type=a.get('type','')
    def handle_data(self,data):
        if self.tag:self.buffer.append(data)
    def handle_endtag(self,tag):
        if tag==self.tag:
            content=''.join(self.buffer)
            if tag=='style':self.styles.append(content)
            elif self.script_type not in ('application/json','application/ld+json'):self.scripts.append(content)
            self.tag=None;self.buffer=[]

def local_target(base,link,root):
    parsed=urlsplit(link)
    if parsed.scheme or parsed.netloc or not parsed.path:return None
    target=(root/unquote(parsed.path.lstrip('/')) if parsed.path.startswith('/') else base.parent/unquote(parsed.path)).resolve()
    if parsed.path.endswith('/'):target/= 'index.html'
    return target

routes=['index.html','emu/index.html','install/index.html','editor/index.html','pieces/index.html','library/index.html','manual/index.html']
required=json.loads((ROOT/'tests/web-required-ids.json').read_text(encoding='utf-8'))['pages']
scripts=0;links=0;manual_duplicates={}
for name in routes:
    page=OUT/name;text=page.read_text(encoding='utf-8');parsed=Page(text)
    assert re.search(r'<title>[^<]*(?:NAIWAN|内湾)[^<]*</title>',text),name
    duplicates={key:value for key,value in Counter(parsed.ids).items() if value>1}
    if name=='manual/index.html':
        # Theme the existing manual without silently masking content-renderer
        # defects. Preserve its exact pre-existing duplicate-id fingerprint.
        original=Page((ROOT/'docs/manual.html').read_text(encoding='utf-8'))
        baseline={key:value for key,value in Counter(original.ids).items() if value>1}
        assert duplicates==baseline,'new duplicate ids in served manual'
        manual_duplicates=baseline
    else:
        assert not duplicates,f'duplicate ids in {name}: {duplicates}'
    assert set(required.get(name,[]))<=set(parsed.ids),f'missing original control IDs in {name}: {set(required.get(name,[]))-set(parsed.ids)}'
    assert not re.search(r'@@[A-Z_]+@@|/\*(?:PARAMS|META|LIB|LIBRARY|RELAY)\*/',text),f'unresolved build data: {name}'
    assert any('site.css' in link for link in parsed.links),f'missing shared theme: {name}'
    assert 'fonts.googleapis.com' not in text,f'external font dependency: {name}'
    for style in parsed.styles:assert '{{' not in style,f'invalid doubled-opening-brace CSS: {name}'
    for link in parsed.links:
        target=local_target(page,link,OUT)
        if target is not None:
            assert target.is_file(),f'missing asset/route in {name}: {link} -> {target}';links+=1
    for script in parsed.scripts:
        result=subprocess.run(['node','--check','--input-type=module'],input=script.encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        assert result.returncode==0,f'JavaScript parse failed in {name}: {result.stderr.decode()}';scripts+=1
css=(OUT/'site.css').read_bytes()
assert css==(ROOT/'web/site.css').read_bytes()
for location in [OUT/'emu',STANDALONE]:
    page=location/'index.html';parsed=Page(page.read_text(encoding='utf-8'))
    for link in parsed.links:
        if 'site.css' in link or 'fonts/' in link or 'naiwan91-mark.svg' in link:
            target=local_target(page,link,location)
            assert target and target.is_file(),f'standalone emulator asset missing: {link}'
    assert (location/'site.css').read_bytes()==css
readme=(ROOT/'README.md').read_text(encoding='utf-8')
assert 'docs/img/readme-' not in readme,'obsolete README imagery returned'
captures=json.loads((ROOT/'docs/img/naiwan91-captures.json').read_text(encoding='utf-8'))
assert len(captures['captures'])==5
for capture in captures['captures']:
    assert (OUT/'img'/capture['file']).read_bytes()==(ROOT/'docs/img'/capture['file']).read_bytes()
print(json.dumps(dict(routes=len(routes),local_links_and_assets=links,parsed_scripts=scripts,shared_theme=True,standalone_assets=True,current_captures=5,preserved_control_ids=sum(map(len,required.values())),baseline_manual_duplicate_ids=manual_duplicates)))
