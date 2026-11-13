#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
"""Build HYP-1 (a fork of AMB-1, built on Felucca as X0X is): the app, the update loader and an installable .fwsc package.

  tools/build.py [--release X.Y[-suffix]]

Outputs in build/: amb.bin (app), loader/ota.bin (update loader, Felucca's, unchanged),
amb.fwsc (package). See BUILDING.md for the toolchain and the SDK.

The JieLi toolchain is Linux x86-64 only. JIELI_TOOLCHAIN points at it; on
macOS (or with JIELI_DOCKER=1) each tool runs in a linux/amd64 container.
"""
import argparse
import hashlib
import os
import platform
import re
import struct
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

SRC = Path(__file__).resolve().parents[1]
FW = SRC / "firmware"
OUT = SRC / "build"
GEN = OUT / "gen"
LDR = OUT / "loader"
STB = OUT / "stub"
sys.path.insert(0, str(SRC / "tools"))
import fm1pkg_make  # noqa: E402
import lz4blk  # noqa: E402
import rust_tooling  # noqa: E402

STUB_XIP = 0x02000120               # app.bin offset 0 in the XIP map, where the SPL jumps: the boot stub
STUB_LEN = fm1pkg_make.STUB_LEN
APP_XIP = 0x02001000                # the body (the app proper), after the stub: its _start (app.ld)
# the stub's own flags, exactly Melodee 1.3.5's (tools/build.py CFLAGS), so the same sources give the bytes its stub.lock pins
STUB_CFLAGS = ["-mcpu=r3", "-mfprev1", "-ffp-contract=off", "-Werror=double-promotion", "-Os", "-ffunction-sections",
               "-fdata-sections", "-fno-builtin", "-Wall", "-Wno-unused-function"]
APP_SLOT = fm1pkg_make.APP_SLOT
LOADER_LOAD = 0x01C0A800
LOADER_NAME = b"usb_hid_ota.bin"    # the file name the SPL looks for
DOCKER_IMAGE = os.environ.get("JIELI_DOCKER_IMAGE", "debian:bookworm-slim")
CFLAGS = ["-Os", "-ffunction-sections", "-fno-builtin", "-Wall", "-Wno-unused-function"]
# the AC79's single-precision FPU (the SDK's own flags); no fused multiply-add, so the host
# build (-ffp-contract=off) computes the same samples
FPU = ["-mcpu=r3", "-mfprev1", "-ffp-contract=off"]
# the instrument: a separate unit at -O2
O2_UNITS = ["dsp/amb.c"]
LINE = re.compile(r"^\s*([0-9a-f]+):\s+((?:[0-9a-f]{2} )+)\s*\t(.*)$")

# SDK files of AC79NN_SDK_V1.2.1_2023-12-13 (the tested version)
SDK_SHA256 = {
    "uboot.boot": "4e3b4c220dc96641cb5a723f41e68ce41d5261ae9434bb33fbd7f2c59976ded4",
    "cfg_tool.bin": "276579954f076886a6a7694f65dc71c034a63a2c204b76749065c0ac7b010d1b",
    "cfg/eq_cfg_hw.bin": "41167491bffed4651750719c973d2758adeb9021a5670d02d6a53c85ed80ea7d",
}

PRODUCT = "FM-1_700"                # package identity; release builds are FM-1_7XXYYZZ (see main)
VERSION = None                      # FELUCCA_VERSION for release builds (default: firmware/src/ui.c)


def toolchain():
    tc = os.environ.get("JIELI_TOOLCHAIN")
    if not tc or not (Path(tc) / "pi32v2" / "bin" / "clang").exists():
        raise SystemExit("JIELI_TOOLCHAIN must point at the JieLi Linux toolchain "
                         "(the directory with pi32v2/ and common/; see tools/get_toolchain.sh)")
    return Path(tc).resolve()


