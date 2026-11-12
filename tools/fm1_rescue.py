#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""fm1_rescue: put stock firmware back on an FM-1 through the chip's own update mode (the
mask-ROM "UBOOT"), from a Mac, with no extra hardware.

For an FM-1 whose firmware crashes at start-up. X0X 0.2-beta crashes at once, and after two failed
boots enters UBOOT, which shows up on USB as "WL80UBOOT1.00" (4C4A:8057). macOS then takes it for a
disk, upsets it, and it reboots into the crash a few seconds later; this script waits for it,
takes it away from macOS (hence sudo) and keeps it.

  python3 -m venv ~/fm1-rescue && ~/fm1-rescue/bin/pip install pyusb libusb-package
  sudo ~/fm1-rescue/bin/python fm1_rescue.py FM-1.fwsc            # check and back up, write nothing
  sudo ~/fm1-rescue/bin/python fm1_rescue.py FM-1.fwsc --write    # then write stock

FM-1.fwsc is M-VAVE's FM-1 V15 (m-vave.com/download, PC Firmware); only that exact file is
accepted. The protocol and the write policy are FM-1-transporter's (github.com/kurogedelic/
FM-1-transporter, docs/PROTOCOL.md), which were verified on hardware:

  1. JieLi's flash loader (wl82loader.bin, from github.com/kagaimiq/jl-uboot-tool, downloaded
     and checked against its hash) is loaded into RAM and started;
  2. the chip key must be 980F and the flash 856014 (Puya, the first units) or B36014 (UC25HQ80,
     the newer ones; both 1 MiB), or nothing is done;
  3. the whole flash is read and saved as a backup before anything else;
  4. only 4 KiB sectors in [0x4000, 0x93000) that differ from V15 are written (the bootloader below
     0x4000 and the device data above 0x93000 are never touched; a difference below 0x4000 stops
     it), each erased, written and read back;
  5. a final full read must equal the expected image; then the FM-1 restarts into V15.
