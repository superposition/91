#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Host-only volatile register proxy for exact original HAL sequences.
Source algorithms are unchanged; ISA bodies are removed from unused entrypoints.
"""
import argparse,re
from pathlib import Path

def balanced(s,start):
    depth=0;quote=None;escape=False
    for i in range(start,len(s)):
        c=s[i]
        if quote:
            if escape:escape=False
            elif c=='\\':escape=True
            elif c==quote:quote=None
        elif c in ('"',"'"):quote=c
        elif c=='(':depth+=1
        elif c==')':
            depth-=1
            if depth==0:return i+1
    raise ValueError('unclosed expression')

def transform(s):
    pattern=re.compile(r'\(\s*(?:const\s+)?volatile\s+(uint(?:8|16|32)_t)\s*\*\s*\)')
    at=0
    while True:
        m=pattern.search(s,at)
        if not m:break
        begin=m.end()
        while s[begin].isspace():begin+=1
        end=balanced(s,begin) if s[begin]=='(' else begin+len(re.match(r'[a-zA-Z_0-9]+',s[begin:]).group())
        replacement='proxy<'+m[1]+'>'+ '('+s[begin:end]+')'
        s=s[:m.start()]+replacement+s[end:];at=m.start()+len(replacement)
    while True:
        m=re.search(r'__asm__\s+(?:volatile\s*)?\(',s)
        if not m:break
        end=balanced(s,s.index('(',m.start()))
        while s[end].isspace():end+=1
        assert s[end]==';';s=s[:m.start()]+s[end+1:]
    s=s.replace('#include "fm1_core1_stop.h"','static void fm1_core1_stop(void) {}')
    s=s.replace('extern const char fm1_fatal_stubs[];','#define fm1_fatal_stubs ((const char *)0x02001000u)')
    s=s.replace('static void fm1_fault(const fm1_crash_t *c);','static void fm1_fault(const fm1_crash_t *c) {(void)c;}')
    return s

def main():
    p=argparse.ArgumentParser();p.add_argument('source');p.add_argument('output');a=p.parse_args()
    out=Path(a.output);out.mkdir(parents=True,exist_ok=True)
    for name in ['fm1_cc.h','fm1_time.h','fm1_gpio.h','fm1_irq.h','fm1_timer.h','fm1_audio.h','fm1_adc.h','fm1_lcd_hw.h']:
        source=Path(a.source)/'firmware/hal'/name
        (out/name).write_bytes(transform(source.read_text(encoding='utf-8')).encode())
    source=(Path(a.source)/'firmware/src/app/main_fm1.c').read_text(encoding='utf-8')
    begin=source.index('void fm1_cstart(void)')
    # cstart is the last function in this source; retain its actual conditionals
    # and every memory-copy loop. Host stubs provide only ISA/device/callbacks.
    (out/'cstart.inc').write_bytes(source[begin:].encode())
    source=(Path(a.source)/'firmware/hal/fm1_sys.h').read_text(encoding='utf-8')
    begin=source.index('static int fm1_mem_readable(')
    end=source.index('\n}',begin)+2
    (out/'readable.inc').write_bytes(source[begin:end].encode())
if __name__=='__main__':main()
