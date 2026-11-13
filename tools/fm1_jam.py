#!/usr/bin/env python3
"""Jam Pack compiler and USB editor for NAIWAN 91."""
import argparse
import json
import pathlib
import sys
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
MAX = 3840
NODE, EDGE, FORM, BASS, DRUM, SOUND, META = 64, 192, 448, 576, 864, 992, 1120
REST, APPROACH, THIRD, FIFTH = -128, -127, -126, -125

from rust_authoring import call as _author

def factory():
    return _author("factory")

def integer(v, lo, hi, name):
    return _author("integer", value=v, low=lo, high=hi, name=name)

def compile_pack(d):
    return bytes(_author("compile", document=d))

def decompile(b):
    return _author("decode", bytes=list(b))

def load(path):
    p=pathlib.Path(path)
    return compile_pack(json.loads(p.read_text(encoding='utf-8'))) if p.suffix=='.json' else compile_pack(decompile(p.read_bytes()))

def write_json(path,d): pathlib.Path(path).write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--port');sub=ap.add_subparsers(dest='cmd',required=True)
    p=sub.add_parser('factory');p.add_argument('output')
    p=sub.add_parser('factory-song');p.add_argument('output');p.add_argument('--song',type=int,choices=range(3),default=0)
    p=sub.add_parser('compile');p.add_argument('input');p.add_argument('output');p.add_argument('--c-header')
    p=sub.add_parser('validate');p.add_argument('input')
    p=sub.add_parser('decode');p.add_argument('input');p.add_argument('output')
    p=sub.add_parser('preview');p.add_argument('input');p.add_argument('output');p.add_argument('--bars',type=int,choices=[8,16],default=16);p.add_argument('--bpm',type=int,default=100);p.add_argument('--key',type=int,choices=range(12),default=0);p.add_argument('--minor',action='store_true');p.add_argument('--phrases',type=int,default=2)
    p=sub.add_parser('import-voice');p.add_argument('input');p.add_argument('base');p.add_argument('output');p.add_argument('--voice',type=int,default=0);p.add_argument('--role',choices=['keys','bass'],required=True)
    p=sub.add_parser('inspect-midi');p.add_argument('input')
    p=sub.add_parser('import-midi');p.add_argument('input');p.add_argument('base');p.add_argument('output');p.add_argument('--track',type=int,required=True);p.add_argument('--tonic',type=int,choices=range(12),required=True);p.add_argument('--bar',type=int,default=1);p.add_argument('--role',choices=['bass','harmony'],required=True);p.add_argument('--minor',action='store_true')
    for cmd in ('ports','capabilities','features','song-status','status','abort'):sub.add_parser(cmd)
    p=sub.add_parser('pull');p.add_argument('output');p.add_argument('--slot',type=int,choices=range(5),default=0)
    p=sub.add_parser('diff');p.add_argument('input');p.add_argument('--against')
    for cmd in ('audition','rollback'):
        p=sub.add_parser(cmd);p.add_argument('input');p.add_argument('--backup-dir',default=str(ROOT/'build/jam-backups'));p.add_argument('--no-wait',action='store_true')
    p=sub.add_parser('save');p.add_argument('slot',type=int,choices=range(1,5))
    p=sub.add_parser('control');p.add_argument('field',choices=['on','key','bars','form','movement','energy','hold','bass','idea','play','song','swell','variation','keys_program','bass_program','touch','release','bass_gate','swell_voice','bass_attack','brightness','keys_octave','bass_octave','synth','space']);p.add_argument('value',type=int)
    a=ap.parse_args()
    if a.cmd=='factory-song':
        from songbook import factory as factory_song
        write_json(a.output,factory_song(a.song));return
    if a.cmd=='factory':write_json(a.output,factory());return
    if a.cmd=='preview':
        from jam_preview import preview
        if not 70<=a.bpm<=190 or not 1<=a.phrases<=16:raise ValueError('BPM 70..190; phrases 1..16')
        print(json.dumps(preview(ROOT,a.input,a.output,a.bars,a.bpm,a.key,a.minor,a.phrases)));return
    if a.cmd=='import-voice':
        d=_author('import-voice', bytes=list(pathlib.Path(a.input).read_bytes()),
                  document=decompile(load(a.base)), voice=a.voice, role=a.role,
                  name=pathlib.Path(a.input).name)
        write_json(a.output,d);print(a.output);return
    if a.cmd=='inspect-midi':
        from jam_preview import inspect_midi
        print(json.dumps(inspect_midi(a.input),indent=2));return
    if a.cmd=='import-midi':
        from jam_preview import import_midi
        if a.bar<1:raise ValueError('bar is one-based')
        d=import_midi(a.input,a.base,a.track,a.tonic,a.bar,a.role,a.minor);compile_pack(d);write_json(a.output,d);print(a.output);return
    if a.cmd=='ports':
        from jam_midi import ports
        print(json.dumps(ports(),indent=2));return
    if a.cmd=='diff':
        import difflib
        left=decompile(load(a.input))
        if a.against:right=decompile(load(a.against))
        else:
            from jam_midi import Device
            with Device(a.port) as dev:right=decompile(dev.read())
        print(''.join(difflib.unified_diff(json.dumps(left,indent=2).splitlines(True),json.dumps(right,indent=2).splitlines(True),fromfile=a.input,tofile=a.against or 'device')));return
    if a.cmd in ('capabilities','features','song-status','status','abort','pull','audition','rollback','save','control'):
        from jam_midi import Device
        with Device(a.port) as dev:
            if a.cmd=='capabilities':result=dev.capabilities()
            elif a.cmd=='features':result=dev.features()
            elif a.cmd=='song-status':result=dev.song_status()
            elif a.cmd=='status':result=dev.status()
            elif a.cmd=='abort':dev.request(9);result={'aborted':True}
            elif a.cmd=='pull':
                b=dev.read(a.slot)
                _author('device-pull-check', bytes=list(b))
                if pathlib.Path(a.output).suffix=='.json':write_json(a.output,decompile(b))
                else:pathlib.Path(a.output).write_bytes(b)
                result={'output':a.output,'crc32':f'{zlib.crc32(b):08x}'}
            elif a.cmd in ('audition','rollback'):
                b=load(a.input);dev.capabilities();
                if _author('device-needs-features', bytes=list(b)):
                    _author('device-compatible', features=dev.features(), version=b[4])
                previous=dev.read();folder=pathlib.Path(a.backup_dir);folder.mkdir(parents=True,exist_ok=True)
                backup=folder/(time.strftime('%Y%m%d-%H%M%S')+f'-{time.time_ns()%1000000:06d}.jmp');backup.write_bytes(previous)
                print(json.dumps({'rollback':str(backup)}),flush=True);dev.audition(b)
                result={'queued':True} if a.no_wait else dev.wait_active(b)
            elif a.cmd=='save':result=dev.save(a.slot)
            else:
                _author('device-control-check', field=a.field, value=a.value)
                result=dev.control(a.field,a.value)
            print(json.dumps(result,indent=2));return
    b=load(a.input)
    if a.cmd=='compile':
        pathlib.Path(a.output).write_bytes(b)
        if a.c_header:pathlib.Path(a.c_header).write_text(_author('c-header', bytes=list(b), name='JP_FACTORY',
            prefix='/* Generated by tools/fm1_jam.py; GPL-3.0-only */\n'),encoding='utf-8')
    elif a.cmd=='decode':write_json(a.output,decompile(b))
    print(json.dumps(dict(bytes=len(b),crc32=f'{zlib.crc32(b):08x}',name=decompile(b)['name'])))

if __name__=='__main__':
    try:main()
    except (ValueError,KeyError,OSError,RuntimeError) as e:sys.exit(str(e))
