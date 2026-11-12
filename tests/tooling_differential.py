#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen installer/telemetry oracle. No MIDI imports, ports, or real sleeps."""
import contextlib
import csv
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import random
import struct
import subprocess
import sys
import tempfile
import textwrap

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(sys.argv[1]).resolve()
CLI = str(Path(sys.argv[2]).resolve())
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest,name=line.split(maxsplit=1)
    assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest()==digest,name
os.environ.update(FM1_TOOLING_CLI=CLI, FM1_CODEC_CLI=CLI)
sys.path.insert(0, str(ROOT / 'tools'))
import fm1_install as current
import fm1log as log
import rust_tooling as tools


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


old = load('frozen_install', BASE / 'tools/fm1_install.py')
oldlog = load('frozen_log', BASE / 'tools/fm1log.py')
cases = []
traces = []


def result(fn):
    try:
        value = fn()
        if isinstance(value, current.Identity) or isinstance(value, old.Identity):
            value = (value.text, value.model, value.version, value.loader)
        return ('ok', value)
    except BaseException as error:
        return ('error', type(error).__name__, getattr(error, 'code', None), str(error))


def check(name, expected, actual):
    assert expected == actual, (name, expected, actual)
    cases.append(name)


ports = ['FM-1', 'FM-1 0', 'FM-1 1', 'Felucca Update 2', 'ota-FM-1', 'composite',
         'SINCO', 'USB-MIDI', 'Other gear', 'fm1', 'PERF', 'nothing 123', 'FM-1 3 4']
rng = random.Random(409)
for i in range(180):
    names = rng.sample(ports, rng.randrange(len(ports) + 1))
    name = rng.choice(ports)
    check(f'pair-{i}', result(lambda: old.pair_output(name, names)), result(lambda: current.pair_output(name, names)))
    backend = type('Ports', (), {'input_names': lambda self: names})()
    for port in [None, '', 'FM-1', 'other', '2']:
        for only in [False, True]:
            check(f'candidates-{i}-{port}-{only}', old.Updater(backend, port).candidates(only), current.Updater(backend, port).candidates(only))

for i, name in enumerate(['FM-1_015', 'FM-1_700', 'FM-1_7000304', 'ota-FM-1_900', 'OTA-FM-1_900', 'bad', '_3', 'FM-1_x', 'FM-1_000', 'A_999999999999999999999999', 'FM-1_1_2', '\u00e9_123']):
    raw = bytes([0, 0x59, 0x11, 0, 0, 0]) + name.encode('latin1') + bytes(28 - len(name))
    packet = b'\xf0' + old.pack7(raw) + b'\xf7'
    check(f'identity-{i}', result(lambda: old.parse_identity(packet)), result(lambda: current.parse_identity(packet)))
    for framing in [b'\0'+packet[1:],packet[:-1]+b'\0']:
        check(f'identity-framing-{i}',result(lambda:old.parse_identity(framing)),result(lambda:current.parse_identity(framing)))
    check(f'model-{i}', old.model_of(name), current.model_of(name))
for i in range(130):
    packet = bytes(rng.randrange(256) for _ in range(rng.randrange(100)))
    check(f'identity-noise-{i}', result(lambda: old.parse_identity(packet)), result(lambda: current.parse_identity(packet)))


class Clock:
    def __init__(self):
        self.now = 0.0
    def monotonic(self):
        return self.now
    def sleep(self, n):
        self.now += n


class Link:
    def __init__(self, backend):
        self.b = backend
        self.generation = backend.generation
        self.name = backend.name
        self.queue = []
    @property
    def lost(self):
        return not self.b.connected or self.generation != self.b.generation
    def send(self, packet):
        self.b.trace.append(['send', self.name, bytes(packet).hex()])
        if self.lost or self.b.mode == 'send-fail':
            return False
        self.b.receive(self, bytes(packet))
        return True
    def read(self, timeout):
        self.b.trace.append(['read', self.name, round(timeout, 10)])
        if self.queue:
            return self.queue.pop(0)
        self.b.clock.sleep(timeout)
        return None
    def close(self):
        self.b.trace.append(['close', self.name])
    def drain(self):
        self.b.trace.append(['drain', self.name])
        self.queue.clear()


