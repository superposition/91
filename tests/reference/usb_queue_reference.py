#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Extract actual producer/consumer and exact saved-IRQ wrapper, no driver clone."""
from pathlib import Path
import argparse,re
p=argparse.ArgumentParser();p.add_argument('source');p.add_argument('output');a=p.parse_args()
root=Path(a.source);out=Path(a.output);out.mkdir(parents=True,exist_ok=True)
usb=(root/'firmware/src/usb.c').read_text(encoding='utf-8');plat=(root/'firmware/src/app/plat_fm1.c').read_text(encoding='utf-8')
def fn(text,signature):
 start=text.index(signature);end=re.search(r'(?m)^}',text[start:]);assert end;return text[start:start+end.end()]
parts=[fn(usb,'static void midi_out_event('),fn(plat,'int plat_midi_in('),fn(plat,'void plat_midi_out(')]
(out/'queue_source.inc').write_bytes(('\n'.join(parts)+'\n').encode())
