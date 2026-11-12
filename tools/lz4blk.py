# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
"""Existing Python block-codec API backed by the Rust tooling implementation."""
import rust_tooling

def _call(method,*args):
    try:
        return bytes(rust_tooling.call('build.lz4.'+method,args))
    except rust_tooling.ToolError as error:
        kind={'IndexError':IndexError,'ValueError':ValueError}.get(error.kind,RuntimeError)
        raise kind(str(error)) from None

def _len_bytes(n):
    return bytearray(_call('length',n))

def compress(src: bytes) -> bytes:
    return _call('compress',list(src))

def decompress(blk: bytes,prefix: bytes=b'') -> bytes:
    return _call('decompress',list(blk),list(prefix))
