#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
"""Install a Felucca package (.fwsc) on an FM-1 over USB-MIDI.

The same update as the web installer (web/fm1ota.js): step 1, the
running firmware reads parts of the package and starts the update loader;
step 2, the loader reads the whole package and writes it. In both steps the
device asks (SysEx read requests on the logical image) and we answer. Then
the FM-1 restarts and the installed identity is checked.

  fm1_install.py PACKAGE.fwsc [--port NAME] [--yes] [--force]
  fm1_install.py --info [--port NAME]

If the FM-1 is still in update mode (an earlier install was cut off), the
install finishes the write. Needs mido with python-rtmidi.

Exit codes: 0 done, 1 cancelled or other error, 2 bad arguments or package,
3 FM-1 not found, 4 connection lost or the device stopped, 5 timeout (no
loader / no restart), 6 wrong model, or another identity after the install.
"""
import argparse
import struct
import rust_codecs as _codecs
import rust_tooling as _tools
import queue
import re
import sys
import time

HS_QUERY = bytes([0xF0, 0x00, 0x32, 0x45, 0x00, 0x00, 0x00, 0x40, 0x7F, 0xF7])
# PERF-1 from 0.16.13 answers HS_QUERY with its model and family only ("FM-1_700": M-UPGRADE 3.11 refuses more than three
# digits) and the whole identity to HS_FULL, the handshake with the one-byte body 01 (built below, after pack7)
HS_FULL = None
UPGRADE = bytes([0xF0, 0x22, 0x24, 0x35, 0x7F, 0xF7])
FINISH_CHECK, FINISH_WRITE = 0xE0000000, 0xF0000000
MAXDATA = 512
BLOCKS, BLK, KEEP = 20, 0x30, 0x2F
LOADER_MARK = b"FELUCCA-LOADER-1"
PORT_RE = re.compile(r"fm-1|felucca|ota|composite|sinco|usb-midi", re.I)   # never probe other gear

# seconds; the tests shorten them
DELAY = {"open": 0.3, "start": 2.0, "reply": 0.01, "loader": 3.0, "reboot": 3.0, "retry": 1.0,
         "poll": 1.0, "hs": 1.0, "idle_check": 8.0, "idle_write": 180.0,
         "wait_loader": 30.0, "wait_reboot": 40.0}

EXIT = {"usage": 2, "badpkg": 2, "notfound": 3, "model": 6, "lost": 4, "stopped": 4, "badreq": 4,
        "noloader": 5, "noreturn": 5, "mismatch": 6}


class InstallError(Exception):
    def __init__(self, code, msg):
        super().__init__(msg)
        self.code = code


# ------------------------------------------------------------ wire format ---

def pack7(data):
    return _codecs.call(72, [], [data])

def unpack7(data):
    return _codecs.call(73, [1_100_000], [data])[4:]

def response(addr, data, fl=0):
    return _codecs.call(93, [addr, fl], [data])

def parse_request(pkt):
    try:
        return struct.unpack('<III', _codecs.call(94, [0], [pkt]))
    except _codecs.CodecError:
        return None

class Identity:
    def __init__(self, text, model, version):
        self.text, self.model, self.version = text, model, version

    @property
    def loader(self):
        return self.model.lower().startswith("ota-")


def _tool(method, args=(), adapter=None, **values):
    global HS_FULL
    if adapter is not None and HS_FULL is None:
        HS_FULL = _codecs.call(95, [1])
    try:
        return _tools.call('install.' + method, args, adapter, **values)
    except _tools.ToolError as error:
        if error.kind == 'InstallError':
            raise InstallError(error.code, str(error)) from error
        kind = {'ValueError': ValueError, 'OSError': OSError, 'ZeroDivisionError': ZeroDivisionError}.get(error.kind, RuntimeError)
        raise kind(str(error)) from error


def _identity(value):
    return Identity(value['text'], value['model'], int(value['version'])) if value else None


def parse_identity(pkt):
    return _identity(_tool('identity', [list(pkt)]))


def product_of(raw):
    try:
        return _codecs.call(14, [], [raw]).decode('latin-1')
    except _codecs.CodecError as error:
        raise InstallError('badpkg', 'not an FM-1 package (too short or damaged)') from error

def logical_image(raw):
    try:
        return _codecs.call(15, [], [raw])
    except _codecs.CodecError as error:
        raise InstallError('badpkg', 'not an FM-1 package (too short or damaged)') from error

def model_of(text):
    return _tool('model', [text])


# ------------------------------------------------------------------ MIDI ---

