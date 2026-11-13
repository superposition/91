# SPDX-License-Identifier: GPL-3.0-only
"""Instrument only publication reads in the actual acid-packet C adapter."""
from pathlib import Path
import re,sys
text=Path('firmware/src/app/ui.c').read_text(encoding='utf-8')
match=re.search(r'static int32_t ui_values_read\([^;{]*\)\s*\{',text)
assert match
end=match.end();depth=1
while depth:
 if text[end]=='{':depth+=1
 if text[end]=='}':depth-=1
 end+=1
body=text[match.start():end]
for old,new in [('en_acid_off[s]','sample_byte(s)'),('en_acid_oct[s]','sample_byte(16+s)'),
 ('en_acid_gate','sample_word(32)'),('en_acid_acc','sample_word(34)'),('en_acid_slide','sample_word(36)')]:
 assert body.count(old)==(1 if '[' in old else 2),(old,body.count(old))
 body=body.replace(old,new)
with Path(sys.argv[1]).open('w',encoding='utf-8',newline='\n')as target:target.write(body+'\n')
