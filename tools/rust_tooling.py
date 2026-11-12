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
    return isolated(request['method'], request.get('args', ()),
                    **{k: v for k, v in request.items() if k not in ('method', 'args')})


def _exchange(process, request, adapter):
    pending = None
    _write(request, process)
    while True:
        size = struct.unpack('<I', rust_codecs._read_exact(process.stdout, 4))[0]
        if size > 32 * 1024 * 1024:
            raise RuntimeError('invalid tooling response length')
        response = json.loads(rust_codecs._read_exact(process.stdout, size))
        if 'action' not in response:
            if pending is not None:
                raise pending
            if not response['ok']:
                raise ToolError(response['kind'], response['code'], response['message'])
            return response['value']
        action = response['action']
        try:
            if adapter is None:
                raise RuntimeError('offline operation requested adapter I/O')
            value = adapter(action)
            _write(dict(ok=True, value=value), process)
        except BaseException as error:
            classes = [cls.__name__ for cls in type(error).__mro__]
            handled = action.get('handled_errors', [])
            matched = next((name for name in handled if name in classes), None)
            consumes = isinstance(error, Exception) and (matched or '*' in handled or action['kind'] == 'open')
            if not consumes:
                pending = error
            kind = matched or type(error).__name__
            if not isinstance(error, Exception):
                kind = 'BaseException'
            _write(dict(ok=False, kind=kind,
                        code=getattr(error, 'code', ''), message=str(error)), process)


def isolated(method, args=(), adapter=None, **values):
    """Independent Rust stack for a pure predicate or a scoped keepalive job.
    It cannot alias a foreground Rust buffer or replace the cached stream.
    """
    executable = os.environ.get('FM1_TOOLING_CLI') or rust_codecs.executable()
    flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
    with subprocess.Popen([str(executable), 'tools-stream'], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, creationflags=flags) as process:
        try:
            return _exchange(process, dict(method=method, args=list(args), **values), adapter)
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
        try:
            return _exchange(_process, dict(method=method, args=list(args), **values), adapter)
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