class MidoLink:
    """one MIDI in/out pair with a SysEx queue"""

    def __init__(self, backend, in_name, out_name):
        import mido
        _codecs.executable()  # build the host dependency before acquiring ports
        self.backend, self.name, self.mido = backend, in_name, mido
        self.q = queue.Queue()
        self.inp = mido.open_input(in_name, callback=self._rx)
        try:
            self.out = mido.open_output(out_name)
        except Exception:
            self.inp.close()
            raise
        time.sleep(DELAY["open"])   # CoreMIDI may drop a message sent just after opening

    def _rx(self, msg):
        if msg.type == "sysex":
            self.q.put(bytes([0xF0, *msg.data, 0xF7]))

    @property
    def lost(self):
        return self.name not in self.backend.input_names()

    def send(self, pkt):
        try:
            self.out.send(self.mido.Message("sysex", data=pkt[1:-1]))
            return True
        except Exception:
            return False

    def read(self, timeout):
        try:
            return self.q.get(timeout=max(timeout, 0))
        except queue.Empty:
            return None

    def drain(self):
        while not self.q.empty():
            self.q.get_nowait()

    def close(self):
        for p in (self.inp, self.out):
            try:
                p.close()
            except Exception:
                pass


class MidoBackend:
    def __init__(self):
        import mido
        self.mido = mido

    def input_names(self):
        return list(dict.fromkeys(self.mido.get_input_names()))

    def output_names(self):
        return list(dict.fromkeys(self.mido.get_output_names()))

    def open(self, in_name, out_name):
        return MidoLink(self, in_name, out_name)


def pair_output(name, outs):
    return _tool('pair', [name, list(outs)])


# --------------------------------------------------------------- updater ---

class Device:
    def __init__(self, link, ident, name):
        self.link, self.id, self.name = link, ident, name


class _Adapter:
    """Link handles are local to this serialized call; Rust retains no OS object."""
    def __init__(self, backend=None, links=(), want=None, step=None, progress=None):
        self.backend, self.want = backend, want
        self.step, self.progress = step, progress
        self.links = list(links)

    def device(self, dev):
        link = len(self.links)
        self.links.append(dev.link)
        return dict(link=link, id=dict(text=dev.id.text, model=dev.id.model,
                                     version=dev.id.version, loader=dev.id.loader), name=dev.name)

    def result(self, value):
        return Device(self.links[value['link']], _identity(value['id']), value['name']) if value else None

    def __call__(self, action):
        kind, a = action['kind'], action['args']
        if kind == 'inputs':
            return self.backend.input_names()
        if kind == 'outputs':
            return self.backend.output_names()
        if kind == 'open':
            link = self.backend.open(*a)
            self.links.append(link)
            return len(self.links) - 1
        if kind == 'clock':
            return time.monotonic()
        if kind == 'sleep':
            time.sleep(a[0])
            return None
        if kind == 'want':
            return bool(self.want(_identity(a[0])))
        if kind == 'step':
            self.step(a[0], a[1]) if a[1] is not None else self.step(a[0])
            return None
        if kind == 'progress':
            self.progress(*a)
            return None
        link = self.links[a[0]]
        if kind == 'read':
            pkt = link.read(a[1])
            return list(pkt) if pkt is not None else None
        if kind == 'send':
            return bool(link.send(bytes(a[1])))
        if kind == 'lost':
            return bool(link.lost)
        if kind == 'drain':
            link.drain()
            return None
        if kind == 'close':
            link.close()
            return None
        raise RuntimeError('unknown installer I/O action: ' + kind)


def ask(link, query, secs):
    value = _tool('ask', [0, list(query), secs], _Adapter(links=[link]), delay=DELAY)
    return False if value is False else _identity(value)


def handshake(link, tries=3):
    return _identity(_tool('handshake', [0, tries], _Adapter(links=[link]), delay=DELAY))


