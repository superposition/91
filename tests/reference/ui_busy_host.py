# SPDX-License-Identifier: GPL-3.0-only
"""Keep production UI and native backend; interpose only three IO entrypoints."""
from pathlib import Path
import sys
repo=Path(sys.argv[1]).resolve();out=Path(sys.argv[2])
text=(repo/'host/amb_host.c').read_text(encoding='utf-8')
text=text.replace('#include "../firmware/','#include "'+str(repo/'firmware').replace('\\','/')+'/')
for name in ['plat_store_load','plat_store_save','plat_store_restore']:
    original='int '+name+'('
    assert text.count(original)==1,name
    text=text.replace(original,'int base_'+name+'(',1)
out.write_text(text,encoding='utf-8')
