#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""PERF-1 telemetry logger: reads the status frames the firmware sends over USB-MIDI (SysEx) and writes a CSV.

    python tools/fm1log.py --list                 the MIDI ports
    python tools/fm1log.py                        log to telemetry/fm1_<time>.csv until Ctrl-C (and a live line)
    python tools/fm1log.py --port FM-1 --out x.csv
    python tools/fm1log.py --bench                time float / int / table / flash-versus-RAM loops on the chip (once; a glitch in the music)
    python tools/fm1log.py --selftest             the decoder, without a device

The device sends nothing until this program asks (F0 7D 50 46 01 F7, repeated every 3 s as a keep-alive);
ten seconds without one and it stops. Needs: pip install mido python-rtmidi
"""
import argparse
import rust_codecs as _codecs
import rust_tooling as _tools
import csv
import os
import struct
import sys
import time

PREFIX = [0x7D, 0x50, 0x46]
def _tool(method, args=()):
    try:
        return _tools.call(method, args)
    except _tools.ToolError as error:
        kind = {'ValueError': ValueError, 'ZeroDivisionError': ZeroDivisionError}.get(error.kind, RuntimeError)
        raise kind(str(error)) from error


_NAMES = _tool('telemetry.fields')
_TIMING = _tool('telemetry.timing')
FIELDS, EXTRA, DIAG, BENCH_NAMES = (_NAMES[k] for k in ('fields', 'extra', 'diag', 'bench_names'))
DIAG_PIECES = [k[2:] for k in DIAG if k.startswith('d_')]


def pack7(data):
    return list(_codecs.call(72, [], [data]))

def unpack7(data):
    return list(_codecs.call(73, [1_100_000], [data])[4:])

def decode(msg):
    return _tool('telemetry.decode', [0x46, list(msg)])


def decode_diag(msg):
    return _tool('telemetry.decode', [0x5B, list(msg)])


def decode_extra(msg):
    return _tool('telemetry.decode', [0x5A, list(msg)])


def decode_bench(msg):
    value = _tool('telemetry.decode', [0x47, list(msg)])
    return tuple(value) if value is not None else None


def show_bench(w):
    result = _tool('telemetry.bench', [list(w)])
    print(result['text'], end='')
    if result['error']:
        raise ZeroDivisionError(result['error'])


def selftest():
    w = list(range(1, len(FIELDS) + 1))
    msg = PREFIX + pack7(struct.pack("<%dI" % len(w), *w))
    d = decode(msg)
    assert d and d.pop("chorus") == 0 and d.pop("bass") == 0 and [d[k] for k in FIELDS] == w, d
    assert decode([0x00, 0x59, 0x11]) is None
    w = list(range(1, 36))                        # a firmware before 0.16.5: no stack words, they read 0
    d = decode(PREFIX + pack7(struct.pack("<35I", *w)))
    assert d and d["batt"] == 35 and d["stk_main"] == 0 and d["stk_c1s"] == 0, d
    wb =[1500, 240] + [3000 + 100 * k for k in range(9)]
    wb = wb + [7] * 5
    assert decode_bench(PREFIX[:2] + [0x47] + pack7(struct.pack("<16I", *wb))) == tuple(wb)
    we = list(range(1, 21))                       # an older firmware's 20 words: the rest read 0
    ex = decode_extra(PREFIX[:2] + [0x5A] + pack7(struct.pack("<20I", *we)))
    assert ex and [ex[k] for k in EXTRA] == we + [0] * (len(EXTRA) - 20), ex
    we = list(range(1, len(EXTRA) + 1))
    ex = decode_extra(PREFIX[:2] + [0x5A] + pack7(struct.pack("<%dI" % len(we), *we)))
    assert ex and [ex[k] for k in EXTRA] == we, ex
    wd = list(range(1, len(DIAG) + 1))
    dg = decode_diag(PREFIX[:2] + [0x5B] + pack7(struct.pack("<%dI" % len(wd), *wd)))
    assert dg and [dg[k] for k in DIAG] == wd, dg
    print("selftest ok:", len(msg) + 2, "bytes a frame")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--port", default="")
    ap.add_argument("--out", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--bench", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    import mido
    if a.list:
        print("in :", mido.get_input_names())
        print("out:", mido.get_output_names())
        return
    ins, outs = mido.get_input_names(), mido.get_output_names()
    key = a.port.lower()
    pick = lambda names: _tool('telemetry.pick', [list(names), key])
    pin, pout = pick(ins), pick(outs)
    if not pin or not pout:
        sys.exit("no FM-1 MIDI port found (ports: in %s, out %s). Use --port NAME." % (ins, outs))
    _codecs.executable()  # finish dependency startup before arming the logger
    out = a.out or os.path.join("telemetry", time.strftime("fm1_%Y%m%d_%H%M%S.csv"))
    print("in:", pin, "| out:", pout, "| csv:", out)
    if a.bench:
        fin, fout = mido.open_input(pin), mido.open_output(pout)
        fout.send(mido.Message("sysex", data=_codecs.call(106, [2])[1:-1]))
        t_end = time.time() + _TIMING['bench_seconds']
        while time.time() < t_end:
            for m in fin.iter_pending():
                if m.type == "sysex":
                    w = decode_bench(m.data)
                    if w:
                        return show_bench(w)
            time.sleep(_TIMING['bench_poll_seconds'])
        sys.exit("no answer (is it the build with the bench?)")
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    fin, fout = mido.open_input(pin), mido.open_output(pout)
    arm = mido.Message("sysex", data=_codecs.call(106, [1])[1:-1])
    last_arm = 0.0
    prev = None
    extra = {}
    t0 = time.time()
    with open(out, "w", newline="") as f:
        cw = csv.writer(f)
        cw.writerow(["t"] + FIELDS + ["chorus", "bass", "flash", "susp_bad", "boot"] + EXTRA + DIAG + ["event"])
        diag = {}
        try:
            while True:
                now = time.time()
                if _tool('telemetry.arm', [now, last_arm]):
                    fout.send(arm)
                    last_arm = now
                for m in fin.iter_pending():
                    if m.type != "sysex":
                        continue
                    d = decode(m.data)
                    if not d:
                        ex = decode_extra(m.data)
                        if ex:
                            extra = ex
                        else:
                            dg = decode_diag(m.data)
                            if dg:
                                diag = dg
                        continue
                    rendered = _tool('telemetry.row', [now, t0, d, prev, extra, diag])
                    prev = d
                    cw.writerow(rendered['row'])
                    extra, diag = {}, {}
                    f.flush()
                    print(rendered['live'], end='', flush=True)
                time.sleep(_TIMING['poll_seconds'])
        except KeyboardInterrupt:
            pass
        finally:
            try:
                fout.send(mido.Message("sysex", data=_codecs.call(106, [0])[1:-1]))
            except Exception:
                pass
    print("\nwritten:", out)


if __name__ == "__main__":
    main()
