# SPDX-License-Identifier: GPL-3.0-only
"""File/API binding to the maintained safe Rust authoring implementation."""
import json
import os
from pathlib import Path
import subprocess
import atexit
import struct
import threading
import rust_codecs

ROOT = Path(__file__).resolve().parents[1]
_process = None
_lock = threading.RLock()


def call(method, **values):
    limits = ROOT / 'build/params.json'
    request = dict(method=method, **values)
    request['limits'] = json.loads(limits.read_text(encoding='utf-8')) if limits.exists() else None
    executable = os.environ.get('FM1_AUTHORING_CLI') or rust_codecs.executable()
    data = json.dumps(request).encode('utf-8')
    if len(data) > 128*1024*1024:
        raise ValueError('authoring request exceeds 128 MiB')
    global _process
    with _lock:
        if _process is None:
            flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            _process = subprocess.Popen([str(executable), 'author-stream'], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, creationflags=flags)
        try:
            _process.stdin.write(struct.pack('<I', len(data)) + data)
            _process.stdin.flush()
            size = struct.unpack('<I', rust_codecs._read_exact(_process.stdout, 4))[0]
            if size > 128*1024*1024:
                raise RuntimeError('invalid authoring response length')
            response = json.loads(rust_codecs._read_exact(_process.stdout, size))
        except (OSError, RuntimeError):
            close()
            raise
    if not response['ok']:
        kind = {'ValueError': ValueError, 'KeyError': KeyError, 'OSError': OSError,
                'AssertionError': AssertionError, 'TimeoutError': TimeoutError,
                'InterruptedError': InterruptedError, 'AttributeError': AttributeError}.get(response['kind'], RuntimeError)
        if kind is AssertionError and not response['message']:
            raise AssertionError()
        raise kind(response['message'])
    return response['value']


def close():
    global _process
    with _lock:
        previous, _process = _process, None
        if previous is None:
            return
        previous.stdin.close()
        try:
            previous.wait(timeout=2)
        except subprocess.TimeoutExpired:
            previous.kill()
            previous.wait(timeout=2)
        previous.stdout.close()


atexit.register(close)
