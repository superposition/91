#!/usr/bin/env python3
import copy
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import fm1_jam as pack
from jam_midi import Device,ERRORS,frame,parse_reply,checksum,nibble

class PackTools(unittest.TestCase):
    def setUp(self): self.data=pack.factory()
    def test_factory_roundtrip(self):
        b=pack.compile_pack(self.data)
        self.assertEqual(pack.compile_pack(pack.decompile(b)),b)
        self.assertEqual((ROOT/'packs/harris-citypop.jmp').read_bytes(),b)
    def test_invalid_music(self):
        for mutation in [lambda d:d['scales'][0].__setitem__(1,0),lambda d:d['transitions'][0].__setitem__(7,1),lambda d:d['sounds'].append(dict(parameter=5,value=120)),lambda d:d['bass'][0]['notes'].__setitem__(0,-120)]:
            d=copy.deepcopy(self.data);mutation(d)
            with self.assertRaises(ValueError):pack.compile_pack(d)
    def test_transport(self):
        self.assertEqual(frame(1,0),[125,80,96,1,0,127])
        raw=bytes([1,0,15,4,26,30]);body=[1,42,0,*nibble(raw)]
        reply=[125,80,97,*body,checksum(body)]
        self.assertEqual(parse_reply(reply,1,42),raw)
        self.assertIsNone(parse_reply(reply,1,43))
        reply[-1]^=1
        with self.assertRaises(ValueError):parse_reply(reply,1,42)
    def test_older_engine_explains_upgrade(self):
        with patch.object(Device,'request',side_effect=RuntimeError(ERRORS[1])):
            with self.assertRaisesRegex(RuntimeError,'0.3.0 engine upgrade'):
                Device.features(object.__new__(Device))
    def test_midi_selection(self):
        try:import mido
        except ImportError:self.skipTest('mido is optional for compilation')
        from jam_preview import import_midi,inspect_midi
        with tempfile.TemporaryDirectory() as td:
            p=pathlib.Path(td)/'bass.mid';mf=mido.MidiFile();t=mido.MidiTrack();mf.tracks.append(t)
            t.extend([mido.Message('note_on',note=36,velocity=90,time=0),mido.Message('note_off',note=36,time=240),mido.Message('note_on',note=43,velocity=70,time=240),mido.Message('note_off',note=43,time=240)])
            mf.save(p)
            self.assertEqual(inspect_midi(p)['tracks'][0]['notes'],2)
            d=import_midi(p,ROOT/'packs/harris-citypop.json',0,0,1,'bass')
            self.assertEqual(len(d['bass']),5);self.assertEqual(d['bass'][-1]['notes'][0],0)
            pack.compile_pack(d)
    def test_empty_and_wrong_version(self):
        for b in (b'',b'JMP2'+bytes(1200)):
            with self.assertRaises(ValueError):pack.decompile(b)

    def test_songbook_roundtrip_and_validation(self):
        from songbook import factory
        for song,stem in enumerate(('velvet-changes','afterhours','neon-turnaround')):
            d=factory(song);b=pack.compile_pack(d)
            self.assertEqual(pack.compile_pack(pack.decompile(b)),b)
            self.assertEqual((ROOT/'packs'/f'{stem}.jmp').read_bytes(),b)
            for mutate in (lambda x:x['charts'][0][0][0].__setitem__(0,16),
                           lambda x:x['voice_programs'][0]['data'].__setitem__(0,128),
                           lambda x:x['parts'].__setitem__('keys',8),
                           lambda x:x['parts'].__setitem__('keys_gate',0)):
                bad=copy.deepcopy(d);mutate(bad)
                with self.assertRaises(ValueError):pack.compile_pack(bad)

    def test_dx_bank_import(self):
        import subprocess
        from songbook import factory
        voice=factory()['voice_programs'][0]['data'];body=bytes(voice*32)
        bank=bytes([0xf0,0x43,0,9,0x20,0])+body+bytes([(-sum(body))&127,0xf7])
        with tempfile.TemporaryDirectory() as td:
            td=pathlib.Path(td);(td/'bank.syx').write_bytes(bank)
            subprocess.run([sys.executable,str(ROOT/'tools/fm1_jam.py'),'import-voice',str(td/'bank.syx'),
                            str(ROOT/'packs/velvet-changes.json'),str(td/'song.json'),'--voice','3','--role','bass'],check=True,capture_output=True)
            d=pack.decompile(pack.load(td/'song.json'))
            self.assertEqual(d['parts']['bass'],5)
            self.assertEqual(d['voice_programs'][5]['data'],voice)

if __name__=='__main__':unittest.main()
