#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Read and write the FM-1's registers over USB-MIDI, through a development build of X0X
(X0X_DEBUG=1 ./build.sh; the commands are in firmware/src/app/editor.c and refuse anything outside
a few register and RAM ranges). Needs mido with python-rtmidi.

  fm1_debug.py peek ADDR [N]          N words (<= 16) from ADDR
  fm1_debug.py poke ADDR VALUE        write a word, print it before and after
  fm1_debug.py clock [ITERATIONS]     time a fixed loop against the 24 MHz timer (IRQs off)
"""
import sys
import rust_codecs as _codecs
import rust_tooling as _tools
import contextlib
import time

import mido

HDR = [0x7D, 0x46, 0x4C]
PEEK, POKE, CLOCK = 40, 41, 42


def w35(v):
    return list(_codecs.call(101, [v & 0xffffffff, (v >> 32) & 7]))


def r35(b, i):
    return _codecs.scalar(102, [], [b[i:i + 5]])


def _tool(method, args=(), adapter=None):
    try:
        return _tools.call('debug.' + method, args, adapter)
    except _tools.ToolError as error:
        kinds = {'SystemExit': SystemExit, 'ValueError': ValueError,
                 'ZeroDivisionError': ZeroDivisionError}
        raise kinds.get(error.kind, RuntimeError)(str(error)) from error


class _Adapter:
    def __call__(self, action):
        kind, a = action['kind'], action['args']
        if kind == 'debug_outputs':
            return list(mido.get_output_names())
        if kind == 'debug_open':
            self.stack = contextlib.ExitStack()
            try:
                self.inp = self.stack.enter_context(mido.open_input(a[0]))
                self.out = self.stack.enter_context(mido.open_output(a[0]))
            except BaseException:
                self.stack.close()
                raise
            return None
        if kind == 'debug_poll':
            message = self.inp.poll()
            return None if message is None else dict(type=message.type,
                data=list(message.data) if message.type == 'sysex' else [])
        if kind == 'debug_send':
            self.out.send(mido.Message('sysex', data=a[0]))
            return None
        if kind == 'debug_clock':
            return time.time()
        if kind == 'debug_sleep':
            time.sleep(a[0])
            return None
        if kind == 'debug_close':
            self.stack.close()
            return None
        raise RuntimeError('unknown debugger I/O action: ' + kind)


def ask(cmd, args, timeout=3.0):
    return _tool('ask', [cmd, list(args), timeout], _Adapter())


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    if a[0] == 'peek':
        x, y = int(a[1], 0), int(a[2], 0) if len(a) > 2 else 1
    elif a[0] == 'poke':
        x, y = int(a[1], 0), int(a[2], 0)
    elif a[0] == 'clock':
        x, y = int(a[1], 0) if len(a) > 1 else 2000000, 0
    else:
        sys.exit(__doc__)
    plan = _tool('plan', [a[0], x, y])
    reply = ask(plan['cmd'], plan['args'], timeout=plan['timeout'])
    rendered = _tool('report', [plan, reply])
    print(rendered['text'], end='')
    error = rendered['error']
    if error:
        if error['kind'] == 'CodecError':
            raise _codecs.CodecError(int(error['code']))
        kind = {'ZeroDivisionError': ZeroDivisionError, 'ValueError': ValueError}.get(error['kind'], RuntimeError)
        raise kind(error['message'])



if __name__ == "__main__":
    main()