class Fake:
    def __init__(self, module, clock, mode, identity):
        self.m, self.clock, self.mode = module, clock, mode
        self.trace, self.generation = [], 0
        self.connected = True
        self.boot(identity)
    def boot(self, identity):
        self.identity = identity
        self.name = 'Felucca Update' if identity.startswith('ota-') else 'FM-1 0'
        self.generation += 1
    def input_names(self):
        names = ['Other synth', self.name] if self.connected else ['Other synth']
        self.trace.append(['inputs', names])
        return names
    def output_names(self):
        if self.mode=='empty-output':
            self.trace.append(['outputs',['']])
            return ['']
        names = ['Other synth out', self.name] if self.connected else ['Other synth out']
        self.trace.append(['outputs', names])
        return names
    def open(self, name, output):
        self.trace.append(['open', name, output])
        if self.mode == 'open-fail':
            raise OSError('occupied')
        return Link(self)
    def ident(self, link, identity):
        raw = bytes([0, 0x59, 0x11, 0, 0, 0]) + identity.encode() + bytes(28 - len(identity))
        link.queue.append(b'\xf0' + self.m.pack7(raw) + b'\xf7')
    def req(self, link):
        if self.pending:
            addr, count = self.pending.pop(0)
            self.waiting = addr
            raw = bytes([0, 0x59, 0x30, 0, 0, 0, 0]) + struct.pack('<IH', addr, count) + b'\0'
            raw += bytes([~sum(raw[6:14]) & 255])
            link.queue.append(b'\xf0' + self.m.pack7(raw) + b'\xf7')
    def receive(self, link, packet):
        if packet == self.m.HS_QUERY:
            if self.mode != 'silent':
                self.ident(link, self.identity[:8] if self.identity.startswith('FM-1_7') else self.identity)
        elif self.m.HS_FULL is not None and packet == self.m.HS_FULL:
            if self.mode != 'short-only':
                self.ident(link, self.identity if self.mode != 'full-wrong-model' else 'WRONG_7000304')
        elif packet == self.m.UPGRADE:
            loader = self.identity.startswith('ota-')
            finish = self.m.FINISH_WRITE if loader else self.m.FINISH_CHECK
            self.pending = [(0, 0), (13, 17), (512, 512), (finish, 8)]
            if self.mode == 'bad-length':
                self.pending.insert(0, (1, 513))
            if self.mode == 'bad-address':
                self.pending.insert(0, (0xffffffff, 2))
            if self.mode == 'wrong-finish':
                self.pending.insert(0, (self.m.FINISH_CHECK if loader else self.m.FINISH_WRITE, 8))
            if self.mode == 'noise':
                link.queue += [b'junk', b'\xf0\xf7']
            self.req(link)
        elif self.waiting >= self.m.FINISH_CHECK:
            if self.waiting == self.m.FINISH_CHECK and not self.identity.startswith('ota-'):
                self.boot('ota-FM-1_900')
                if self.mode=='no-loader':
                    self.connected=False
            elif self.waiting == self.m.FINISH_WRITE and self.identity.startswith('ota-'):
                self.boot('FM-1_999' if self.mode == 'mismatch' else 'FM-1_900')
                if self.mode == 'no-return':
                    self.connected = False
            else:
                self.req(link)
        elif self.mode == 'lost':
            self.connected = False
        elif self.mode != 'stall':
            self.req(link)


for mode in ['normal', 'short-only', 'full-wrong-model', 'bad-length', 'bad-address', 'wrong-finish', 'noise', 'lost', 'stall', 'mismatch', 'no-return', 'no-loader', 'send-fail', 'silent', 'open-fail','empty-output']:
    for identity in ['FM-1_015', 'FM-1_7000303', 'ota-FM-1_900', 'OTHER_015']:
        outcomes = []
        for module in (old, current):
            clock = Clock()
            module.time = clock
            module.HS_FULL = None
            module.DELAY.update(open=0, start=.02, reply=.01, loader=.03, reboot=.03, retry=.1, poll=.05, hs=.1,
                                idle_check=.2, idle_write=.2, wait_loader=.5, wait_reboot=.5)
            fake = Fake(module, clock, mode, identity)
            up = module.Updater(fake)
            step = lambda *a: fake.trace.append(['step', *a])
            def run():
                dev = up.find()
                return None if dev is None else up.install(dev, bytes(range(256)) * 4, 'FM-1_900', step)
            outcome = result(run)
            if mode=='normal' and identity!='OTHER_015':
                assert outcome==('ok','FM-1_900'),(mode,identity,outcome)
            outcomes.append((outcome, fake.trace, round(clock.now, 10)))
        check(f'transaction-{mode}-{identity}', *outcomes)
        traces.append(dict(mode=mode,identity=identity,result=outcomes[0]))

