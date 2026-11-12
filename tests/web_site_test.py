#!/usr/bin/env python3
"""Check the built site's real routes/assets and parse its shipped JavaScript.

python3 tests/web_site_test.py build/site-web-review [build/emu] [--no-sharing]
"""
from html.parser import HTMLParser
from collections import Counter
import json
import hashlib
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urljoin, urlsplit

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from web_site_support import product_of, release_identity, verify_owned_site
arguments=[value for value in sys.argv[1:] if value!='--no-sharing']
SHARING='--no-sharing' not in sys.argv
OUT=Path(arguments[0] if arguments else ROOT/'build/site-web-review').resolve()
STANDALONE=Path(arguments[1] if len(arguments)>1 else ROOT/'build/emu').resolve()

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

routes=['index.html','emu/index.html','install/index.html','editor/index.html','pieces/index.html','library/index.html','manual/index.html','webapp/installer/index.html','webapp/editor/index.html']
if not SHARING:routes=[name for name in routes if name not in ('pieces/index.html','library/index.html')]
id_contract=json.loads((ROOT/'tests/web-required-ids.json').read_text(encoding='utf-8'))
required={name:ids for name,ids in id_contract['pages'].items() if name in routes}
removed=id_contract.get('intentionally_removed',{})
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
    retired=set(removed.get(name,{}))
    assert retired<=set(required.get(name,[])),f'unknown retired IDs: {name}'
    assert not retired&set(parsed.ids),f'retired engineering UI returned: {name}'
    expected=set(required.get(name,[]))-retired
    assert expected<=set(parsed.ids),f'missing original control IDs in {name}: {expected-set(parsed.ids)}'
    assert not re.search(r'@@[A-Z_]+@@|/\*(?:PARAMS|META|LIB|LIBRARY|RELAY|DEVICE)\*/',text),f'unresolved build data: {name}'
    assert any('site.css' in link for link in parsed.links),f'missing shared theme: {name}'
    assert 'fonts.googleapis.com' not in text,f'external font dependency: {name}'
    for style in parsed.styles:assert '{{' not in style,f'invalid doubled-opening-brace CSS: {name}'
    for link in parsed.links:
        target=local_target(page,link,OUT)
        if target is not None:
            mounted=urljoin('https://example.invalid/nested/91/'+name,link)
            assert mounted.startswith('https://example.invalid/nested/91/'),f'link escapes hosted subpath in {name}: {link}'
            assert target.is_file(),f'missing asset/route in {name}: {link} -> {target}';links+=1
    for script in parsed.scripts:
        result=subprocess.run(['node','--check','--input-type=module'],input=script.encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        assert result.returncode==0,f'JavaScript parse failed in {name}: {result.stderr.decode()}';scripts+=1
css=(OUT/'site.css').read_bytes()
assert css==(ROOT/'web/site.css').read_bytes()
modules=0
for location in [OUT/'emu',STANDALONE]:
    page=location/'index.html';parsed=Page(page.read_text(encoding='utf-8'))
    for link in parsed.links:
        if 'site.css' in link or 'fonts/' in link or 'naiwan91-mark.svg' in link:
            target=local_target(page,link,OUT if location==OUT/'emu' else location)
            assert target and target.is_file(),f'standalone emulator asset missing: {link}'
    if location==OUT/'emu':
        assert not (location/'site.css').exists() and not (location/'fonts').exists() and not (location/'img').exists(),'full site must share root assets once'
    else:
        assert (location/'site.css').read_bytes()==css
    assert 'src/app.mjs' in parsed.links,'missing emulator module entry point'
    for source in sorted((ROOT/'web/emu/src').glob('*.mjs')):
        bundled=location/'src'/source.name
        assert bundled.read_bytes()==source.read_bytes(),f'stale or missing browser module: {bundled}'
        code=bundled.read_text(encoding='utf-8')
        result=subprocess.run(['node','--check',str(bundled)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        assert result.returncode==0,f'module parse failed: {bundled}: {result.stderr.decode()}'
        for match in re.finditer(r'''(?:from\s*|import\(|new URL\()(["'])(\.\.?/[^"']+)\1''',code):
            target=local_target(bundled,match.group(2),location)
            assert target and target.is_file(),f'missing module dependency: {bundled}: {match.group(2)}'
        modules+=1
css_assets=0
for location in [OUT,STANDALONE]:
    for stylesheet in location.rglob('*.css'):
        for asset in re.findall(r'''url\(\s*["']?([^\s)"']+)''',stylesheet.read_text(encoding='utf-8')):
            target=local_target(stylesheet,asset,location)
            if target is not None:
                assert target.is_file(),f'missing CSS dependency: {stylesheet}: {asset}'
                css_assets+=1
verify_owned_site(OUT)
record=json.loads((OUT/'site-build.json').read_text(encoding='utf-8'))
release=record['release'];package=OUT/'firmware'/release['firmware'];raw=package.read_bytes()
assert release['firmware']==f"naiwan91-{release['version']}.fwsc",'release filename/version drift'
assert hashlib.sha256(raw).hexdigest()==release['firmware_sha256'],'release package hash drift'
assert product_of(raw)==release['product'],'release product identity drift'
assert release_identity(release['version'])==release['product'],'release version/product drift'
assert not (OUT/'app').exists(),'archived prototype must not be shipped'
readme=(ROOT/'README.md').read_text(encoding='utf-8')
assert 'docs/img/readme-' not in readme,'obsolete README imagery returned'
captures=json.loads((ROOT/'docs/img/naiwan91-captures.json').read_text(encoding='utf-8'))
assert len(captures['captures'])==5
for capture in captures['captures']:
    assert (OUT/'img'/capture['file']).read_bytes()==(ROOT/'docs/img'/capture['file']).read_bytes()
print(json.dumps(dict(routes=len(routes),local_links_and_assets=links,parsed_scripts=scripts,parsed_module_copies=modules,css_asset_dependencies=css_assets,hosted_subpath=True,manifest_verified=True,shared_theme=True,standalone_assets=True,current_captures=5,preserved_control_ids=sum(map(len,required.values()))-sum(map(len,removed.values())),retired_engineering_ids=sum(map(len,removed.values())),baseline_manual_duplicate_ids=manual_duplicates)))
