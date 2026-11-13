# SPDX-License-Identifier: GPL-3.0-only
"""Actual build.check against the immutable Python gate; no compiler/device I/O."""
from pathlib import Path
import hashlib,importlib.util,json,os,random,sys,tempfile
from unittest.mock import patch
from contextlib import nullcontext
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve()
os.environ['FM1_TOOLING_CLI']=str(Path(sys.argv[2]).resolve())
os.environ['FM1_CODEC_CLI']=os.environ['FM1_TOOLING_CLI']
sys.path.insert(0,str(ROOT/'tools'))
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
package=load('frozen_package',BASE/'tools/fm1pkg_make.py');lz4=load('frozen_lz4',BASE/'tools/lz4blk.py')
with patch.dict(sys.modules,{'fm1pkg_make':package,'lz4blk':lz4}):frozen=load('frozen_build',BASE/'tools/build.py')
sys.path.insert(0,str(ROOT/'tools'))
import build as current
assert Path(current.__file__).resolve()==ROOT/'tools/build.py'
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1);assert hashlib.sha256((BASE/name.lstrip('*')).read_bytes()).hexdigest()==digest
def syms(start=0x2001000,bss=98304,pool=339968,extra=''):
    return ''.join('%08x g .text 00000000 %s\n'%(value,name)for value,name in[(start,'_start'),(0x1c08000+bss,'_bss_end'),(0x1000,'_pool_start'),(0x1000+pool,'_pool_end')])+extra
def instruction(text):return ' 2001000: 00 00 00 00 \t'+text
trace=hashlib.sha256();cases=0
def compare(image,symbols,disassembly,ram):
    global cases
    expected=frozen.check(image,symbols,disassembly,ram);actual=current.check(image,symbols,disassembly,ram)
    assert actual==expected,dict(case=cases,expected=expected,actual=actual,dis=disassembly,ram=ram)
    trace.update(json.dumps(actual,ensure_ascii=True,separators=(',',':')).encode());cases+=1
lengths=[0,1,3,4,0xee0-1,0xee0,0xee0+1,0xee0+0x8c000-1,0xee0+0x8c000,0xee0+0x8c000+1,0x8dfbc,0x8dfbd]
for n in lengths:
    for prefix in [bytes.fromhex('04818000'),b'FAIL']:
        image=prefix[:n]if n<4 else prefix+bytes(n-4)
        compare(image,syms(),'','')
for bss in [-1,0,98303,98304,98305,131072]:
    for pool in [-1,0,339967,339968,339969,344064,344065]:compare(bytes.fromhex('04818000'),syms(bss=bss,pool=pool),'','')
instructions=['= -4194304 <x>','= 4290772992 <x>','= 4291821567 <x>','= 4291821568 <x>',
    '= 18446744078000324608 <x>','call 7 <f: ffc00000 >','call -7 <f: ffcfffff >','call 0 <f: ffd00000 >',
    'call 1 <f: 100000000ffc00000 >','rts','callout','recall','call','bad '+chr(39)+'call'+chr(39),
    '"call"','\\call','écall','callé','call\u0301','call\x1f']
instructions += ['call\xa0','call\u200b','call\ue000','call\U000e0001',
    '= \u0664\u0662\u0669\u0660\u0667\u0667\u0662\u0669\u0669\u0662 <x>',
    '= \uff14\uff12\uff19\uff10\uff17\uff17\uff12\uff19\uff19\uff12 <x>',
    '= \U0001d7dc\U0001d7da\U0001d7e1\U0001d7d8\U0001d7df\U0001d7df\U0001d7da\U0001d7e1\U0001d7e1\U0001d7da <x>']
for text in instructions:
    compare(bytes.fromhex('04818000'),syms(),instruction(text),instruction(text)+'\n'+text)
randomizer=random.Random(91)
soft=['__adddf3','__muldf3','__extendsfdf2','__truncdfsf2','__cmpdf2','__floatsidf','__fixunsdfsi','__divdf3']
for i in range(512):
    chosen=randomizer.sample(soft,randomizer.randrange(9));extra=''.join('12345678 g .text 00000000 '+name+'\n'for name in chosen+chosen[:2])
    start=randomizer.choice([0,0x2001000,0x2001004]);bss=randomizer.randrange(-128,100000);pool=randomizer.randrange(330000,350000)
    dis='\n'.join(instruction(randomizer.choice(instructions[:11]))for _ in range(randomizer.randrange(5)))
    ram='\n'.join(randomizer.choice([instruction('rts'),instruction('call 1 <x>'),'recall','callout','plain'])for _ in range(randomizer.randrange(6)))
    image=bytes.fromhex('04818000')+bytes(randomizer.randrange(4000))
    compare(image,syms(start,bss,pool,extra),dis,ram)