def use_docker():
    native = platform.system() == "Linux" and platform.machine() in ("x86_64", "AMD64")
    return os.environ.get("JIELI_DOCKER", "0" if native else "1") == "1"


_BUILDER = None
_BUILDER_LOCK = __import__("threading").Lock()


def builder():
    """one long-lived toolchain container per source tree (starting a container per tool call cost
    most of the build); it stops by itself after an hour without a build"""
    global _BUILDER
    with _BUILDER_LOCK:            # the loader and the app build in parallel: start it once
        return _builder_start()


def _builder_start():
    global _BUILDER
    if _BUILDER:
        return _BUILDER
    name = "jieli-" + hashlib.sha1(str(SRC).encode()).hexdigest()[:10]
    st = subprocess.run(["docker", "inspect", "-f", "{{.State.Running}}", name], capture_output=True, text=True)
    if st.stdout.strip() != "true":
        subprocess.run(["docker", "rm", "-f", name], capture_output=True)
        r = subprocess.run(["docker", "run", "-d", "--rm", "--name", name, "--platform", "linux/amd64",
                            "-v", f"{SRC}:/work", "-v", f"{toolchain()}:/opt/jieli:ro", "-w", "/work",
                            DOCKER_IMAGE, "sh", "-c", "sleep 3600"], capture_output=True, text=True)
        if r.returncode:
            sys.stderr.write(r.stderr)
            raise SystemExit("build: cannot start the toolchain container")
    else:                           # an hour from now again
        subprocess.run(["docker", "exec", "-d", name, "sh", "-c", "sleep 3600 && kill 1"], capture_output=True)
    _BUILDER = name
    return name


def tc(tool, *args):
    """run a toolchain binary (pi32v2/bin/..., common/bin/...) with cwd SRC; paths relative to SRC"""
    rel = [str(Path(a).resolve().relative_to(SRC)) if isinstance(a, Path) else a for a in args]
    if tool == "cc":                # the toolchain's cc wrapper needs python3; call clang directly
        tool, rel = "pi32v2/bin/clang", ["-target", "pi32v2", *rel]
    if use_docker():
        cmd = ["docker", "exec", "-w", "/work", builder(), f"/opt/jieli/{tool}", *rel]
    else:
        cmd = [str(toolchain() / tool), *rel]
    r = subprocess.run(cmd, cwd=SRC, capture_output=True, text=True)
    if r.returncode:
        sys.stderr.write(r.stdout + r.stderr)
        raise SystemExit(f"build: {tool} failed")
    if r.stderr.strip():
        sys.stderr.write(r.stderr)
    return r.stdout


def tc_all(*cmds):
    # at most 4 at once: a podman machine drops concurrent container connections (ssh EOF)
    with ThreadPoolExecutor(min(len(cmds), int(os.environ.get("EN_JOBS", "4")))) as ex:
        return list(ex.map(lambda c: tc(*c), cmds))


def generate():
    """generated headers: the font (Felucca's)"""
    GEN.mkdir(parents=True, exist_ok=True)
    r = subprocess.run([sys.executable, str(SRC / "tools" / "gen_font.py"), str(GEN / "felucca_font.h")],
                       capture_output=True, text=True)
    sys.stdout.write(r.stdout)
    if r.returncode:
        sys.stderr.write(r.stderr)
        raise SystemExit("build: gen_font.py failed")


# ---- update loader

def _policy(method, args):
    try:
        return rust_tooling.call('build.' + method, args)
    except rust_tooling.ToolError as error:
        if error.kind == 'SystemExit':
            raise SystemExit(str(error)) from None
        raise ValueError(str(error)) from None


def crc16(d, c=0):
    return fm1pkg_make.crc16(d, c)

def ldr_head(dcrc, a, b, attr, name):
    return fm1pkg_make.entry(dcrc, a, b, attr, 0, 0, name)

