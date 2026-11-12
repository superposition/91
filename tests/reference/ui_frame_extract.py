# SPDX-License-Identifier: GPL-3.0-only
"""Extract complete source functions without rewriting their decisions."""
from pathlib import Path
import re, sys

text=(Path(sys.argv[1])/'firmware/src/app/ui.c').read_text(encoding='utf-8')
mode=sys.argv[3]
name={'frozen':'void ui_frame(', 'port':'static uint32_t ui_frame_port(', 'accel':'static int32_t accel(',
      'button_frozen':'static void button(', 'button_current':'static void button(', 'button_port':'static uint32_t ui_button_port('}[mode]
match=re.search(re.escape(name)+r'[^;{]*\{',text)
if not match:raise ValueError('missing '+name)
start=match.start();end=match.end();depth=1
while depth:
    if text[end]=='{':depth+=1
    elif text[end]=='}':depth-=1
    end+=1
body=text[start:end]
if mode in ['frozen','button_frozen','button_current']:
    views=re.search(r'enum \{ V_TEMPO.*?\};',text,re.S).group(0)
    if mode=='frozen':body=views+'\n'+body.replace('void ui_frame(', 'static void reference_frame(',1)
    else:
        group=re.search(r'static const struct\s*\{\s*uint8_t btn, first, n;\s*\}\s*GROUP\[\]\s*=\s*\{.*?\};',text,re.S).group(0)
        helpers=[]
        for function in ['static int group_of(', 'static int oct_par(']:
            match=re.search(re.escape(function)+r'[^;{]*\{',text);end=match.end();depth=1
            while depth:
                if text[end]=='{':depth+=1
                elif text[end]=='}':depth-=1
                end+=1
            helpers.append(text[match.start():end])
        port='#if EN_RUST_APP\n#include "ui_button_port.inc"\n#endif\n' if mode=='button_current' else ''
        body=views+'\n'+group+'\n#define NGROUP (sizeof GROUP/sizeof GROUP[0])\n'+'\n'.join(helpers)+'\n'+port+body
with Path(sys.argv[2]).open('w',encoding='utf-8',newline='\n') as output:
    output.write('/* SPDX-License-Identifier: GPL-3.0-only */\n'+body+'\n')
