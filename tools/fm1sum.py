#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Summarise a telemetry CSV (tools/fm1log.py): by genre and piece, the load, the xruns, and where the first core's time goes.
    python tools/fm1sum.py telemetry/fm1_....csv"""
import csv
import sys
import rust_tooling as _tools

_NAMES = _tools.call('telemetry.fields')
GENRES, STAGES = _NAMES['genres'], _NAMES['stages']
FIELDS = [k if k in ('w_pad', 'w_rev') else 'p_' + k for k in STAGES]


def _report(section, path):
    with open(path) as stream:
        rows = list(csv.DictReader(stream))
    try:
        result = _tools.call('telemetry.summary', [section, rows])
    except _tools.ToolError as error:
        if error.kind == 'SystemExit':
            raise SystemExit(str(error)) from error
        raise ValueError(str(error)) from error
    print(result, end='')


def main():
    _report('main', sys.argv[1])


def peaks(path):
    _report('peaks', path)


def halves(path):
    _report('halves', path)


def fixed(path):
    _report('fixed', path)


def diag(path):
    _report('diag', path)


peaks(sys.argv[1])
halves(sys.argv[1])
fixed(sys.argv[1])
diag(sys.argv[1])
main()
