# SPDX-License-Identifier: GPL-3.0-only
"""Retain the actual read-only wrappers, without changing their decisions."""
from pathlib import Path
import re,sys
text=Path('firmware/src/dsp/rust_harmony.inc').read_text(encoding='utf-8')
functions=[]
for name in ['harmony_preview','harmony_capture']:
    match=re.search(r'static (?:int|void) '+name+r'\([^;{]*\)\s*\{',text)
    assert match,name
    end=match.end();depth=1
    while depth:
        if text[end]=='{':depth+=1
        if text[end]=='}':depth-=1
        end+=1
    functions.append(text[match.start():end])
with Path(sys.argv[1]).open('w',encoding='utf-8',newline='\n')as target:
    target.write('/* SPDX-License-Identifier: GPL-3.0-only */\n'+'\n'.join(functions)+'\n')
