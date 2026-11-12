#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Use the real USB driver with only its physical HAL include replaced."""
import argparse,re
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('source');p.add_argument('output');p.add_argument('--preserve-ready-length',action='store_true');a=p.parse_args()
root=Path(a.source);out=Path(a.output);out.mkdir(parents=True,exist_ok=True)
source=(root/'firmware/src/usb.c').read_text(encoding='utf-8')
if a.preserve_ready_length:
    old_start='        dx_on = !dx_state;\n        dx_pos = 0;'
    assert source.count(old_start)==1
    source=source.replace(old_start,'        dx_on = !dx_state;\n        if (dx_on) dx_pos = 0;')
old='#include "../hal/fm1_usb.h"'
assert source.count(old)==1
(out/'usb_source.inc').write_bytes(source.replace(old,'#include "usb_mock.h"').encode('utf-8'))
header=(root/'firmware/hal/fm1_usb.h').read_text(encoding='utf-8')
header=re.sub(r'/\*.*?\*/','',header,flags=re.S)
signatures=re.findall(r'FM1_INLINE\s+([^{}]+?)\s*\{',header)
assert len(signatures)>=20
(out/'usb_mock.h').write_bytes(('#pragma once\n#include <stdint.h>\n'+'\n'.join('static '+s.strip()+';' for s in signatures)+'\n').encode('utf-8'))
