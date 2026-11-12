#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Export immutable Python recovery/logger/debug byte functions; no I/O adapter."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys

ROOT=Path(__file__).resolve().parents[2]
baseline=Path(sys.argv[1])
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1)
    assert hashlib.sha256((baseline/name.strip()).read_bytes()).hexdigest()==digest
def load(file):
    spec=importlib.util.spec_from_file_location(file.stem,baseline/file);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
rescue=load(Path('tools/fm1_rescue.py'));logger=load(Path('tools/fm1log.py'))
tree=ast.parse((baseline/'tools/fm1_debug.py').read_text());functions=[node for node in tree.body if isinstance(node,ast.FunctionDef)and node.name in ['w35','r35']];debug={};exec(compile(ast.Module(body=functions,type_ignores=[]),'frozen-debug-codecs','exec'),debug)
vectors=[]
def add(name,op,args,blobs,expected):vectors.append(dict(name=name,op=op,args=args,blobs=[list(b)for b in blobs],expected=list(expected)))
for seed in range(256):
    data=bytes((seed+i*37)&255 for i in range(seed*2));cipher=bytearray(data);rescue.crc_cipher(cipher);add('frozen-rescue-cipher-'+str(seed),100,[],[data],cipher)
for value in [0,1,127,128,255,0x12345678,0xffffffff,0x7ffffffff,-1,-2147483648]:
    encoded=debug['w35'](value);add('frozen-debug-word-'+str(value),101,[value&0xffffffff,(value>>32)&7],[],encoded);add('frozen-debug-read-'+str(value),102,[],[encoded],struct.pack('<I',debug['r35'](encoded,0)))
for count in [0,1,16,20,35,36,40]:
    raw=struct.pack('<%dI'%count,*[(i*0x12345678+count)&0xffffffff for i in range(count)])
    for tail in [b'',b'\x91',b'\x7f\xff\x80']:
        packed=logger.pack7(raw+tail)
        for maximum in [16,20,35,40]:
            decoded=bytes(logger.unpack7(packed));expected=decoded[:min(len(decoded)//4,maximum)*4];add('frozen-logger-'+str((count,len(tail),maximum)),103,[70,maximum],[[125,80,70,*packed]],expected)
py=json.loads((ROOT/'rust/fm1-formats/tests/formats/python-reference.json').read_text());add('frozen-rescue-flash-entry',105,[],[py['package']],rescue.package_flash_image(bytes(py['package'])))
for command in range(3):add('frozen-logger-request-'+str(command),106,[command],[],bytes([240,*logger.PREFIX,command,247]))
for field,value in [(0,0),(127,16383),(17,8192)]:add('jam-control-'+str(field),104,[2,field,value],[],bytes([field,value>>7,value&127]))
for length,crc in [(0,0),(3840,0x12345678),(16383,0xffffffff)]:add('jam-begin-'+str(length),104,[0,length,crc],[],bytes([length>>7,length&127,*[(crc>>i)&15 for i in range(28,-1,-4)]]))
for length in [0,1,127,128,255,256]:
    data=bytes((i*37+length)&255 for i in range(length));add('jam-chunk-'+str(length),104,[1,15],[data],bytes([15,length>>7,length&127,*[v for b in data for v in [b>>4,b&15]]]))
(ROOT/'rust/fm1-formats/tests/formats/tool-reference.json').write_text(json.dumps(vectors,separators=(',',':'))+'\n')
print(len(vectors),'frozen tool codec vectors')
