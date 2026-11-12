# SPDX-License-Identifier: GPL-3.0-only
"""Extract the exact existing alpha pixel function; no rewritten C oracle."""
import sys
from pathlib import Path
source=Path(sys.argv[1])/'firmware/src/app/ui.c'
text=source.read_text(encoding='utf-8')
start=text.index('static void px_blend(')
brace=text.index('{',start)
level=1
end=brace+1
while level:
    if text[end]=='{':level+=1
    elif text[end]=='}':level-=1
    end+=1
Path(sys.argv[2]).write_text(text[start:end]+'\n',encoding='utf-8')
