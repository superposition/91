#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
# Boot stub layout, commit marker and CRC compensation (0.16.17): Copyright (C) 2026 ChoralRoot FM-1 contributors
# (v0.16), with modifications Copyright (C) 2026 Kerem Kilic (Ellic Studio), from Melodee 1.3.5; GPL-3.0-only.
"""Build an FM-1 update package (.fwsc) from the Felucca app and update loader.

  flash.bin  head [0, 0x4000): flash header (JieLi SDK default values), JLFS
             entries, the SDK SPL (uboot.boot, Apache-2.0) and an isd_config
             blob that decodes to the chip key. The update loader never writes
             the head.
             app area 0x4000..: app_area_head, app.bin, the SDK cfg_tool.bin and
             eq_cfg_hw.bin, the region descriptors; SFC-encrypted with the chip key
  ota.bin    the Felucca update loader (firmware/loader)

The SDK files come from the JieLi AC79 SDK (AC79_SDK, or --sdk).
All integrity checks in the format are CRC16 (poly 0x1021, init 0).

  fm1pkg_make.py APP.bin OTA.bin OUT.fwsc [--product FM-1_9XY] [--sdk DIR]
"""
import argparse
import os
import struct
import rust_codecs as _codecs
from pathlib import Path

APP_SLOT = 0x8DFBC
FLASH_SIZE = 0x93000
KEY = 0x980F
# the boot stub's layout (firmware/stub/stub_core.h), as app.bin offsets (app.bin offset 0 = flash 0x4120)
STUB_LEN = 0xEE0                    # the stub; the body (the app proper) follows: flash 0x5000, XIP 0x02001000
STUB_SIG = b"CRSTUB"                # at app.bin + 8 (crt0_stub.S), then the stub's version (u16)
BODY_MAX = 0x8C000                  # the body ends below the marker's sector (flash 0x91000)
MARK_OFF = 0x91FE0 - 0x4120         # the marker: the last 32 bytes of flash sector 0x91000
MARK_MAGIC, MARK_FORMAT = 0x4D425243, 1     # "CRBM"
COMP_OFF = MARK_OFF + 20            # the four compensation bytes
APP_CRC_FIXED = 0x4352              # app.bin's CRC-16 in every stub package ("CR")
# flash header: JieLi SDK defaults (burner_size 544, VID "0.01", flash_size 0xFF000,
# fs_ver 0x10, PID "AC791N_STORY"), plain, without its CRC
FLASH_HDR = bytes.fromhex("20022f104cc900f00f001010a4ff4e5d0b41e0cd781dc856417fb367cf9f")
UFW_CHIP = b"AC791N"
SDK = None


def crc16(data, crc=0):
    return _codecs.scalar(70, [crc], [data])


def enc(buf, off, size, key=0xFFFF):
    """JieLi ENC stream cipher (in place), shared with native/WASM readers."""
    if off < 0 or size < 0 or off + size > len(buf):
        raise ValueError("ENC byte range is outside the buffer")
    buf[off:off + size] = _codecs.call(71, [key], [buf[off:off + size]])


def sfc(buf, off, size, base, key):
    if off < 0 or size < 0 or off + size > len(buf) or off < base:
        raise ValueError("SFC byte range/address is outside the buffer")
    buf[off:off + size] = _codecs.call(9, [off - base, 0, key], [buf[off:off + size]])


def chipkey_decode(data):
    return _codecs.scalar(10, [], [data[:32]])


def entry(dcrc, offset, size, flags, resvd, index, name, fill=0xFF):
    # Names are application strings; Rust owns the byte layout and checksum.
    nm = name.ljust(16, b"\0")[:16] if isinstance(name, bytes) else (name.encode() + b"\0").ljust(16, bytes([fill]))[:16]
    struct.pack("<HBBH", dcrc, flags, resvd, index)  # preserve Python range errors
    return _codecs.call(18, [dcrc, offset, size, flags, resvd, index], [nm])


def key_blob(key):
    return _codecs.call(11, [key])


def sdk_file(rel):
    root = Path(SDK or os.environ.get("AC79_SDK", ""))
    path = root / "cpu" / "wl82" / "tools" / rel
    try:
        return path.read_bytes()
    except OSError as e:
        raise SystemExit(f"fm1pkg_make: {path}: {e.strerror}. Set AC79_SDK to a JieLi AC79 SDK checkout.")


def has_stub(app):
    return len(app) >= STUB_LEN and app[8:8 + len(STUB_SIG)] == STUB_SIG


def stub_version(app):
    return struct.unpack_from("<H", app, 8 + len(STUB_SIG))[0] if has_stub(app) else None


def marker(body):
    return _codecs.call(12, [], [body])


def compensate(app, off, want):
    return _codecs.call(13, [off, want], [app])


def crc16_slow(data):
    """bit by bit, no table: the check of compensate() and of both directory CRCs"""
    c = 0
    for b in data:
        c ^= b << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) & 0xFFFF if c & 0x8000 else c << 1
    return c


