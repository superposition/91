# SPDX-License-Identifier: GPL-3.0-only
"""Actual loader/stub builders with compiler delivery doubles; never executes tools."""
from pathlib import Path
import contextlib,hashlib,importlib.util,io,json,os,random,sys,tempfile
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve()
os.environ['FM1_TOOLING_CLI']=str(Path(sys.argv[2]).resolve());os.environ['FM1_CODEC_CLI']=os.environ['FM1_TOOLING_CLI']
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
package=load('image_frozen_package',BASE/'tools/fm1pkg_make.py');lz4=load('image_frozen_lz4',BASE/'tools/lz4blk.py')
with patch.dict(sys.modules,{'fm1pkg_make':package,'lz4blk':lz4}):frozen=load('image_frozen_build',BASE/'tools/build.py')
sys.path.insert(0,str(ROOT/'tools'));import build as current
assert Path(current.__file__).resolve()==ROOT/'tools/build.py'
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.lstrip('*')).read_bytes()).hexdigest()==digest
STUB=0xee0
def stub(n=128,version=3):
    body=bytes.fromhex('04818000')+bytes(4)+b'CRSTUB'+version.to_bytes(2,'little')
    return (body+bytes(max(0,n-len(body))))[:n]
def symbols(start=0x02000120):return '%08x g .text 00000000 _start\n'%start
def instruction(text):return ' 2000120: 00 00 00 00 \t'+text
def normalize(value,workspace):
    if isinstance(value,bytes):return dict(bytes=len(value),sha256=hashlib.sha256(value).hexdigest())
    if isinstance(value,str):return value.replace(str(workspace),'<workspace>')
    if isinstance(value,list):return [normalize(v,workspace)for v in value]
    if isinstance(value,dict):return {k:normalize(v,workspace)for k,v in value.items()}
    return value