outcomes=[]
for module in (old,current):
    clock=Clock();module.time=clock;fake=Fake(module,clock,'normal','ota-FM-1_900');up=module.Updater(fake)
    def empty_write():
        dev=up.find()
        return up.install(dev,b'','FM-1_900',lambda *a:None)
    outcomes.append(result(empty_write))
check('empty-image-zero-request',*outcomes)

for kind, function in [(70, 'decode'), (90, 'decode_extra'), (91, 'decode_diag'), (71, 'decode_bench')]:
    for count in range(45):
        words = [rng.getrandbits(32) for _ in range(count)]
        msg = [125, 80, kind] + oldlog.pack7(struct.pack('<%dI' % count, *words))
        check(f'decode-{kind}-{count}', result(lambda: getattr(oldlog, function)(msg)), result(lambda: getattr(log, function)(msg)))
    for i in range(30):
        msg = [rng.randrange(256) for _ in range(rng.randrange(100))]
        check(f'frame-noise-{kind}-{i}', result(lambda: getattr(oldlog, function)(msg)), result(lambda: getattr(log, function)(msg)))

for i in range(46):
    words = [1500, 24] + [rng.randrange(1, 1000000) for _ in range(14)]
    if i>=40:
        words[[0,1,9,0,2,6][i-40]]=[0,0,0,32,0,0][i-40]
    outputs = []
    for module in [oldlog, log]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            outcome=result(lambda:module.show_bench(words))
        outputs.append((outcome,output.getvalue()))
    check(f'bench-{i}', *outputs)

# Execute the exact frozen row/event/display statements with I/O-only sinks.
source=(BASE/'tools/fm1log.py').read_text()
start=source.index('                    ev = []')
end=source.index('\n                time.sleep(0.01)',start)
fragment=compile(textwrap.dedent(source[start:end]),'frozen-live-row','exec')
for i in range(100):
    d={k:rng.getrandbits(32) for k in oldlog.FIELDS}
    d.update(chorus=i%2,bass=(i+1)%2,flash=1,susp_bad=0,boot=1)
    prev=None if i%5==0 else {k:rng.getrandbits(32) for k in oldlog.FIELDS}
    extra={k:rng.getrandbits(32) for k in oldlog.EXTRA} if i%2 else {}
    diag={k:rng.getrandbits(32) for k in oldlog.DIAG} if i%3 else {}
    now,start_time=100.005+i*.25,3.125
    rows=[];live=[]
    cw=type('Writer',(),{'writerow':lambda self,r:rows.append(r)})()
    f=type('File',(),{'flush':lambda self:None})()
    values=dict(now=now,t0=start_time,d=d,prev=prev,extra=extra,diag=diag,cw=cw,f=f,
                FIELDS=oldlog.FIELDS,EXTRA=oldlog.EXTRA,DIAG=oldlog.DIAG,print=lambda x,**kw:live.append(x))
    exec(fragment,values)
    check(f'live-row-{i}',dict(row=rows[0],live=live[0]),tools.call('telemetry.row',[now,start_time,d,prev,extra,diag]))
for now,last in [(0,0),(3,0),(3.0000001,0),(4,1),(4.001,1),(10,15)]:
    check('keepalive-threshold',now-last>3.0,tools.call('telemetry.arm',[now,last]))

