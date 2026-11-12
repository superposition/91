#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Actual Python→host Rust consumer vs independent frozen Python and SDK inputs."""
import argparse
import binascii
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import fm1pkg_make as current
import rust_codecs

def sha(data):return hashlib.sha256(data).hexdigest()
def raw(n,seed=91):return bytes((seed+i*37)&255 for i in range(n))

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline',type=Path)
    parser.add_argument('--sdk',type=Path,default=Path(os.environ.get('AC79_SDK',Path.home()/'fw-AC79_AIoT_SDK')))
    parser.add_argument('--app',type=Path)
    parser.add_argument('--loader',type=Path)
    args=parser.parse_args()
    py=json.loads((ROOT/'rust/fm1-formats/tests/formats/python-reference.json').read_text())
    checks=0
    for seed,row in enumerate(py['vectors']):
        data=raw(seed*2,seed)
        assert current.crc16(data,seed*257)==row['crc']
        enc=bytearray(data);current.enc(enc,0,len(enc),seed*257);assert list(enc)==row['enc']
        sfc=bytearray(data);current.sfc(sfc,0,len(sfc),-seed,0x980f);assert list(sfc)==row['sfc']
        assert current.chipkey_decode(raw(32,seed))==row['key']
        assert list(current.marker(data))==row['marker'];checks+=5
    for seed,expected in enumerate(py['entries']):
        assert list(current.entry(seed*2053,0x12345678+seed*4096,0x100+seed,seed,255-seed,seed*1993,raw(16,seed)))==expected;checks+=1
    for row in py['compensation']:
        assert list(current.compensate(bytes(row['app']),row['offset'],row['want']))==row['bytes'];checks+=1
    expected=bytes(py['package']);assert current.ufw(bytes(py['flash']),bytes(py['ota']),'FM-1_7000300')==expected;checks+=1
    assert rust_codecs.call(1,[],[b'123456789'])==struct.pack('<I',0xcbf43926)
    def concurrent(seed):
        data=raw(seed,seed)
        return rust_codecs.scalar(70,[seed*257],[data])==binascii.crc_hqx(data,seed*257)
    with ThreadPoolExecutor(max_workers=8) as workers:
        assert all(workers.map(concurrent,range(256))),'interleaved host responses'
    for op,blobs in [(20,[bytes(127)]),(21,[bytes(154)]),(13,[b'bad']), (99,[b'',b'',b'x'*21])]:
        try:rust_codecs.call(op,[],blobs)
        except rust_codecs.CodecError:pass
        else:raise AssertionError('bad input accepted')
    # The framed child rejects an oversized message before allocating its body.
    binary=rust_codecs.executable()
    result=subprocess.run([str(binary),'codec-stream'],input=struct.pack('<II',1,1_100_001),capture_output=True,timeout=10)
    assert result.returncode!=0 and not result.stdout
    result=subprocess.run([str(binary),'codec','1'],input=bytes(255),capture_output=True,timeout=10)
    assert result.returncode==0 and result.stdout==struct.pack('<i',-1)
    # A failed child is reported, closed, and a subsequent explicit call retries.
    rust_codecs._process.kill();rust_codecs._process.wait(timeout=2)
    try:rust_codecs.call(1,[],[b'123456789'])
    except (OSError,RuntimeError):pass
    else:raise AssertionError('dead Rust child was silently retried')
    assert rust_codecs._process is None
    assert rust_codecs.scalar(1,blobs=[b'123456789'])==0xcbf43926
    # No fallback can hide a missing executable or malformed/truncated response.
    rust_codecs.close()
    rust_codecs.executable.cache_clear()
    with patch.dict(os.environ,{'FM1_CODEC_CLI':str(ROOT/'build/definitely-absent-codec')}):
        try:rust_codecs.call(1,[],[b'123'])
        except RuntimeError:pass
        else:raise AssertionError('missing Rust executable was silently ignored')
    rust_codecs.executable.cache_clear()
    result={'primitive_consumer_checks':checks,'frozen_package_sha256':sha(expected),'rust_execution_required':True,'host_transport':{'concurrent_calls':256,'oversized_truncated_messages':'pass','explicit_failure_retry':'pass','required_executable':'pass'},'full_sdk_package':'not requested'}
    if args.baseline:
        if not args.app or not args.loader:parser.error('--baseline requires --app and --loader for full SDK proof')
        for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
            digest,name=line.split(maxsplit=1)
            assert sha((args.baseline/name.strip()).read_bytes())==digest,'immutable R01 source drift: '+name
        spec=importlib.util.spec_from_file_location('frozen_package',args.baseline/'tools/fm1pkg_make.py');frozen=importlib.util.module_from_spec(spec);spec.loader.exec_module(frozen)
        frozen.SDK=current.SDK=args.sdk
        app=args.app.read_bytes();loader=args.loader.read_bytes()
        assert frozen.has_stub(app),'full-package proof requires the actual recovery stub'
        packages=[]
        for writer in [frozen,current]:
            flash=writer.flash_image(app,writer.KEY);package=writer.ufw(flash,loader,'FM-1_7000300');packages.append((flash,package))
        assert packages[0]==packages[1],'full SDK/recovery package drift'
        result['full_sdk_package']={'bytes':len(packages[1][1]),'flash_sha256':sha(packages[1][0]),'package_sha256':sha(packages[1][1]),'app_sha256':sha(app),'loader_sha256':sha(loader),'stub_version':frozen.stub_version(app),'sdk':{rel:sha(frozen.sdk_file(rel))for rel in ['uboot.boot','cfg_tool.bin','cfg/eq_cfg_hw.bin']}}
    print(json.dumps(result,sort_keys=True))

if __name__=='__main__':main()