trace=hashlib.sha256();cases=0;counts={}
parent=ROOT/'build/build-images/cases';parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='compiler-delivery-',dir=str(parent)) as folder:
    workspace=Path(folder).resolve();assert workspace.parent==parent.resolve()
    out=workspace/'build';fw=workspace/'firmware';stb=out/'stub';ldr=out/'loader'
    for path in [stb,ldr,fw/'stub',fw/'loader']:path.mkdir(parents=True,exist_ok=True)
    def run(module,kind,row):
        lock=fw/'stub/stub.lock'
        for path in [lock,stb/'stub.bin',stb/'stub.dis',ldr/'loader.bin',ldr/'loader.dis',ldr/'ota.bin']:
            if path.exists():path.unlink()
        if row.get('lock')=='match':
            padded=row['image']+b'\xff'*max(0,STUB-len(row['image']))
            lock.write_text(str(int.from_bytes(padded[14:16],'little'))+' '+hashlib.sha256(padded).hexdigest()+'\n')
        elif row.get('lock') is not None:lock.write_text(row['lock'])
        calls=[]
        def args(values):return [str(v.relative_to(workspace)) if isinstance(v,Path)else v for v in values]
        def tc(*command):calls.append(['tc']+args(command));return ''
        def tc_all(*commands):
            calls.append(['tc_all']+[args(c)for c in commands])
            if commands[0][0]=='common/bin/objcopy':
                if row.get('deliver',True):Path(commands[0][-1]).write_bytes(row['image'])
                return '',row.get('dis',''),row.get('headers',''),row.get('symbols','')
            return ['' for _ in commands]
        output=io.StringIO();result={}
        with contextlib.ExitStack() as stack:
            for name,value in dict(SRC=workspace,FW=fw,OUT=out,STB=stb,LDR=ldr,VERSION=row.get('release'),tc=tc,tc_all=tc_all).items():stack.enter_context(patch.object(module,name,value))
            stack.enter_context(patch.dict(os.environ,{'FM1_STUB_LOCK':row.get('update','')}))
            stack.enter_context(contextlib.redirect_stdout(output))
            try:result['return']=getattr(module,'build_'+kind)()
            except BaseException as error:result['error']=[type(error).__name__,str(error)]
        result.update(stdout=output.getvalue(),calls=calls,lock=lock.read_bytes()if lock.exists()else None,
            dis=(stb/'stub.dis'if kind=='stub'else ldr/'loader.dis').read_bytes()if(stb/'stub.dis'if kind=='stub'else ldr/'loader.dis').exists()else None,
            ota=(ldr/'ota.bin').read_bytes()if(ldr/'ota.bin').exists()else None)
        return result
    def compare(kind,row):
        global cases
        expected=run(frozen,kind,row);actual=run(current,kind,row)
        assert actual==expected,dict(case=cases,kind=kind,row=normalize(row,workspace),expected=normalize(expected,workspace),actual=normalize(actual,workspace))
        trace.update(json.dumps(normalize(actual,workspace),sort_keys=True,separators=(',',':')).encode());cases+=1;counts[kind]=counts.get(kind,0)+1
    for n in [0,1,3,4,13,14,15,16,128,STUB-257,STUB-256,STUB-255,STUB,STUB+1]:
        for version in [0,3,65535]:compare('stub',dict(image=stub(n,version),symbols=symbols(),lock='match'))
    for lock in [None,'match','bad hash\n','3 '+('a'*64)+' extra\n']:
        for update in ['', 'update','UPDATE']:
            for release in [None,'','0.3.3']:compare('stub',dict(image=stub(),symbols=symbols(),lock=lock,update=update,release=release))
    for used in [238*odd for odd in range(1,14,2)]:
        compare('stub',dict(image=stub(used),symbols=symbols(),lock='match'))
    headers=['','header line','0 .text 00000020 02000120 0','1 .data 00000000 01c08000 0','1 .data 00000001 01c08000 0',
        '2 .bss 00000100 01c08000 0','3 .symtab 00000010 00000000 0','4 .strtab 00000010 00000000 0',
        '5 .shstrtab 00000010 00000000 0','6 .other -1 00000000 0','0 .data 0x10 0x0 0','10 .data -+1 0 0']
    for header in headers:
        for start in [0,0x2000120,0x2001000]:
            for dis in ['',instruction('= -4194304 <rom>'),instruction('call 7 <f: ffcfffff >')]:
                compare('stub',dict(image=stub(),headers=header,symbols=symbols(start),dis=dis,lock='match'))
    for image in [b'',b'x',bytes(4095),bytes(4096),bytes(4097),bytes(0x14000),bytes(0x14001)]:
        for syms in ['', '01c0b000 g .bss 00000000 _bss_end\n','-1 g .bss 0 _bss_end\n','0 g 0 one_bss_end\n01c0b000 g 0 _bss_end\n']:
            compare('loader',dict(image=image,symbols=syms))
    for header in ['0 .text 00000010 01c0a800 0','1 .text 00000000 02000000 0','2 .data 00000001 02000000 0',
        '3 .data 00000010 01ffffff 0','4 .data 0x10 0x2000000 0','5 .bss -1 02000000 0','0 .text bad bad 0','0 .text 0 bad 0',
        '6 .data -+1 02000000 0','7 .data --1 02000000 0','8 .data 0x_1 0x_2000000 0','9 .data 0x__1 02000000 0']:
        for deliver in [False,True]:compare('loader',dict(image=bytes(128),headers=header,deliver=deliver))
    compare('stub',dict(image=stub(),symbols=symbols(),headers='1 .data 1 0 0',deliver=False))
    compare('stub',dict(image=stub(),symbols=symbols(),deliver=False))
    rng=random.Random(91)
    for index in range(128):
        n=rng.randrange(16,STUB+20);image=stub(n,rng.randrange(65536))
        if index%11==0:image=b'FAIL'+image[4:]
        if index%13==0:image=image[:8]+b'BADSTB'+image[14:]
        compare('stub',dict(image=image,symbols=symbols(rng.choice([0x2000120,0])),headers=rng.choice(headers),
            dis=instruction(rng.choice(['rts','= 4290772992 <rom>','= 4291821568 <outside>'])),lock=rng.choice([None,'match','wrong']),update=rng.choice(['','update']),release=rng.choice([None,'0.3.3'])))
        data=bytes(rng.randrange(256)for _ in range(4096))if index%3==0 else bytes(rng.randrange(4097))
        compare('loader',dict(image=data,headers=rng.choice(['','0 .text 10 01c0a800 0','0 .text 1 02000000 0']),symbols=rng.choice(['','01c0b000 g 0 _bss_end\n','bad! g 0 _bss_end\n'])))
    class Delivered(Exception):pass
    def run_main(module,value,prior_product,prior_version,sdk=None):
        output=io.StringIO();errors=io.StringIO();result={}
        def observed():
            frame=sys._getframe(2)
            return dict(product=module.PRODUCT,version=module.VERSION,name=frame.f_locals['name'])
        def sdk_file(name):
            if sdk is None:raise Delivered(observed())
            return sdk[name][0]
        def mkdir(path,*args,**kwargs):
            assert path==out
            raise Delivered(observed())
        with contextlib.ExitStack()as stack:
            for name,assigned in dict(PRODUCT=prior_product,VERSION=prior_version,OUT=out).items():stack.enter_context(patch.object(module,name,assigned))
            stack.enter_context(patch.object(module.fm1pkg_make,'sdk_file',sdk_file))
            stack.enter_context(patch.object(module.fm1pkg_make,'SDK',None))
            if sdk is not None:
                stack.enter_context(patch.object(module,'SDK_SHA256',{k:v[1]for k,v in sdk.items()}))
                stack.enter_context(patch.object(Path,'mkdir',mkdir))
            stack.enter_context(patch.object(sys,'argv',['build.py']+([]if value is None else['--release='+value])))
            stack.enter_context(contextlib.redirect_stdout(output));stack.enter_context(contextlib.redirect_stderr(errors))
            try:module.main()
            except Delivered as delivered:result['metadata']=delivered.args[0]
            except BaseException as error:result['error']=[type(error).__name__,str(error)]
        return dict(result,stdout=output.getvalue(),stderr=errors.getvalue())
    def main_compare(value,prior_product='FM-1_700',prior_version=None,sdk=None):
        global cases
        expected=run_main(frozen,value,prior_product,prior_version,sdk);actual=run_main(current,value,prior_product,prior_version,sdk)
        assert actual==expected,dict(case=cases,value=value,expected=expected,actual=actual)
        trace.update(json.dumps(normalize(actual,workspace),sort_keys=True,separators=(',',':')).encode());cases+=1
        name='sdk'if sdk is not None else'release';counts[name]=counts.get(name,0)+1
    versions=[None,'','0.1','0.3.3','99.99.99','01.02.03-Rc1','1.2.3.4','100.1','-1.2','1.2-','1.2_foo','1.2-foo-bar','1.2\n',
        '\u0661.\u0662.\u0663-alpha','\uff11.\uff12.\uff13','\U0001d7d9.\U0001d7da.\U0001d7db','1.2-\u00df']
    versions+=[''.join(rng.choice('0123456789.-abCZ')for _ in range(rng.randrange(16)))for _ in range(64)]
    for value in versions:
        for prior_product,prior_version in [('FM-1_700',None),('CUSTOM','prior')]:main_compare(value,prior_product,prior_version)
    for index in range(32):
        sdk={}
        for part,name in enumerate(frozen.SDK_SHA256):
            data=bytes([part])*rng.randrange(4097);digest=hashlib.sha256(data).hexdigest()
            sdk[name]=(data,digest if(index>>part)&1 else digest.upper())
        main_compare('0.3.3'if index&1 else None,sdk=sdk)
result=dict(cases=cases,profiles=counts,source_hashes=440,sha256=trace.hexdigest(),compiler_executed=False,hardware_io=False)
dest=ROOT/'build/build-images'/('windows'if os.name=='nt'else'linux');dest.mkdir(parents=True,exist_ok=True)
(dest/'differential.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