with tempfile.TemporaryDirectory(prefix='fm1-tooling-') as folder:
    folder = Path(folder)
    for product in ['FM-1_900','FM-1_7000304','OTHER_001','bad_identity_x']:
        for marker in [True,False]:
            raw=bytearray((i*7)&255 for i in range(8212))
            for i in range(20):
                raw[i*48+47]=(ord(product[i])+i+1)&255 if i<len(product) else 125
            raw[6144:6160]=old.LOADER_MARK if marker else bytes(16)
            path=folder/'package.fwsc';path.write_bytes(raw)
            for force in [False,True]:
                check(f'package-{product}-{marker}-{force}',result(lambda:old.load_package(str(path),force)),result(lambda:current.load_package(str(path),force)))
                if marker and product!='bad_identity_x':
                    completed=subprocess.run([CLI,'tools','package-info',str(path)],capture_output=True,check=True)
                    check('native-package-info',dict(product=product,logical_bytes=len(old.logical_image(raw))),json.loads(completed.stdout))
                    check('native-package-bad-flag',2,subprocess.run([CLI,'tools','package-info',str(path),'--unknown'],capture_output=True).returncode)
    for i in range(14):
        columns = ['t'] + oldlog.FIELDS + ['chorus', 'bass', 'flash', 'susp_bad', 'boot']
        if i % 4:
            columns += oldlog.EXTRA
        if i % 3:
            columns += oldlog.DIAG
        rows = []
        for j in range(15 if i else 0):
            row = {k: str(rng.randrange(1, 100000)) for k in columns}
            row.update(t=str(j / 4), genre=str(j % 3), piece=str(j % 4), chunks='500', sm_n='220', sm_frames='27000',
                       chorus=str(j % 2), bass=str(j % 3 == 0 and 1 or 0), pk_info=str(rng.randrange(1 << 19)))
            if i==13:
                row['chunks']='-500'
            rows.append({k: row[k] for k in columns})
        path = folder / f'log{i}.csv'
        with path.open('w', newline='') as stream:
            writer = csv.DictWriter(stream, columns)
            writer.writeheader()
            writer.writerows(rows)
        outputs = []
        for source in [BASE, ROOT]:
            env = dict(os.environ)
            outputs.append(subprocess.run([sys.executable, str(source / 'tools/fm1sum.py'), str(path)], capture_output=True, env=env))
        check(f'summary-{i}', (outputs[0].returncode, outputs[0].stdout, outputs[0].stderr),
              (outputs[1].returncode, outputs[1].stdout, outputs[1].stderr))
        if rows:
            native=subprocess.run([CLI,'tools','summary',str(path)],capture_output=True)
            check(f'summary-native-{i}',(0,outputs[0].stdout,b''),(native.returncode,native.stdout,native.stderr))

# A callback failure unwinds only its transaction; the next pure request works.
clock=Clock();current.time=clock
fake=Fake(current,clock,'normal','FM-1_015')
up=current.Updater(fake)
def interrupted():
    raise KeyboardInterrupt()
outcome=result(lambda:up.find(lambda ident:interrupted()))
check('callback-interrupt',('error','KeyboardInterrupt',None,''),outcome)
check('restart-after-interrupt', 'FM-1',current.model_of('ota-FM-1_900'))
fake=Fake(current,clock,'normal','FM-1_015');up=current.Updater(fake)
dev=up.find(lambda ident:current.model_of(ident.text)=='FM-1')
check('nested-pure-predicate',True,dev is not None)
for malformed in [dict(method='unknown'),dict(method='install.serve',args=[0,[],0,None])]:
    pid=tools._process.pid
    try:
        tools.call(malformed['method'],malformed.get('args',[]))
    except tools.ToolError:
        pass
    else:
        raise AssertionError('malformed request accepted')
    check('restart-after-malformed', 'FM-1',current.model_of('FM-1_900'))
    check('same-stream-after-malformed',pid,tools._process.pid)

for payload in [struct.pack('<I',32*1024*1024+1),struct.pack('<I',4)+b'{']:
    completed=subprocess.run([CLI,'tools-stream'],input=payload,capture_output=True)
    check('truncated-or-oversize-frame',2,completed.returncode)

digest = hashlib.sha256(json.dumps(cases, separators=(',', ':')).encode()).hexdigest()
print(json.dumps(dict(cases=len(cases), case_names_sha256=digest, transactions=len(traces), transaction_trace_sha256=hashlib.sha256(json.dumps(traces,separators=(',',':')).encode()).hexdigest(), ports_opened=0, real_sleeps=0), indent=2))