def ldr_wrap(image):
    """ota.bin = outer JLFS head (32 B) + inner head (32 B, load address) +
    blocks [clen u32][dlen u32][LZ4 block], dlen 4096 except the last"""
    try:
        return bytes(rust_tooling.call('build.loader-wrap',[list(image)]))
    except rust_tooling.ToolError as error:
        raise SystemExit(str(error)) from None


def build_loader():
    LDR.mkdir(parents=True, exist_ok=True)
    src = FW / "loader"
    flags = [*CFLAGS, "-Ifirmware/hal", "-Ifirmware/src"]
    tc_all(("cc", "-c", src / "crt0_ldr.S", "-o", LDR / "crt0_ldr.o"),
           ("cc", *flags, "-c", src / "loader.c", "-o", LDR / "loader.o"))
    elf = LDR / "loader.elf"
    tc("pi32v2/bin/ld", "--gc-sections", "-e", "_start", "-T", src / "loader.ld",
       LDR / "crt0_ldr.o", LDR / "loader.o", "-o", elf)
    _, dis, hdr, syms = tc_all(("common/bin/objcopy", "-O", "binary", "-j", ".text", elf, LDR / "loader.bin"),
                               ("common/bin/objdump", "-d", elf),
                               ("common/bin/objdump", "-h", elf),
                               ("common/bin/objdump", "-t", elf))
    (LDR / "loader.dis").write_text(dis)
    _policy('loader-sections', [hdr])
    image = (LDR / "loader.bin").read_bytes()
    _policy('loader-image', [len(image)])
    ota = ldr_wrap(image)
    (LDR / "ota.bin").write_bytes(ota)
    print(_policy('loader-summary', [len(image), syms]))
    print(f"loader: ota.bin {len(ota)} B")
    return ota


# ---- boot stub (0.16.17): ChoralRoot FM-1 v0.16's, as Melodee 1.3.5 ships it (version 3), unchanged

def build_stub():
    """firmware/stub -> app.bin's first STUB_LEN bytes (0xFF padded). Code and constants only, built from its own
    sources alone, so it is the same bytes in every build: the loader rewrites sector 0x4000 only when it changes
    (firmware/stub/stub_core.h). firmware/stub/stub.lock records the version and hash of the stub that is meant to ship.
    (Adapted from Melodee's tools/build.py, Kerem Kilic / ChoralRoot FM-1 contributors, GPL-3.0-only.)"""
    STB.mkdir(parents=True, exist_ok=True)
    src = FW / "stub"
    tc_all(("cc", "-Ifirmware/stub", "-c", src / "crt0_stub.S", "-o", STB / "crt0_stub.o"),
           ("cc", *STUB_CFLAGS, "-Ifirmware/stub", "-c", src / "stub.c", "-o", STB / "stub.o"))
    elf = STB / "stub.elf"
    tc("pi32v2/bin/ld", "--gc-sections", "-e", "_start", "-T", src / "stub.ld",
       STB / "crt0_stub.o", STB / "stub.o", "-o", elf)
    (STB / "stub.bin").unlink(missing_ok=True)
    _, dis, hdr, syms = tc_all(("common/bin/objcopy", "-O", "binary", "-j", ".text", elf, STB / "stub.bin"),
                               ("common/bin/objdump", "-d", elf),
                               ("common/bin/objdump", "-h", elf),
                               ("common/bin/objdump", "-t", elf))
    (STB / "stub.dis").write_text(dis)
    img = (STB / "stub.bin").read_bytes()
    inspected = _policy('stub-inspect', [list(img), hdr, syms, dis])
    img = bytes(inspected['image'])
    print(inspected['line'])
    lock = src / "stub.lock"
    decision = _policy('stub-lock', [lock.read_text() if lock.exists() else None,
                                   str(lock.relative_to(SRC)), inspected['version'], inspected['sha'],
                                   os.environ.get('FM1_STUB_LOCK') == 'update', bool(VERSION)])
    if decision['kind'] == 'update':
        lock.write_text(decision['write'])
    if 'line' in decision:
        print(decision['line'])
    return img


