# SPDX-License-Identifier: GPL-3.0-only
"""Synchronous file/OS adapter transport. Installer decisions live in Rust."""
import atexit
import json
import os
import struct
import subprocess
import threading
import rust_codecs

_process = None
_lock = threading.RLock()
_active = False


class ToolError(Exception):
    def __init__(self, kind, code, message):
        super().__init__(message)
        self.kind, self.code = kind, code


def _write(value, process=None):
    data = json.dumps(value, ensure_ascii=True).encode('utf-8')
    if len(data) > 32 * 1024 * 1024:
        raise ValueError('tool request exceeds 32 MiB')
    process = process or _process
    process.stdin.write(struct.pack('<I', len(data)) + data)
    process.stdin.flush()


def _offline(request):
    """A predicate may call pure API helpers while its transaction is waiting.
    Give that query an independent Rust stack; it cannot access session links.
    """
    executable = os.environ.get('FM1_TOOLING_CLI') or rust_codecs.executable()
    flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
    with subprocess.Popen([str(executable), 'tools-stream'], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, creationflags=flags) as process:
        try:
            _write(request, process)
            size = struct.unpack('<I', rust_codecs._read_exact(process.stdout, 4))[0]
            if size > 32 * 1024 * 1024:
                raise RuntimeError('invalid tooling response length')
            response = json.loads(rust_codecs._read_exact(process.stdout, size))
            if 'action' in response:
                raise RuntimeError('nested offline operation requested adapter I/O')
            if not response['ok']:
                raise ToolError(response['kind'], response['code'], response['message'])
            return response['value']
        finally:
            process.terminate()


def call(method, args=(), adapter=None, **values):
    global _process, _active
    with _lock:
        if _active:
            if adapter is None:
                return _offline(dict(method=method, args=list(args), **values))
            raise RuntimeError('tool adapter cannot re-enter a transaction')
        if _process is None:
            executable = os.environ.get('FM1_TOOLING_CLI') or rust_codecs.executable()
            flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
            _process = subprocess.Popen([str(executable), 'tools-stream'], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, creationflags=flags)
        _active = True
        pending = None
        try:
            _write(dict(method=method, args=list(args), **values))
            while True:
                size = struct.unpack('<I', rust_codecs._read_exact(_process.stdout, 4))[0]
                if size > 32 * 1024 * 1024:
                    raise RuntimeError('invalid tooling response length')
                response = json.loads(rust_codecs._read_exact(_process.stdout, size))
                if 'action' not in response:
                    if pending is not None:
                        raise pending
                    if not response['ok']:
                        raise ToolError(response['kind'], response['code'], response['message'])
                    return response['value']
                try:
                    if adapter is None:
                        raise RuntimeError('offline operation requested adapter I/O')
                    value = adapter(response['action'])
                    _write(dict(ok=True, value=value))
                except BaseException as error:
                    # Rust receives the failure and unwinds the transaction,
                    # preserving the original Python exception/API traceback.
                    if response['action']['kind'] != 'open' or not isinstance(error, Exception):
                        pending = error
                    _write(dict(ok=False, kind=type(error).__name__, code=getattr(error, 'code', ''), message=str(error)))
        except (OSError, RuntimeError):
            close()
            raise
        finally:
            _active = False


def close():
    global _process
    if _process is not None:
        _process.terminate()
        _process.wait()
        _process = None


atexit.register(close)
