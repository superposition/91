# SPDX-License-Identifier: GPL-3.0-only
"""Thin host binding to the same bounded safe dispatcher used by the browser."""
from functools import lru_cache
import atexit
import os
from pathlib import Path
import struct
import subprocess
import threading

ROOT = Path(__file__).resolve().parents[1]
CAPACITY = 1_100_000
_lock = threading.RLock()
_process = None


class CodecError(ValueError):
    def __init__(self, code):
        self.code = code
        super().__init__(f"shared Rust codec rejected the input ({code})")


@lru_cache(maxsize=1)
def executable():
    override = os.environ.get("FM1_CODEC_CLI")
    if override:
        path = Path(override).resolve()
        if not path.is_file():
            raise RuntimeError(f"FM1_CODEC_CLI does not exist: {path}")
        return path
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.94.1")
    # This is a host tool, independent of the firmware cross-compiler flags.
    for key in ("CARGO_BUILD_TARGET", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    # Windows MIDI tools and WSL builds can run against the same checkout.
    target = ROOT / "build/rust-codecs-host" / os.name
    subprocess.run(["cargo", "+1.94.1", "build", "--locked", "--release", "-p", "fm1-cli", "--target-dir", str(target)], cwd=ROOT, env=env, check=True)
    return target / "release" / ("fm1-rs.exe" if os.name == "nt" else "fm1-rs")


def call(operation, args=(), blobs=()):
    if len(args) > 32 or len(blobs) > 16:
        raise CodecError(-1)
    chunks = [bytes(blob) for blob in blobs]
    length = 256 + sum(map(len, chunks))
    if length > CAPACITY:
        raise CodecError(-8)
    request = bytearray(256)
    for index, value in enumerate(args):
        struct.pack_into("<I", request, index * 4, value & 0xffffffff)
    offset = 256
    for index, chunk in enumerate(chunks):
        struct.pack_into("<II", request, 128 + index * 8, offset, len(chunk))
        request.extend(chunk)
        offset += len(chunk)
    global _process
    with _lock:
        if _process is None:
            flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
            _process = subprocess.Popen([str(executable()), "codec-stream"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, creationflags=flags)
        try:
            _process.stdin.write(struct.pack("<II", operation, len(request)) + request)
            _process.stdin.flush()
            size = struct.unpack("<i", _read_exact(_process.stdout, 4))[0]
            if size < 0:
                raise CodecError(size)
            if size > CAPACITY:
                raise RuntimeError("shared Rust codec returned an invalid response length")
            return _read_exact(_process.stdout, size)
        except (OSError, RuntimeError):
            close()
            raise


def _read_exact(stream, length):
    data = bytearray()
    while len(data) < length:
        chunk = stream.read(length - len(data))
        if not chunk:
            raise RuntimeError("shared Rust codec returned a truncated response")
        data.extend(chunk)
    return bytes(data)


def close():
    global _process
    with _lock:
        previous, _process = _process, None
        if previous is None:
            return
        try:
            previous.stdin.close()
        except OSError:
            pass
        try:
            previous.wait(timeout=2)
        except subprocess.TimeoutExpired:
            previous.kill()
            previous.wait(timeout=2)
        previous.stdout.close()


atexit.register(close)


def scalar(operation, args=(), blobs=()):
    return struct.unpack("<I", call(operation, args, blobs))[0]