# ---- app

def build_app(stub):
    os.environ.setdefault("EN_CORE2", "1")      # the reverb on the second core (fallback to one core built in); EN_CORE2=0 for single core
    flags = [*CFLAGS, *FPU, "-Ifirmware/hal", "-Ifirmware/src", "-Ibuild/gen"]
    for flag in ("EN_CDC", "EN_CORE2", "EN_DIAG"):
        v = os.environ.get(flag)    # unset: the default in firmware/src/main.c (EN_DIAG: firmware/src/dsp/amb.h)
        if v in ("0", "1"):
            flags.append(f"-D{flag}={v}")
    flags.append(f'-DFELUCCA_ID="{PRODUCT}"')
    if VERSION:
        flags.append(f'-DEN_VERSION="{VERSION}"')
    o2 = [f if f != "-Os" else "-O2" for f in flags]
    asm = ["-DEN_CORE2=1"] if os.environ.get("EN_CORE2") == "1" else []      # the assembly files test it too
    units = [("cc", *asm, "-c", FW / "crt0.S", "-o", OUT / "crt0.o"),
             ("cc", *asm, "-c", FW / "hal" / "fm1_vec.S", "-o", OUT / "fm1_vec.o"),
             ("cc", "-c", FW / "hal" / "fm1_isr.S", "-o", OUT / "fm1_isr.o"),
             ("cc", *flags, "-c", FW / "src" / "main.c", "-o", OUT / "main.o")]
    objs = [OUT / "crt0.o", OUT / "fm1_vec.o", OUT / "fm1_isr.o", OUT / "main.o"]
    (OUT / "o2").mkdir(parents=True, exist_ok=True)
    for u in O2_UNITS:
        o = OUT / "o2" / (u.replace("/", "_")[:-2] + ".o")
        units.append(("cc", *o2, "-c", FW / "src" / u, "-o", o))
        objs.append(o)
    tc_all(*units)
    elf = OUT / "amb.elf"
    # unreferenced functions (each in its own section: -ffunction-sections) are dropped: the DSP keeps
    # mono reference paths for the host tests that the firmware no longer calls
    tc("pi32v2/bin/ld", "--gc-sections", "-e", "_start", "-T", FW / "app.ld", *objs, "-o", elf)
    for sect in ("text.bin", "data.bin", "ramtext.bin", "dsptext.bin", "ramhot.bin"):
        (OUT / sect).unlink(missing_ok=True)
    *_, syms, dis, rt = tc_all(("common/bin/objcopy", "-O", "binary", "-j", ".text", elf, OUT / "text.bin"),
                               ("common/bin/objcopy", "-O", "binary", "-j", ".data", elf, OUT / "data.bin"),
                               ("common/bin/objcopy", "-O", "binary", "-j", ".ram_text", elf, OUT / "ramtext.bin"),
                               ("common/bin/objcopy", "-O", "binary", "-j", ".dsp_text", elf, OUT / "dsptext.bin"),
                               ("common/bin/objcopy", "-O", "binary", "-j", ".ram_hot", elf, OUT / "ramhot.bin"),
                               ("common/bin/objdump", "-t", elf),
                               ("common/bin/objdump", "-d", elf),
                               ("common/bin/objdump", "-d", "-j", ".ram_text", elf))
    (OUT / "amb.dis").write_text(dis)

    def symv(name):
        return int(re.search(r"^([0-9a-f]+) .*\s" + name + r"$", syms, re.M).group(1), 16)
    img = bytearray((OUT / "text.bin").read_bytes())
    # .ram_text and .data follow .text at their load addresses; crt0 copies them by words
    for sect, lname in (("ramtext.bin", "_rt_load"), ("dsptext.bin", "_dsp_load"), ("ramhot.bin", "_rh_load"), ("data.bin", "_data_load")):
        load = symv(lname)
        if load % 4:
            raise SystemExit(f"{lname} {load:#x} is not word aligned")
        blob = (OUT / sect).read_bytes() if (OUT / sect).exists() else b""
        if blob:
            if load - APP_XIP < len(img):
                raise SystemExit(f"{lname} overlaps the image")
            img += b"\xff" * (load - APP_XIP - len(img))
            img += blob
    img += b"\xff" * (-len(img) % 4)
    img = bytearray(stub) + img                 # app.bin: the stub (flash 0x4120..0x4FFF), then the body (0x5000 on)
    (OUT / "amb.bin").write_bytes(img)
    return bytes(img), syms, dis, rt


