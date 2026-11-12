#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare maintained authoring logic with the immutable R01 Python tools.

Usage: python tests/authoring_differential.py FROZEN_SOURCE FM1_RS [OUTPUT]
Requires mido only for the independent frozen oracle; the Rust tool needs none.
"""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import random
import subprocess
import struct
import atexit
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(sys.argv[1]).resolve()
CLI = Path(sys.argv[2]).resolve()
OUT = Path(sys.argv[3] if len(sys.argv) > 3 else ROOT / 'build/authoring-proof')
OUT.mkdir(parents=True, exist_ok=True)
for line in (ROOT/'tests/reference/source.sha256').read_text().splitlines():
    digest, name = line.split(maxsplit=1)
    assert hashlib.sha256((BASE/name.strip()).read_bytes()).hexdigest() == digest, name
sys.path.insert(0, str(BASE/'tools'))
import fm1_jam as old
import songbook
import jam_preview
import gen_fm6_dy
import mido

LIMITS = json.loads((BASE/'build/params.json').read_text(encoding='utf-8')) if (BASE/'build/params.json').exists() else None
cases = []
hashes = {}
stream = subprocess.Popen([str(CLI),'author-stream'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
def finish():
    stream.stdin.close();stream.wait(timeout=10);stream.stdout.close()
atexit.register(finish)
def read_exact(n):
    out=bytearray()
    while len(out)<n:
        part=stream.stdout.read(n-len(out))
        assert part,'truncated Rust authoring response'
        out.extend(part)
    return bytes(out)


def call(method, **values):
    request = dict(method=method, limits=LIMITS, **values)
    data=json.dumps(request).encode()
    stream.stdin.write(struct.pack('<I',len(data))+data);stream.stdin.flush()
    return json.loads(read_exact(struct.unpack('<I',read_exact(4))[0]))


def outcome(fn):
    try:
        value = fn()
        return dict(ok=True, value=list(value) if isinstance(value, bytes) else value)
    except Exception as error:
        return dict(ok=False, kind=type(error).__name__, message=error.args[0] if isinstance(error, KeyError) else str(error))


def compare(case_name, method, reference, **values):
    expected = outcome(reference)
    actual = call(method, **values)
    if actual != expected:
        (OUT/'failure.json').write_text(json.dumps(dict(name=case_name, request=values, expected=expected, actual=actual), indent=2), encoding='utf-8')
        raise AssertionError(case_name + ': ' + str(expected)[:220] + ' != ' + str(actual)[:220])
    cases.append(dict(name=case_name, accepted=actual['ok']))
    return actual.get('value')


docs = [old.factory()] + [songbook.factory(i) for i in range(3)]
for index, d in enumerate(docs):
    compare('factory-'+str(index), 'factory' if index == 0 else 'song', lambda d=d: d,
            **({} if index == 0 else dict(song=index-1)))
    b = old.compile_pack(d)
    compare('compile-'+str(index), 'compile', lambda d=d: old.compile_pack(d), document=d)
    compare('decode-'+str(index), 'decode', lambda b=b: old.decompile(b), bytes=list(b))
    assert list(call('decode',bytes=list(b))['value'])==list(old.decompile(b)),('decoded object key order',index)
    name = ['harris-citypop','velvet-changes','afterhours','neon-turnaround'][index]
    assert b == (BASE/'packs'/f'{name}.jmp').read_bytes()
    hashes[name] = hashlib.sha256(b).hexdigest()
compare('programs', 'programs', songbook.programs)
for value in [2.0]:
    d=copy.deepcopy(docs[1]);d['version']=value
    compare('numeric-v2-version','compile',lambda d=d:old.compile_pack(d),document=d)
for cadence in [[],docs[1]['cadence'][:1]]:
    d=copy.deepcopy(docs[1]);d['cadence']=cadence
    compare('legacy-cadence-'+str(len(cadence)),'compile',lambda d=d:old.compile_pack(d),document=d)
for sources in [dict(credit='opaque legacy metadata'),None,'legacy metadata']:
    d=copy.deepcopy(docs[1]);d['sources']=sources
    compare('legacy-metadata-'+str(sources),'compile',lambda d=d:old.compile_pack(d),document=d)
for sources in [dict(z='first',a='second'),dict(z=dict(c='first',a='second'),a=['last'])]:
    d=copy.deepcopy(docs[1]);d['sources']=sources
    b=compare('ordered-opaque-metadata-'+str(sources),'compile',lambda d=d:old.compile_pack(d),document=d)
    compare('ordered-opaque-metadata-roundtrip-'+str(sources),'decode',lambda b=b:old.decompile(bytes(b)),bytes=b)

rng = random.Random(91)
for index in range(160):
    d = copy.deepcopy(docs[index % 4])
    d['name'] = 'Author '+str(index)
    d['sources'] = ['日本語 – café', 'line\nquote " slash \\', str(index)]
    d['forms'][rng.randrange(4)]['roles'] = [rng.randrange(4) for _ in range(rng.randrange(1,9))]
    for bass in d['bass']:
        bass['notes'] = [rng.choice(list(range(-12,13))+[-128,-127,-126,-125]) for _ in range(16)]
        bass['velocity'] = [rng.randrange(128) for _ in range(16)]
    for drum in d['drums']:
        drum['lanes'] = [rng.randrange(65536) for _ in range(6)]
        drum['comp_gate'] = rng.randrange(1,101)
    if index % 4:
        for key in ['keys_gate','bass_gate','touch','brightness','attack','variation']:
            d['parts'].pop(key, None)
    b = compare('authored-'+str(index), 'compile', lambda d=d: old.compile_pack(d), document=d)
    compare('authored-roundtrip-'+str(index), 'decode', lambda b=b: old.decompile(bytes(b)), bytes=b)

mutations = [
    lambda d: d.__setitem__('version', 3), lambda d: d.__setitem__('name',''),
    lambda d: d.__setitem__('name','A'*24), lambda d: d['scales'][0].__setitem__(1,0),
    lambda d: d['cadence'][0].__setitem__(0,100), lambda d: d['cadence'][0].__setitem__(0,8),
    lambda d: d['nodes'][0].__setitem__('bass',12), lambda d: d['nodes'][0].__setitem__('mode',2),
    lambda d: d['forms'][0].__setitem__('roles',[]), lambda d: d['forms'][0]['roles'].__setitem__(0,4),
    lambda d: d['bass'][0]['notes'].__setitem__(0,-120), lambda d: d['bass'][0]['velocity'].__setitem__(0,128),
    lambda d: d['bass'][0].__setitem__('gate',0), lambda d: d['drums'][0]['lanes'].pop(),
    lambda d: d['drums'][0].__setitem__('comp',65536), lambda d: d['drums'][0].__setitem__('comp_gate',0),
    lambda d: d['sounds'][0].__setitem__('parameter',5), lambda d: d['sounds'][0].__setitem__('value',32768),
    lambda d: d.__setitem__('sources',[42]), lambda d: d.__setitem__('sources',['a'*4000]),
    lambda d: d.pop('name'), lambda d: d['bass'][0]['notes'].__setitem__(0,True),
]
for ix,d0 in enumerate(docs):
    for n,mutate in enumerate(mutations):
        d=copy.deepcopy(d0);mutate(d)
        compare('invalid-doc-'+str((ix,n)),'compile',lambda d=d:old.compile_pack(d),document=d)
    if ix:
        extra=[lambda d:d['charts'][0][0][0].__setitem__(0,16),lambda d:d['voice_programs'][0]['data'].__setitem__(0,128),
               lambda d:d['parts'].__setitem__('keys',8),lambda d:d['parts'].__setitem__('keys_gate',0),
               lambda d:d['parts'].__setitem__('touch',101),lambda d:d['charts'][0][0][0].pop()]
        for n,mutate in enumerate(extra):
            d=copy.deepcopy(d0);mutate(d)
            compare('invalid-song-'+str((ix,n)),'compile',lambda d=d:old.compile_pack(d),document=d)

# Binary input deliberately canonicalizes defaults/reserved bytes, not runtime padding.
for ix,d in enumerate(docs):
    original=old.compile_pack(d)
    for n in range(96):
        b=bytearray(original);at=rng.randrange(len(b));b[at]^=rng.choice([1,64,128,255])
        # Invalid UTF-8/JSON errors carry interpreter-specific positions; the
        # format rejection itself is checked separately from musical messages.
        if at >= (1120 if ix == 0 else 2560): continue
        expected=outcome(lambda b=b:old.decompile(b));actual=call('decode',bytes=list(b))
        if expected.get('kind') in ('UnicodeDecodeError','JSONDecodeError'):
            assert not actual['ok']
            cases.append(dict(name='binary-invalid-text-'+str((ix,n)),accepted=False))
        else:
            compare('binary-'+str((ix,n)),'decode',lambda b=b:old.decompile(b),bytes=list(b))

with tempfile.TemporaryDirectory() as td:
    td=Path(td)
    for i,d in enumerate(docs): (td/f'base{i}.json').write_text(json.dumps(d),encoding='utf-8')
    for mode in range(3):
        mf=mido.MidiFile(type=mode,ticks_per_beat=480)
        for track in range(1 if mode == 0 else 3):
            t=mido.MidiTrack();mf.tracks.append(t)
            t.append(mido.MetaMessage('track_name',name='Bäss '+str(track)))
            t.append(mido.MetaMessage('time_signature',numerator=4,denominator=4))
            # Quantization ties, velocity-zero note-ons, channel/program data,
            # end-exclusive bar selection, and repeated same-pitch events.
            t.extend([mido.Message('program_change',program=2,channel=track),
                      mido.Message('note_on',note=36+track,velocity=90,channel=track,time=60),
                      mido.Message('note_off',note=36+track,channel=track,time=120),
                      mido.Message('note_on',note=36+track,velocity=0,channel=track,time=60),
                      mido.Message('note_on',note=43+track,velocity=70,channel=track,time=240),
                      mido.Message('note_off',note=43+track,channel=track,time=240),
                      mido.Message('note_on',note=36+track,velocity=80,channel=track,time=1200)])
        p=td/f'type{mode}.mid';mf.save(p);raw=p.read_bytes()
        compare('inspect-'+str(mode),'inspect-midi',lambda p=p:jam_preview.inspect_midi(p),bytes=list(raw))
        if mode==0:
            for sources in [None,'legacy metadata',dict(credit='legacy')]:
                unusual=copy.deepcopy(docs[1]);unusual['sources']=sources
                unusual_path=td/'unusual.json';unusual_path.write_text(json.dumps(unusual),encoding='utf-8')
                compare('metadata-import-'+str(sources),'import-midi',lambda:jam_preview.import_midi(p,unusual_path,0,0,1,'bass'),bytes=list(raw),document=unusual,track=0,tonic=0,bar=1,role='bass',minor=False,name=p.name)
        for track in [-1,0,1,2,3]:
            for bar in [1,2,3]:
                for ix in [0,1]:
                    values=dict(bytes=list(raw),document=docs[ix],track=track,tonic=2,bar=bar,role='bass',minor=False,name=p.name)
                    compare('midi-import-'+str((mode,track,bar,ix)),'import-midi',lambda p=p,ix=ix,track=track,bar=bar:jam_preview.import_midi(p,td/f'base{ix}.json',track,2,bar,'bass'),**values)
    # Exact authored harmony grouping, root tie order and minor-mode selection.
    for ticks in [-6360,-1,0]:
        mf=mido.MidiFile(ticks_per_beat=ticks);t=mido.MidiTrack();mf.tracks.append(t);t.append(mido.Message('note_on',note=36,velocity=90))
        p=td/'division.mid';mf.save(p);raw=p.read_bytes()
        compare('signed-division-inspect-'+str(ticks),'inspect-midi',lambda:jam_preview.inspect_midi(p),bytes=list(raw))
        compare('signed-division-import-'+str(ticks),'import-midi',lambda:jam_preview.import_midi(p,td/'base0.json',0,0,1,'bass'),bytes=list(raw),document=docs[0],track=0,tonic=0,bar=1,role='bass',minor=False,name=p.name)
    mf=mido.MidiFile();t=mido.MidiTrack();mf.tracks.append(t)
    for root in [1,6]:
        for i,n in enumerate([root,root+4,root+7,root+9]):t.append(mido.Message('note_on',note=48+n,velocity=90,time=0 if i else 480))
    p=td/'harmony.mid';mf.save(p)
    for minor in [False,True]:
        for ix in [0,1]:compare('harmony-'+str((ix,minor)),'import-midi',lambda ix=ix,minor=minor:jam_preview.import_midi(p,td/f'base{ix}.json',0,0,1,'harmony',minor),bytes=list(p.read_bytes()),document=docs[ix],track=0,tonic=0,bar=1,role='harmony',minor=minor,name=p.name)
    # Log sink has exact channel insertion order, running status and tempo rounding.
    log='ignored\n0 ms: F8 00 00\n1 ms: 92 24 64\n2 ms: 92 25 64\n3 ms: C0 02 00\n4 ms: D0 20 00\n5 ms: 99 24 7F\n6 ms: 82 24 00\ninvalid\n'
    (td/'log.txt').write_text(log)
    for bpm in [70,91,100,127,190]:
        target=td/'out.mid';jam_preview.midi_from_log(td/'log.txt',target,bpm)
        actual=compare('midi-log-'+str(bpm),'midi-log',lambda:target.read_bytes(),log=log,bpm=bpm)
        hashes['midi-log-'+str(bpm)]=hashlib.sha256(bytes(actual)).hexdigest()
    # Bulk selection scans headers, preserves permissive file checksums, skips
    # nonbulk prefixes and indexes across concatenated banks.
    voice=bytes(docs[1]['voice_programs'][0]['data'])
    body=b''.join(bytes([(i+j)%128 for j in range(118)])+bytes(('Voice %04d'%i).encode()) for i in range(64))
    bank=b'junk\xf0\x43\x00\x00\x00\x00'+b'\xf0\x43\x00\x09\x20\x00'+body[:4096]+b'\x01\x00'+b'\xf0\x43\x00\x09\x20\x00'+body[4096:]+b'\x00\xf7'
    (td/'voices.syx').write_bytes(bank)
    for index in [-1,0,3,31,32,63,64]:
        def selected(index=index):
            voices=[v for bank in gen_fm6_dy.dumps(bank) for v in bank]
            if not 0<=index<len(voices):raise ValueError('voice index outside bank')
            d=old.decompile(old.compile_pack(docs[1]));d['parts']['bass']=len(d['voice_programs'])
            d['voice_programs'].append(dict(name=gen_fm6_dy.name_of(voices[index]),data=list(voices[index])))
            d['sources'].append('User-selected DX voice: voices.syx, voice '+str(index));old.compile_pack(d);return d
        compare('voice-selection-'+str(index),'import-voice',selected,bytes=list(bank),document=docs[1],voice=index,role='bass',name='voices.syx')

report=dict(cases=len(cases),accepted=sum(c['accepted'] for c in cases),rejected=sum(not c['accepted'] for c in cases),hashes=hashes,source_files=440,results=cases)
(OUT/'authoring.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:v for k,v in report.items() if k!='results'}))
