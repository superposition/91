#!/usr/bin/env python3
"""Exercise the shipped Python updater against the real C app/protocol handler.

The MIDI port adapter is replaced by a subprocess pipe. Physical USB packet
delivery remains a separate device check. No mido/rtmidi dependency is needed.
"""
import os
import pathlib
import subprocess
import sys
import types
import unittest
from unittest.mock import patch

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import fm1_jam
from jam_midi import Device


class Wire:
    def __init__(self,exe):
        env=os.environ.copy();env.pop('AMB_LEGACY_BOOT',None)
        self.proc=subprocess.Popen([str(exe)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
                                   text=True,bufsize=1,env=env)
        self.pending=[]

    def exchange(self,line):
        self.proc.stdin.write(line+'\n');self.proc.stdin.flush()
        result=self.proc.stdout.readline().strip()
        if not result:raise RuntimeError('firmware bridge exited')
        return result

    def send(self,msg):
        raw=bytes.fromhex(self.exchange(bytes(msg.data).hex()))
        if raw[0]!=0xf0 or raw[-1]!=0xf7:raise ValueError('bad SysEx envelope')
        self.pending.append(types.SimpleNamespace(type='sysex',data=raw[1:-1]))

    def iter_pending(self):
        result=self.pending;self.pending=[];return iter(result)

    def close(self):pass  # A MIDI port close does not power off the instrument.

    def finish(self):
        self.proc.stdin.close();self.proc.wait(timeout=10);self.proc.stdout.close()
        if self.proc.returncode:raise RuntimeError('firmware bridge failed')


class TransportIntegration(unittest.TestCase):
    def test_factory_selection_profiles_and_saved_source(self):
        wire=Wire(ROOT/'build/host/jam_transport_bridge')
        midi=types.SimpleNamespace(get_input_names=lambda:['FM-1 input'],get_output_names=lambda:['FM-1 output'],
            open_input=lambda name:wire,open_output=lambda name:wire,
            Message=lambda kind,**kw:types.SimpleNamespace(type=kind,**kw))
        try:
            with patch.dict(sys.modules,{'mido':midi}),patch('jam_midi.time.sleep'):
                with Device() as dev:
                    dev.control('key',18);dev.control('bars',8)
                    for song,stem in enumerate(('velvet-changes','afterhours','neon-turnaround')):
                        dev.control('song',song)
                        active=dev.wait_active((ROOT/'packs'/f'{stem}.jmp').read_bytes())
                        self.assertEqual((active['key'],active['minor'],active['bars']),(6,1,8))
                    dev.control('touch',23);dev.control('swell',45)
                    edited=dev.read();self.assertEqual(fm1_jam.decompile(edited)['parts']['touch'],23)
                    dev.control('play',0);dev.save(2)
                    self.assertEqual(wire.exchange('reboot'),'ok')
                    self.assertEqual(dev.read(),edited)
                    self.assertEqual(dev.song_status()['song'],4)
                    self.assertEqual(dev.status()['slot'],2)
        finally:wire.finish()

    def test_live_update_rollback_and_standalone_save(self):
        wire=Wire(ROOT/'build/host/jam_transport_bridge')
        midi=types.SimpleNamespace(
            get_input_names=lambda:['FM-1 simulated input'],
            get_output_names=lambda:['FM-1 simulated output'],
            open_input=lambda name:wire,open_output=lambda name:wire,
            Message=lambda kind,**kw:types.SimpleNamespace(type=kind,**kw))
        try:
            with patch.dict(sys.modules,{'mido':midi}),patch('jam_midi.time.sleep'):
                with Device() as dev:
                    self.assertEqual(dev.capabilities()['max_bytes'],3840)
                    original=dev.read()
                    self.assertEqual(original,(ROOT/'packs/velvet-changes.jmp').read_bytes())
                    self.assertEqual(dev.features()['pack_versions'],[1,2])
                    self.assertEqual(dev.song_status()['pack_version'],2)
                    self.assertEqual(dev.request(13,[1]),b'\x01')
                    self.assertEqual(dev.request(13,[0]),b'\x00')
                    self.assertEqual(dev.control('hold',1)['hold'],1)
                    data=fm1_jam.decompile(original);data['name']='Transport audition'
                    edited=fm1_jam.compile_pack(data)
                    before=dev.status();dev.audition(edited);queued=dev.status()
                    self.assertEqual(queued['pending'],1)
                    self.assertEqual(queued['crc32'],before['crc32'])
                    active=dev.wait_active(edited)
                    self.assertEqual(active['playing'],1)
                    self.assertEqual(active['bar'],0)
                    self.assertEqual(active['hold'],1)
                    self.assertEqual(dev.read(),edited)
                    # Old packs still use their original engine and survive readback.
                    legacy=(ROOT/'packs/harris-citypop.jmp').read_bytes()
                    dev.audition(legacy);dev.wait_active(legacy)
                    self.assertEqual(dev.song_status()['pack_version'],1)
                    self.assertEqual(dev.read(),legacy)
                    dev.audition(edited);dev.wait_active(edited)
                    with self.assertRaisesRegex(RuntimeError,'stop playback'):
                        dev.save(4)
                    dev.control('play',0)
                    dev.control('key',21);dev.control('bars',8);dev.control('form',2)
                    dev.save(4)
                    self.assertEqual(dev.read(4),edited)
                    self.assertEqual(wire.exchange('reboot'),'ok')
                    saved=dev.status()
                    self.assertEqual((saved['key'],saved['minor'],saved['bars'],saved['form'],saved['slot']),
                                     (9,1,8,2,4))
                    self.assertEqual(dev.read(),edited)
                    # RAM rollback changes playback, while the saved slot remains.
                    dev.audition(original);dev.wait_active(original)
                    self.assertEqual(dev.read(),original)
                    self.assertEqual(dev.read(4),edited)
                    self.assertEqual(wire.exchange('reboot'),'ok')
                    self.assertEqual(dev.read(),edited)
        finally:wire.finish()


if __name__=='__main__':unittest.main()