def check(img, syms, dis, rt):
    try:
        return tuple(rust_tooling.call('build.check',[list(img),syms,dis,rt]))
    except rust_tooling.ToolError as error:
        raise ValueError(str(error)) from None


# Every hardware register access lives in hal/. In src/ and loader/ (comments stripped):
#  - no volatile pointer cast, except of a C object's address (`*(volatile T *)&x`: a read-once
#    of a RAM flag shared with an ISR, e.g. usb.c ota_wire_send);
#  - no literal in a register or reserved window (core SFRs 0x10000-0x13FFF, SFC 0x40000-0x43FFF,
#    GPIO/IOMAP 0x50000-0x51FFF, CPU 0x1EE0000-0x1EEFFFF, RAM top 0x01C7F000- (boot info,
#    mailbox, vectors), XIP 0x02000000-0x020FFFFF);
#  - no inline asm, except the empty compiler barrier RING_PUBLISH() (emits no instruction);
#  - no HAL register macro (a hal/ #define that is, or expands to, a volatile access).
# Linker symbols (_bss_start[], _rt_load[], ...) are plain C objects and pass.
def mmio_check():
    headers = [(str(h.relative_to(FW)), h.read_text())
               for h in sorted((FW / "hal").glob("*.h"))]
    sources = [(str(f.relative_to(FW)), f.read_text())
               for f in sorted([*(FW / "src").rglob("*.[ch]"), *(FW / "loader").glob("*.c")])]
    return rust_tooling.call('build.mmio-check', [headers, sources])


def main():
    global PRODUCT, VERSION
    ap = argparse.ArgumentParser()
    ap.add_argument("--release", metavar="X.Y[.Z]", help="release build: identity FM-1_7XXYYZZ, version string X.Y[.Z]")
    ap.add_argument("--sdk", type=Path, help="JieLi AC79 SDK checkout (default: $AC79_SDK)")
    a = ap.parse_args()
    release = _policy('release', [a.release, PRODUCT, VERSION])
    PRODUCT, VERSION, name = release['product'], release['version'], release['name']
    fm1pkg_make.SDK = a.sdk
    for rel, sha in SDK_SHA256.items():          # fail early without the SDK
        note = _policy('sdk-check', [rel, list(fm1pkg_make.sdk_file(rel)), sha])
        if note:
            print(note)
    OUT.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(2) as ex:
        gen, ldr = ex.submit(generate), ex.submit(build_loader)
        gen.result()
        ota = ldr.result()
    stub = build_stub()
    img, syms, dis, rt = build_app(stub)
    errors, notes = check(img, syms, dis, rt)
    hal_err = mmio_check()
    errors += hal_err
    if not hal_err:
        notes.append("register access: hal/ only (src/, loader/ clean)")
    for n in notes:
        print("  ok   ", n)
    for e in errors:
        print("  FAIL ", e)
    if errors:
        raise SystemExit("build: checks failed")
    pkg = fm1pkg_make.ufw(fm1pkg_make.flash_image(img, fm1pkg_make.KEY), ota, PRODUCT)
    (OUT / name).write_bytes(pkg)
    print(f"app      {OUT / 'amb.bin'}  {len(img)} B")
    print(f"loader   {LDR / 'ota.bin'}  {len(ota)} B")
    print(f"package  {OUT / name}  {len(pkg)} B, identity {PRODUCT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