"""
import argparse
import rust_codecs as _codecs
import rust_tooling as _tools
import threading
import os
import struct
import sys
import time
import urllib.request

_CONFIG = _tools.call('rescue.config')
VID, PID = _CONFIG['vid'], _CONFIG['pid']
STOCK_SHA256, LOADER_SHA256 = _CONFIG['stock_sha256'], _CONFIG['loader_sha256']
LOADER_URL = _CONFIG['loader_url']
LOADER_ADDR, LOADER_ARG = _CONFIG['loader_addr'], _CONFIG['loader_arg']
FLASH_IDS = tuple(_CONFIG['flash_ids'])
FLASH_SIZE, APP_END = _CONFIG['flash_size'], _CONFIG['app_end']
WRITE_MIN, SECTOR, IO = (_CONFIG[k] for k in ('write_min', 'sector', 'io'))

# ROM UBOOT1.00
CMD_WRITE_MEMORY, CMD_READ_MEMORY, CMD_JUMP = 0xFB06, 0xFD07, 0xFB08
# loader
CMD_ERASE_SECTOR, CMD_WRITE_FLASH, CMD_READ_FLASH = 0xFB01, 0xFB04, 0xFD05
CMD_READ_KEY, CMD_GET_ONLINE_DEVICE, CMD_RUN_APP = 0xFC09, 0xFC0A, 0xFC0C


class Fail(Exception):
    pass


# ------------------------------------------------------------------ maths ---
def crc16(data, crc=0):
    return _codecs.scalar(70, [crc], [data])

MAGIC = bytes([0xC3, 0xCF, 0xC0, 0xE8, 0xCE, 0xD2, 0xB0, 0xAE,
               0xC4, 0xE3, 0xA3, 0xAC, 0xD3, 0xF1, 0xC1, 0xD6])


def crc_cipher(buf):
    buf[:] = _codecs.call(100, [], [buf])

def enc(buf, key=0xFFFF):
    buf[:] = _codecs.call(71, [key], [buf])

def package_flash_image(raw):
    try:
        return _codecs.call(105, [], [raw])
    except _codecs.CodecError as error:
        message = 'package has no flash entry' if error.code == -4 else 'package flash entry: bad CRC' if error.code == -6 else 'not an FM-1 package (header)'
        raise Fail(message) from error

class Bot:
    """USB mass storage bulk-only transport: one SCSI-style command per call."""

    def __init__(self, dev, usb):
        self.dev, self.usb, self.tag = dev, usb, 1
        self._rust_lock = threading.RLock()
        cfg = dev.get_active_configuration() if self._configured() else None
        if cfg is None:
            dev.set_configuration()
            cfg = dev.get_active_configuration()
        intf = cfg[(0, 0)]
        self.intf = intf.bInterfaceNumber
        util = usb.util
        self.ep_out = util.find_descriptor(intf, custom_match=lambda e: util.endpoint_direction(
            e.bEndpointAddress) == util.ENDPOINT_OUT and util.endpoint_type(e.bmAttributes) == util.ENDPOINT_TYPE_BULK)
        self.ep_in = util.find_descriptor(intf, custom_match=lambda e: util.endpoint_direction(
            e.bEndpointAddress) == util.ENDPOINT_IN and util.endpoint_type(e.bmAttributes) == util.ENDPOINT_TYPE_BULK)
        if self.ep_out is None or self.ep_in is None:
            raise Fail("no bulk endpoints")
        util.claim_interface(dev, self.intf)

    def _configured(self):
        try:
            self.dev.get_active_configuration()
            return True
        except self.usb.core.USBError:
            return False

    def reset_recovery(self):
        _call('reset', adapter=_Adapter(bot=self), bulk=True)

    def xfer(self, cdb, data_out=None, n_in=0, timeout=5000):
        return bytes(_call('bot', [list(cdb), None if data_out is None else list(data_out), n_in, timeout],
                           _Adapter(bot=self), bulk=True))



def cdb_for(cmd, args=b''):
    return bytes(_call('cdb', [cmd, list(args)]))


# ----------------------------------------------------------------- device ---
class Uboot:
    def __init__(self, bot):
        self.bot = bot

    def _request(self, method, args=()):
        return _call(method, args, _Adapter(bot=self.bot), bulk=True)

    def inquiry(self):
        return tuple(self._request('inquiry'))

    def cmd(self, cmd, args=b''):
        return bytes(self._request('cmd', [cmd, list(args)]))

    def write_block(self, cmd, addr, data):
        self._request('write_block', [cmd, addr, list(data)])

    def start_loader(self, blob):
        self._request('start_loader', [list(blob)])

    def chip(self):
        return tuple(self._request('chip'))

    def read(self, addr, n):
        return bytes(self._request('read', [addr, n]))

    def write_sector(self, addr, data):
        self._request('write_sector', [addr, list(data)])

    def run_app(self):
        self._request('run_app')


# ------------------------------------------------------------------- flow ---
def _call(method, args=(), adapter=None, isolated=False, **values):
    try:
        call = _tools.isolated if isolated else _tools.call
        return call('rescue.' + method, args, adapter, **values)
    except _tools.ToolError as error:
        kinds = {'Fail': Fail, 'ValueError': ValueError, 'StructError': struct.error,
                 'IndexError': IndexError, 'SystemExit': SystemExit}
        raise kinds.get(error.kind, RuntimeError)(str(error)) from error


class _Adapter:
    def __init__(self, dev=None, bot=None, say=print, confirm=None, stop=None, usb=None):
        self.dev, self.bot, self.usb = dev, bot, usb
        self.say, self.confirm, self.stop = say, confirm, stop
        if bot is not None and not hasattr(bot, '_rust_lock'):
            bot._rust_lock = threading.RLock()

    def __call__(self, action):
        kind, a = action['kind'], action['args']
        if kind == 'say':
            self.say(*a) if action['end'] is None else self.say(*a, end=action['end'])
            return None
        if kind == 'clock':
            return time.time()
        if kind == 'timestamp':
            return time.strftime('%Y%m%d-%H%M%S')
        if kind == 'backup':
            with open(a[0], 'wb') as stream:
                stream.write(bytes(a[1]))
            return None
        if kind == 'confirm_keep_alive':
            return bool(keep_alive(self.dev, self.confirm))
        if kind == 'stop_wait':
            return self.stop.wait(a[0])
        if kind == 'sleep':
            time.sleep(a[0])
            return None
        if kind == 'usb_backend':
            self.backend = backend(self.usb)
            return None
        if kind == 'usb_find':
            self.candidate = self.usb.core.find(idVendor=a[0], idProduct=a[1], backend=self.backend)
            return self.candidate is not None
        if kind == 'usb_detach':
            try:
                if self.candidate.is_kernel_driver_active(0):
                    self.candidate.detach_kernel_driver(0)
            except (NotImplementedError, self.usb.core.USBError) as error:
                if getattr(error, 'errno', None) not in (None, 2):
                    print(f"  (could not detach macOS's driver: {error})")
            return None
        if kind == 'usb_open':
            self.bot = Bot(self.candidate, self.usb)
            self.dev = Uboot(self.bot)
            return None
        if kind == 'bulk_reset':
            self.bot.dev.ctrl_transfer(0x21, 0xFF, 0, self.bot.intf, None, timeout=2000)
            return None
        if kind == 'bulk_clear_in':
            self.bot.dev.clear_halt(self.bot.ep_in)
            return None
        if kind == 'bulk_clear_out':
            self.bot.dev.clear_halt(self.bot.ep_out)
            return None
        if kind == 'usb_dispose':
            self.usb.util.dispose_resources(self.bot.dev)
            return None
        if kind == 'bulk_begin':
            self.bot._rust_lock.acquire()
            return None
        if kind == 'bulk_end':
            self.bot._rust_lock.release()
            return None
        if kind == 'tag':
            return self.bot.tag
        if kind == 'tag_set':
            self.bot.tag = a[0]
            return None
        if kind == 'bulk_write':
            self.bot.ep_out.write(bytes(a[0]), a[1])
            return None
        if kind == 'bulk_read':
            return list(self.bot.ep_in.read(*a))
        method = kind.removeprefix('dev_') if hasattr(str, 'removeprefix') else kind[4:]
        if method in ('inquiry', 'chip'):
            return list(getattr(self.dev, method)())
        if method in ('start_loader',):
            getattr(self.dev, method)(bytes(a[0]))
            return None
        if method == 'cmd':
            return list(self.dev.cmd(a[0], bytes(a[1])))
        if method == 'read':
            return list(self.dev.read(*a))
        if method == 'write_sector':
            self.dev.write_sector(a[0], bytes(a[1]))
            return None
        if method == 'run_app':
            self.dev.run_app()
            return None
        raise RuntimeError('unknown rescue I/O action: ' + kind)


def rescue(dev, image, loader, write, backup_dir, say=print, confirm=None):
    bulk = isinstance(dev, Uboot)
    adapter = _Adapter(dev, dev.bot if bulk else None, say=say, confirm=confirm)
    return _call('restore', [list(image), list(loader), bool(write), str(backup_dir), confirm is not None],
                 adapter, bulk=bulk)


def keep_alive(dev, confirm):
    stop, lost = threading.Event(), []
    bulk = isinstance(dev, Uboot)
    adapter = _Adapter(dev, dev.bot if bulk else None, stop=stop)
    def run():
        try:
            value = _call('keep_alive', adapter=adapter, isolated=True, bulk=bulk)
            if value['lost'] is not None:
                lost.append(value['lost'])
        except Exception as error:
            lost.append(str(error))
    thread = threading.Thread(target=run, daemon=True)
    thread.start()
    try:
        answer = confirm()
    finally:
        stop.set()
        thread.join()
    if lost:
        raise Fail(f'the FM-1 dropped out while waiting ({lost[0]}); run it again')
    return answer


def get_loader(path):
    if path:
        blob = open(path, "rb").read()
    else:
        print("downloading JieLi's flash loader (wl82loader.bin) ...")
        with urllib.request.urlopen(LOADER_URL, timeout=30) as r:
            blob = r.read()
    _call('loader', [list(blob)])
    return blob


def backend(usb):
    """libusb: pip's libusb-package (no Homebrew needed), else the system's or Homebrew's"""
    try:
        import libusb_package
        b = libusb_package.get_libusb1_backend()
        if b is not None:
            return b
    except ImportError:
        pass
    import usb.backend.libusb1 as lb
    for path in _CONFIG['backend_paths']:
        b = lb.get_backend() if path is None else (lb.get_backend(find_library=lambda _: path)
                                                   if os.path.exists(path) else None)
        if b is not None:
            return b
    raise Fail("libusb not found: pip install libusb-package")


