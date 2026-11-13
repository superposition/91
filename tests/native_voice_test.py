#!/usr/bin/env python3
"""Opt-in voice profile codec compatibility; no hardware access."""
import copy
import hashlib
import pathlib
import sys
import unittest

ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import fm1_jam
import songbook
from chord_chart import from_chords

def chart():
    return from_chords(songbook.factory(),['Dm7','G7','Cmaj7','Cmaj7','Dm7','G7b9','C6','C6'],
                       key='C',name='Native Test')

class NativeVoiceProfiles(unittest.TestCase):
    def test_old_pack_bytes_unchanged(self):
        # Captured before adding voice_flags, including an authored JMP3 pack.
        fixtures=[(fm1_jam.factory(),'5af1401d7226c1483e0a90577087a64e6ced048367132f9bcfcf5d4a3a58550a'),
                  (songbook.factory(),'5ff54336047b67f2f7898f6aef38a1c080adfec78f098db59fbd9b88634e954f'),
                  (chart(),'5b20cd64e0f4ca73a75ef6f47fb977ca6a4a37797d1bad74c6e932eb02ee08a9')]
        for data,want in fixtures:
            blob=fm1_jam.compile_pack(data)
            self.assertEqual(hashlib.sha256(blob).hexdigest(),want)
            self.assertEqual(fm1_jam.compile_pack(fm1_jam.decompile(blob)),blob)

    def test_flags_roundtrip_and_only_one_byte_changes(self):
        data=chart();baseline=fm1_jam.compile_pack(data)
        for flags in range(4):
            data['parts']['voice_flags']=flags
            blob=fm1_jam.compile_pack(data)
            want=bytearray(baseline);want[songbook.H+22]=flags
            self.assertEqual(blob,bytes(want))
            decoded=fm1_jam.decompile(blob)
            self.assertEqual(decoded['parts']['voice_flags'],flags)
            self.assertEqual(fm1_jam.compile_pack(decoded),blob)

    def test_invalid_flags_rejected(self):
        for flags in (-1,4,255,True,1.0,'3',None):
            data=chart();data['parts']['voice_flags']=flags
            with self.assertRaises(ValueError):fm1_jam.compile_pack(data)
        for flags in (4,128,255):
            blob=bytearray(fm1_jam.compile_pack(chart()));blob[songbook.H+22]=flags
            with self.assertRaises(ValueError):fm1_jam.decompile(bytes(blob))

    def test_flag_field_is_v3_only(self):
        data=songbook.factory();baseline=fm1_jam.compile_pack(data)
        data['parts']['voice_flags']=3
        self.assertEqual(fm1_jam.compile_pack(data),baseline)
        self.assertNotIn('voice_flags',fm1_jam.decompile(baseline)['parts'])

if __name__=='__main__':unittest.main()
