#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Test the actual detach guard against every pending-source combination."""
from pathlib import Path
import os,subprocess

repo=Path(__file__).resolve().parents[2]
text=(repo/'firmware/src/app/ui.c').read_text(encoding='utf-8')
start=text.index('int ui_flush_pending(void)')
brace=text.index('{',start);end=brace+1;depth=1
while depth:
    if text[end]=='{':depth+=1
    elif text[end]=='}':depth-=1
    end+=1
source='''#include <stdint.h>
static struct {uint8_t store_wait,vox_dirty,set_dirty,set_lazy;}ui;
static uint8_t ui_io_pending,rs_state;
static int8_t ui_genre_pending;
static int pending_project,pending_globals;
static int project_pending(void){return pending_project;}
static int project_pending_globals(void){return pending_globals;}
'''+text[start:end]+'''
int main(void){unsigned mask,state;
 for(state=0;state<4;state++)for(mask=0;mask<256;mask++){
  ui_io_pending=(mask&1)?7:0;ui_genre_pending=(mask&2)?17:-1;
  ui.store_wait=(mask>>2)&1;ui.vox_dirty=(mask>>3)&1;
  ui.set_dirty=(mask>>4)&1;ui.set_lazy=(mask>>5)&1;
  pending_project=(mask>>6)&1;pending_globals=(mask>>7)&1;rs_state=(uint8_t)state;
  if(!!ui_flush_pending()!=(mask!=0||state==1))return 1;
 }
 return 0;
}
'''
out=repo/'build/ui-flush-api';out.mkdir(parents=True,exist_ok=True)
(out/'test.c').write_text(source)
subprocess.run([os.environ.get('CC','cc'),'-std=c99','-O2',str(out/'test.c'),'-o',str(out/'test')],check=True)
subprocess.run([str(out/'test')],check=True)
print('1024 actual UI detach-guard cases passed; clean/completed/error restore states remain detachable')
