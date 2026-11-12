#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Actual pending foreground dispatch plus the real USB SysEx collector."""
from pathlib import Path
import argparse,re,subprocess,sys
p=argparse.ArgumentParser();p.add_argument('source');p.add_argument('output');p.add_argument('--frozen-quirk',action='store_true');a=p.parse_args()
repo=Path(__file__).resolve().parents[2];out=Path(a.output).resolve()
subprocess.run([sys.executable,str(repo/'tests/reference/storage_dispatch_reference.py'),str(out)],check=True)
source=(Path(a.source)/'firmware/src/usb.c').read_text(encoding='utf-8')
def function(signature):
    start=source.index(signature);end=re.search(r'(?m)^}',source[start:]);assert end
    return source[start:start+end.end()]
parser='\n'.join(function(s) for s in ['static void px_end(', 'static void dx_end(', 'static void sysex_byte('])
if '/* Exact raw roots only:' in source:
    start=source.rfind('#if EN_RUST_USB',0,source.index('/* Exact raw roots only:'))
    end=source.index('/* SysEx assembly:',start)
    parser=source[start:end]+'\n'+parser
text=(out/'test.c').read_text(encoding='utf-8')
text=text.replace('#include "dx_proto.h"','#include "dx_proto.h"\n#include "rust_usb.h"\n#define USB_DX 1\n#define DX_MAX 4112u')
text=text.replace('static uint8_t dx_buf[4112],dx_state,dx_kind;static uint32_t dx_pos;', 'static uint8_t dx_buf[4112],dx_state,dx_kind,dx_on,px_on,px_buf[16];static uint32_t dx_pos,px_pos;')
text=text.replace('static struct {uint8_t ota_req,uboot_req;}usb;', 'static struct {uint8_t sysex[16],sx_len,sx_on,uboot_req,ota_req,tel_req;}usb;')
text=text.replace('static uint32_t px_q[32],px_r,px_w;', 'static uint32_t px_q[32],px_r,px_w,midi_in_q[64],mi_w,mi_r;')
needle='static unsigned raw_request('
assert text.count(needle)==1
text=text.replace(needle,parser+'\n'+needle)
needle='deny=1;dx_service();assert(dx_state==2&&ju_ack_n==0);'
assert text.count(needle)==1
case=r"""
    uint8_t pending[8];memcpy(pending,dx_buf,8);
    sysex_byte(0xf0);assert(!dx_on&&dx_state==2&&px_on&&usb.sx_on);
    // EXPECT_LENGTH
    sysex_byte(0xf7);assert(!dx_on&&dx_state==2&&!usb.sx_on&&!memcmp(pending,dx_buf,8));
    dx_service();assert(dx_state==2&&ju_ack_n==0);
"""
case=case.replace('// EXPECT_LENGTH', 'assert(dx_pos==0);dx_pos=8; /* observed frozen quirk; repair only the test continuation */' if a.frozen_quirk else 'assert(dx_pos==8);')
text=text.replace(needle,needle+case)
(out/'test.c').write_bytes(text.encode('utf-8'))
