# SPDX-License-Identifier: GPL-3.0-only
"""Retain the complete host/app implementation; only resolve its include roots."""
from pathlib import Path
import sys
repo=Path(sys.argv[1]).resolve();out=Path(sys.argv[2])
text=(repo/'host/amb_host.c').read_text(encoding='utf-8')
text=text.replace('#include "../firmware/','#include "'+str(repo/'firmware').replace('\\','/')+'/')
with out.open('w',encoding='utf-8',newline='\n') as target:target.write(text)
