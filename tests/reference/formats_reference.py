#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Export original Python package primitives from a verified R01 extraction."""
import importlib.util
import json
import sys
from pathlib import Path

spec=importlib.util.spec_from_file_location('r01pkg',Path(sys.argv[1])/'tools/fm1pkg_make.py')
pkg=importlib.util.module_from_spec(spec)
spec.loader.exec_module(pkg)
def raw(n,seed=91):return bytes((seed+i*37)&255 for i in range(n))
vectors=[]
for seed in range(256):
    b=raw(seed*2,seed);encrypted=bytearray(b);pkg.enc(encrypted,0,len(b),seed*257)
    sf=bytearray(b);pkg.sfc(sf,0,len(sf),-seed,0x980f)
    keydata=raw(32,seed)
    vectors.append({'crc':pkg.crc16(b,seed*257),'enc':list(encrypted),'sfc':list(sf),'key':pkg.chipkey_decode(keydata),'marker':list(pkg.marker(b))})
keyhash=2166136261
for key in range(65536):
    for byte in pkg.key_blob(key):keyhash=((keyhash^byte)*16777619)&0xffffffff
package=pkg.ufw(raw(1200),raw(37,13),'FM-1_7000300')
entries=[list(pkg.entry(seed*2053,0x12345678+seed*4096,0x100+seed,seed,255-seed,seed*1993,raw(16,seed))) for seed in range(32)]
compensation=[]
for seed in range(64):
    app=bytearray(raw(8+seed*3));offset=seed*17%(len(app)-3);app[offset:offset+4]=bytes(4);want=0x4352^(seed*257)
    compensation.append({'app':list(app),'offset':offset,'want':want,'bytes':list(pkg.compensate(app,offset,want))})
print(json.dumps({'vectors':vectors,'key_blob_hash':keyhash,'package':list(package),'flash':list(raw(1200)),'ota':list(raw(37,13)),'entries':entries,'compensation':compensation},separators=(',',':')))
