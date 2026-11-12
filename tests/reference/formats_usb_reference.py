#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Extract the dependency-free original USB message classifier, unchanged."""
from pathlib import Path
import sys
text=(Path(sys.argv[1])/'firmware/src/usb.c').read_text()
start=text.index('static void dx_end(void)')
brace=text.index('{',start);depth=1;end=brace+1
while depth:
    depth+=(text[end]=='{')-(text[end]=='}');end+=1
prefix='''/* Exact R01 dx_end; single-thread oracle replaces only publication barrier. */
#ifndef RING_PUBLISH
#define RING_PUBLISH() ((void)0)
#endif
static uint8_t dx_buf[4112],dx_on;
static volatile uint8_t dx_state,dx_kind;
static uint32_t dx_pos;
'''
Path(sys.argv[2]).write_text(prefix+text[start:end]+'\n')
