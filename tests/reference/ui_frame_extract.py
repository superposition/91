# SPDX-License-Identifier: GPL-3.0-only
"""Extract complete source functions without rewriting their decisions."""
from pathlib import Path
import re, sys

text=(Path(sys.argv[1])/'firmware/src/app/ui.c').read_text(encoding='utf-8')
mode=sys.argv[3]
name={'frozen':'void ui_frame(', 'port':'static uint32_t ui_frame_port(', 'accel':'static int32_t accel('}[mode]
match=re.search(re.escape(name)+r'[^;{]*\{',text)
if not match:raise ValueError('missing '+name)
start=match.start();end=match.end();depth=1
while depth:
    if text[end]=='{':depth+=1
    elif text[end]=='}':depth-=1
    end+=1
body=text[start:end]
if mode=='frozen':
    views=re.search(r'enum \{ V_TEMPO.*?\};',text,re.S).group(0)
    body=views+'\n'+body.replace('void ui_frame(', 'static void reference_frame(',1)
with Path(sys.argv[2]).open('w',encoding='utf-8',newline='\n') as output:
    output.write('/* SPDX-License-Identifier: GPL-3.0-only */\n'+body+'\n')