def commit(app, marker_of=marker):
    """a stub app padded to the slot, with its marker and compensation bytes (marker_of allows
    host tests to supply a deliberately wrong marker)"""
    body = app[STUB_LEN:]
    body += b"\xFF" * (-len(body) % 4)
    if len(body) > BODY_MAX:
        raise SystemExit(f"the app body is {len(body)} B, the room below the marker's sector is {BODY_MAX} B")
    if not body:
        raise SystemExit("the app has a stub and no body")
    a = bytearray(app[:STUB_LEN] + body + b"\xFF" * (APP_SLOT - STUB_LEN - len(body)))
    a[MARK_OFF:MARK_OFF + 20] = marker_of(body)
    a[COMP_OFF:COMP_OFF + 4] = bytes(4)
    a[COMP_OFF:COMP_OFF + 4] = compensate(bytes(a), COMP_OFF, APP_CRC_FIXED)
    if crc16_slow(a) != APP_CRC_FIXED:
        raise SystemExit("fm1pkg_make: the compensated app.bin CRC-16 is wrong")
    return bytes(a)


def flash_image(app, key, marker_of=marker):
    spl = sdk_file("uboot.boot")                         # SDK SPL, BANKCB-encoded as in the SDK
    cfg_tool = sdk_file("cfg_tool.bin")
    eq = sdk_file("cfg/eq_cfg_hw.bin")
    isd = key_blob(key) + b"[FELUCCA]\r\n"
    f = bytearray(b"\xFF" * FLASH_SIZE)
    # ---- head
    spl_off = 0xA0
    isd_off = spl_off + len(spl)
    assert isd_off + len(isd) <= 0x4000
    f[spl_off:spl_off + len(spl)] = spl
    f[isd_off:isd_off + len(isd)] = isd
    top = (entry(crc16(spl), spl_off, len(spl), 0x00, 0x00, 0, "uboot.boot") +
           entry(crc16(isd), isd_off, len(isd), 0x02, 0x80, 0, "isd_config.ini") +
           entry(0xFFFF, 0x4000, 0xFFFFFFFF, 0x81, 0xFF, 0, "app_dir_head") +
           entry(0xFFFF, 0xFF000, 0x1000, 0x12, 0x01, 0xFFFF, "key_mac"))
    hdr = struct.pack("<H", crc16(FLASH_HDR)) + FLASH_HDR
    head = bytearray(hdr + top)
    for o in range(0, len(head), 32):                   # header and each JLFS entry: ENC 0xFFFF
        enc(head, o, 32)
    f[0:len(head)] = head
    # ---- app area (plain first, then SFC)
    if len(app) > APP_SLOT:
        raise SystemExit(f"app is {len(app)} B, the slot is {APP_SLOT} B")
    app = commit(app, marker_of) if has_stub(app) else app + b"\xFF" * (APP_SLOT - len(app))
    blk =0x120 + APP_SLOT + len(cfg_tool)              # app_area_head block size
    area = bytearray(b"\xFF" * blk)
    area[0x120:0x120 + APP_SLOT] = app
    area[0x120 + APP_SLOT:blk] = cfg_tool
    ents = [entry(crc16(app), 0x120, APP_SLOT, 0x82, 0xFF, 0, "app.bin"),
            entry(crc16(cfg_tool), 0x120 + APP_SLOT, len(cfg_tool), 0x82, 0xFF, 0, "cfg_tool.bin"),
            # region descriptors; the bytes after the names are SDK packer fields (VM size, "CODE", "AUTO")
            entry(0xFFFF, 0x93000, 352256, 0x12, 0x80, 0, bytes.fromhex("564d00ffffffffffffffffff00800000")),
            entry(0xFFFF, 0, FLASH_SIZE, 0x92, 0x82, 0, bytes.fromhex("5052435400ffffffffffffff434f4445")),
            entry(0xFFFF, 0xE9000, 0x1000, 0x92, 0x80, 0, bytes.fromhex("4254494600ffffffffffffff4155544f")),
            entry(0xFFFF, 0xEA000, 0x12000, 0x92, 0x81, 1, "USR")]
    area[32:32 + len(b"".join(ents))] = b"".join(ents)
    area[0:32] = entry(crc16(area[32:blk]), 0x02000120, blk, 0x83, 0xFF, 0, "app_area_head")
    # nested cfg directory right after the block: SDK default EQ table
    eq_e = entry(crc16(eq), 0x40, len(eq), 0x82, 0xFF, 1, "eq_cfg_hw.bin")
    cfg_body = eq_e + eq
    cfg = entry(crc16(cfg_body), 0x20, 0x20 + len(cfg_body), 0x83, 0xFF, 1, "cfg") + cfg_body
    region = bytearray(area + cfg)
    sfc(region, 0, len(region), 0, key)
    f[0x4000:0x4000 + len(region)] = region
    return bytes(f)


def ufw(flash, ota, product):
    try:
        # Match the original byte-valued product marker semantics.
        identity = bytes(ord(char) & 255 for char in product)
        return _codecs.call(99, [], [flash, ota, identity])
    except _codecs.CodecError as error:
        raise SystemExit(f"fm1pkg_make: invalid or oversized package input: {error}") from error


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("app", type=Path)
    ap.add_argument("ota", type=Path)
    ap.add_argument("out", type=Path)
    ap.add_argument("--product", default="FM-1_900")
    ap.add_argument("--key", type=lambda s: int(s, 0), default=KEY)
    ap.add_argument("--sdk", type=Path, help="JieLi AC79 SDK checkout (default: $AC79_SDK)")
    a = ap.parse_args()
    global SDK
    SDK = a.sdk
    pkg = ufw(flash_image(a.app.read_bytes(), a.key), a.ota.read_bytes(), a.product)
    a.out.write_bytes(pkg)
    print(f"package {a.out}: {len(pkg)} B, identity {a.product}, chip key {a.key:#06x}")


if __name__ == "__main__":
    main()
