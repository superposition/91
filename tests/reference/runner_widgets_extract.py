# SPDX-License-Identifier: GPL-3.0-only
"""Copy exact existing helpers used by runner.c; no rewritten C renderer."""
from pathlib import Path
import sys,re
text=(Path(sys.argv[1])/'firmware/src/app/ui.c').read_text(encoding='utf-8')
functions=[]
for name in ['static void itoa_u(', 'static void cv_round(', 'static void text_c(',
             'static void px_blend(', 'static uint16_t mix(', 'static int q16(']:
    # Skip forward declarations; only a complete definition can be an oracle.
    match=re.search(re.escape(name)+r'[^;{]*\{',text)
    if not match:raise ValueError('missing function definition: '+name)
    start=match.start();brace=match.end()-1;end=brace+1;depth=1
    while depth:
        if text[end]=='{':depth+=1
        elif text[end]=='}':depth-=1
        end+=1
    functions.append(text[start:end])
Path(sys.argv[2]).write_text('/* SPDX-License-Identifier: GPL-3.0-only */\n'+
    '/* Exact UI widgets used by the independent game renderer. */\n'+'\n'.join(functions)+'\n',encoding='utf-8')
