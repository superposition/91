# SPDX-License-Identifier: GPL-3.0-only
"""Live build/codec APIs against independent frozen Python; no compiler/device I/O."""
from pathlib import Path
import hashlib,importlib.util,json,os,random,sys,subprocess,tempfile
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve()
os.environ['FM1_TOOLING_CLI']=str(Path(sys.argv[2]).resolve())
os.environ['FM1_CODEC_CLI']=os.environ['FM1_TOOLING_CLI']
sys.path.insert(0,str(ROOT/'tools'))
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
frozen_lz4=load('frozen_lz4',BASE/'tools/lz4blk.py')
frozen_package=load('frozen_package',BASE/'tools/fm1pkg_make.py')
with patch.dict(sys.modules,{'lz4blk':frozen_lz4,'fm1pkg_make':frozen_package}):
    frozen_build=load('frozen_build',BASE/'tools/build.py')
sys.path.insert(0,str(ROOT/'tools'))
import lz4blk,build,rust_tooling
assert Path(lz4blk.__file__).resolve()==ROOT/'tools/lz4blk.py'
assert Path(build.__file__).resolve()==ROOT/'tools/build.py'
randomizer=random.Random(91);trace=hashlib.sha256();cases=0
for row in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    checksum,name=row.split(maxsplit=1);assert hashlib.sha256((BASE/name.lstrip('*')).read_bytes()).hexdigest()==checksum,name
def record(data):
    global cases
    cases+=1;trace.update(len(data).to_bytes(8,'little'));trace.update(data)
def outcome(function,*args):
    try:return ('bytes',function(*args))
    except (IndexError,ValueError,SystemExit)as error:return (type(error).__name__,str(error))
sizes=list(range(41))+[63,64,255,256,4095,4096,4097,8192,16384,65534,65535,65536,65537,70000,1_100_000]
for n in sizes:
    for kind in range(4):
        source=bytes(n)if kind==0 else bytes((i%7 for i in range(n)))if kind==1 else bytes((i%256 for i in range(n)))if kind==2 else bytes(randomizer.getrandbits(8)for _ in range(n))
        expected=frozen_lz4.compress(source);actual=lz4blk.compress(source);assert actual==expected,(n,kind)
        assert lz4blk.decompress(expected)==source
        assert frozen_lz4.decompress(actual)==source
        record(actual)
for n in [0,1,14,15,16,254,255,256,509,510,511,4096,65536]:
    assert lz4blk._len_bytes(n)==frozen_lz4._len_bytes(n);record(bytes(lz4blk._len_bytes(n)))
for prefix in [b'',b'Z',b'abcdefg',bytes(range(64))]:
    blocks=[b'',b'\x20A',b'\x10A\0\0',b'\0\0\0',b'\xf0',b'\x0f\x01\0',b'\x10A\x01\0']
    blocks.extend(bytes(randomizer.getrandbits(8)for _ in range(randomizer.randrange(33)))for _ in range(128))
    for block in blocks:
        expected=outcome(frozen_lz4.decompress,block,prefix);actual=outcome(lz4blk.decompress,block,prefix)
        assert actual[0]==expected[0],(block,prefix,actual,expected)
        if actual[0]=='bytes':assert actual==expected;record(actual[1])
        else:record(actual[0].encode())
loader_cases=0
for n in [0,1,5,12,15,255,4095,4096,4097,8192,8193,0x14000]:
    for kind in range(3):
        image=bytes(n)if kind==0 else bytes((i%251 for i in range(n)))if kind==1 else bytes(randomizer.getrandbits(8)for _ in range(n))
        expected=outcome(frozen_build.ldr_wrap,image);actual=outcome(build.ldr_wrap,image)
        assert actual==expected,(n,kind,actual[0],expected[0])
        record(actual[1]if actual[0]=='bytes'else actual[1].encode());loader_cases+=1
# The wire adapter bounds output; this replaces unbounded legacy expansion.
oversized=b'\x0f\x01\0'+b'\xff'*4314+b'\0'
assert len(frozen_lz4.decompress(oversized,b'X'))>1_100_000
try:lz4blk.decompress(oversized,b'X');raise AssertionError('expansion must be refused')
except ValueError as error:assert '1100000'in str(error)
with tempfile.TemporaryDirectory()as directory:
    source=Path(directory)/'input.bin';compressed=Path(directory)/'compressed.bin';decoded=Path(directory)/'decoded.bin';wrapped=Path(directory)/'ota.bin'
    image=bytes(range(251))*33;source.write_bytes(image)
    for command,src,dest in [('lz4-compress',source,compressed),('lz4-decompress',compressed,decoded),('loader-wrap',source,wrapped)]:
        subprocess.run([sys.argv[2],'tools',command,str(src),str(dest)],check=True)
    assert compressed.read_bytes()==frozen_lz4.compress(image)
    assert decoded.read_bytes()==image
    assert wrapped.read_bytes()==frozen_build.ldr_wrap(image)
    record(wrapped.read_bytes())
rust_tooling.close()
result=dict(cases=cases,compression_cases=len(sizes)*4,malformed_prefix_cases=4*135,loader_cases=loader_cases,native_cli_commands=3,explicit_expansion_refusals=1,source_hashes=440,sha256=trace.hexdigest())
out=ROOT/'build/build-policy';out.mkdir(parents=True,exist_ok=True);(out/'differential.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