image_cases=cases

read_text=Path.read_text
def mmio(path,utf8=False):
    global cases
    # Controlled Unicode probes supply the same decoded source on both
    # platforms. The two real trees above retain native filesystem decoding.
    reader=patch.object(Path,'read_text',lambda self,*args,**kwargs:read_text(self,encoding='utf-8')) if utf8 else nullcontext()
    with reader:
        with patch.object(frozen,'FW',path):expected=frozen.mmio_check()
        with patch.object(current,'FW',path):actual=current.mmio_check()
    assert actual==expected,dict(case=cases,expected=expected,actual=actual)
    trace.update(json.dumps(actual,ensure_ascii=True,separators=(',',':')).encode());cases+=1

# Both real source trees plus controlled filesystem snapshots exercise the
# actual adapters, including native path spelling/order and newline decoding.
mmio(BASE/'firmware');mmio(ROOT/'firmware')
parent=ROOT/'build/build-guards';parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='mmio-',dir=str(parent)) as folder:
    source=Path(folder).resolve();assert source.parent==parent.resolve()
    for directory in ['hal','src/sub','loader']:(source/directory).mkdir(parents=True,exist_ok=True)
    header=source/'hal/registers.h';override=source/'hal/z-last.h'
    inputs=[source/'src/A.c',source/'src/sub/z.h',source/'loader/test.c']
    definitions=[
        '#define REG (*(volatile unsigned *)0x10000)\n#define MID REG\n#define ALIAS(x) MID\n',
        '#define REG (*(volatile unsigned *)0x10000)\n#define MID ALIAS\n#define ALIAS MID\n',
        '#define REG volatile\n#define MID REG\n#define ALIAS MID\n#define REG ordinary\n',
        '#define REG volatile\n#define MID REG\n#define ALIAS MID\n',
        '#define REG\x1fvolatile\n#define MID REG\n#define ALIAS MID\n',
        '#define REG volatile\u0301\n#define MID REG\n#define ALIAS MID\n',
        '#define REG notvolatile\n#define MID REG\n#define ALIAS MID\n',
        '/* #define REG volatile */\n#define MID REG\n#define ALIAS MID\n',
    ]
    values=['0x10000','0x13fffUL','0x14000','0x40000','0x43fff','0x44000','0x50000','0x51fff','0x52000',
        '0x1ee0000','0x1eeffff','0x1ef0000','0x1c7f000','0x1c7ffff','0x1c80000','0x2000000','0x20fffff','0x2100000',
        '0X00010000uL','x0x10000','0x10000x','\u00e90x10000','0x10000\u00e9','0x10000\u0301',
        '(volatile unsigned *)0','( const volatile unsigned * )ptr','(volatile unsigned *)&flag','(volatile unsigned *)\x1f &flag',
        '(volatilex *)0','(volatile\u0301 *)0','(volatile unsigned * volatile *)0','(volatile (unsigned) *)0',
        'asm("nop")','__asm__("nop")','__asm__ volatile ("" ::: "memory")','asm volatile("":::"memory")',
        'asm volatile ("" :: "memory")','asm\x1fvolatile ("" ::: "memory")','asm\u0301("nop")','reasm asmX',
        'REG MID ALIAS','REG\u0301','REG\u00e9','\u00e9REG','// 0x10000 REG asm("nop")',
        '/* 0x10000\nREG asm("nop")\n */ safe;\n0x10000',
        'asm("nop"); (volatile int *)0x10000; ALIAS',
        '/* unterminated 0x10000 ALIAS','"//" 0x10000 ALIAS',
    ]
    for index in range(320):
        header.write_text(definitions[index%len(definitions)],encoding='utf-8')
        override.write_text('#define REG ordinary\n' if index%13==0 else '',encoding='utf-8')
        selected=[values[(index*7+part*13)%len(values)] for part in range(7)]
        breaks=['\n','\r\n','\r','\v','\f','\x1c','\x1d','\x1e','\x85','\u2028','\u2029']
        for part,path in enumerate(inputs):
            path.write_text(breaks[(index+part)%len(breaks)].join(selected[part:]+selected[:part]),encoding='utf-8')
        mmio(source,utf8=True)
result=dict(cases=cases,image_cases=image_cases,mmio_cases=cases-image_cases,source_hashes=440,sha256=trace.hexdigest())
out=ROOT/'build/build-guards'/('windows' if os.name=='nt' else 'linux');out.mkdir(parents=True,exist_ok=True);(out/'differential.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