class Updater:
    def __init__(self, backend, port=None):
        self.backend, self.port = backend, port

    def candidates(self, only_port):
        return _tool('candidates', [self.backend.input_names(), self.port, only_port])

    def find(self, want=None, only_port=True):
        adapter = _Adapter(self.backend, want=want)
        value = _tool('find', [only_port, 'callback' if want is not None else 'any'],
                      adapter, port=self.port, delay=DELAY)
        return adapter.result(value)

    def wait_for(self, want, secs):
        adapter = _Adapter(self.backend, want=want)
        return adapter.result(_tool('wait', [secs], adapter, port=self.port, delay=DELAY))

    def serve(self, link, image, finish, idle, progress):
        return tuple(_tool('serve', [0, list(image), finish, idle],
                           _Adapter(links=[link], progress=progress), delay=DELAY))

    def check(self, dev, image, step):
        adapter = _Adapter(self.backend, step=step)
        value = _tool('check', [adapter.device(dev), list(image)], adapter, port=self.port, delay=DELAY)
        return adapter.result(value)

    def write(self, ota, image, step):
        adapter = _Adapter(self.backend, step=step)
        return _tool('write', [adapter.device(ota), list(image)], adapter, port=self.port, delay=DELAY)

    def verify(self, product, step):
        return _tool('verify', [product], _Adapter(self.backend, step=step), port=self.port, delay=DELAY)

    def install(self, dev, image, product, step=lambda *a: None):
        adapter = _Adapter(self.backend, step=step)
        return _tool('install', [adapter.device(dev), list(image), product], adapter, port=self.port, delay=DELAY)


# ------------------------------------------------------------------- CLI ---

def load_package(path, force):
    try:
        raw = open(path, "rb").read()
    except OSError as e:
        raise InstallError("badpkg", f"cannot read {path}: {e.strerror}")
    product, image = _tool('package', [list(raw), force, str(path)])
    return product, bytes(image)


def not_found(up):
    names = up.backend.input_names()
    where = f"no MIDI port matching {up.port!r} answered" if up.port else "FM-1 not found"
    return InstallError("notfound", f"{where} (USB data cable? another app using it?)"
                        f"\n  MIDI inputs: {', '.join(names) if names else '(none)'}")


class Progress:
    def __init__(self, out):
        self.out, self.line = out, False

    def __call__(self, k, a=None):
        if k == "check":
            self.put(f"checking the package ({a} reads)")
        elif k == "write":
            self.put(f"writing {a:3d}%")
        else:
            self.end()
            msg = {"start": f"starting the update on {a}", "loader": "switching to update mode",
                   "reboot": "restarting", "done": f"done: the FM-1 runs {a}"}[k]
            print(msg, file=self.out, flush=True)

    def put(self, s):
        print(f"\r{s}\033[K", end="", file=self.out, flush=True)
        self.line = True

    def end(self):
        if self.line:
            print(file=self.out, flush=True)
            self.line = False


def run(a, backend, out, ask):
    up = Updater(backend, a.port)
    if a.info:
        dev = up.find()
        if not dev:
            raise not_found(up)
        dev.link.close()
        mode = "update loader (update not finished)" if dev.id.loader else "running"
        print(f"{dev.id.text}  [{mode}]  port: {dev.name}", file=out)
        return 0
    product, image = load_package(a.package, a.force)
    print(f"package: {product}  ({a.package})", file=out)
    dev = up.find()
    if not dev:
        raise not_found(up)
    print(f"device:  {dev.id.text}  ({dev.name})" + ("  in update mode: the write will be finished" if dev.id.loader else ""),
          file=out)
    if not a.yes and not ask("Install? Do not unplug the FM-1 while writing. [y/N] "):
        dev.link.close()
        print("cancelled", file=out)
        return 1
    prog = Progress(out)
    try:
        up.install(dev, image, product, prog)
    finally:
        prog.end()
    return 0


def ask_tty(prompt):
    try:
        return input(prompt).strip().lower() in ("y", "yes")
    except EOFError:
        return False


def main(argv=None, backend=None, out=sys.stdout, ask=ask_tty):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("package", nargs="?", help="Felucca package (.fwsc)")
    ap.add_argument("--info", action="store_true", help="print the identity of the connected FM-1")
    ap.add_argument("--port", metavar="NAME", help="MIDI port to use (part of its name)")
    ap.add_argument("--yes", action="store_true", help="do not ask for confirmation")
    ap.add_argument("--force", action="store_true", help="install a package without the Felucca loader marker")
    a = ap.parse_args(argv)
    if bool(a.info) == bool(a.package):
        ap.error("give a PACKAGE.fwsc or --info")
    try:
        if backend is None:
            try:
                backend = MidoBackend()
            except ImportError:
                raise InstallError("usage", "needs mido and python-rtmidi (pip install mido python-rtmidi)")
        return run(a, backend, out, ask)
    except InstallError as e:
        print(f"error: {e}", file=sys.stderr)
        return EXIT.get(e.code, 1)
    except KeyboardInterrupt:
        print("\ninterrupted: if the FM-1 is in update mode, run the install again to finish", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