def open_device(usb, timeout):
    adapter = _Adapter(usb=usb)
    _call('open', [timeout], adapter, bulk=True)
    return adapter.bot


def ask():
    print("\n  Everything checks out, and the backup is saved.")
    try:
        return input("  Type yes and press Return to put the stock firmware back: ").strip().lower() == "yes"
    except EOFError:
        return False


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("package", help="M-VAVE's FM-1 V15 .fwsc")
    ap.add_argument("--write", action="store_true", help="write V15 (without it: check and back up only)")
    ap.add_argument("--ask", action="store_true", help="after the check and the backup, ask before writing")
    ap.add_argument("--loader", help="a local wl82loader.bin instead of downloading it")
    ap.add_argument("--wait", type=float, default=120, help="seconds to wait for the FM-1 (default 120)")
    ap.add_argument("--tries", type=int, default=5, help="attempts if it drops out mid-way (default 5)")
    args = ap.parse_args()
    try:
        raw = open(args.package, "rb").read()
        image = bytes(_call('stock', [list(raw), args.package]))
        loader = get_loader(args.loader)
        _call('uid', [os.geteuid()])
        try:
            import usb.core
            import usb.util
        except ImportError:
            raise Fail("pyusb is missing: ~/fm1-rescue/bin/pip install pyusb libusb-package")
        backup_dir = os.path.dirname(os.path.abspath(args.package))
        adapter = _Adapter(usb=usb, say=lambda *a, **k: print(*a, **k, flush=True),
                           confirm=ask if args.ask else None)
        return _call('recover', [list(image), list(loader), args.write, backup_dir,
                                 args.ask, args.tries, args.wait], adapter, bulk=True)
    except Fail as e:
        print(f"fm1_rescue: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
